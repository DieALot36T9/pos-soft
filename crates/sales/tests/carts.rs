//! Carts: adding, merging, quantities, holding, resuming, cancelling, crash recovery.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use pos_core::{Bps, Id, Money, Qty, Timestamp};
use sales::{CartStatus, Discount, LineSource, NewCartLine, PriceOverride};
use support::{Till, cart_with, now, scanned, test_db};

fn later(minutes: i64) -> Timestamp {
    Timestamp(now().0 + minutes * 60_000)
}

#[test]
fn chk_01_repeat_scan_merges_into_the_same_line() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let milk = Id::new();
    let cart = cart_with(&mut conn, &till, &[]);

    let tx = conn.transaction().unwrap();
    let first = sales::add_line(&tx, &cart, scanned(&milk, 1_000), true, now()).unwrap();
    let second = sales::add_line(&tx, &cart, scanned(&milk, 1_000), true, later(1)).unwrap();

    assert_eq!(first, second);
    let cart = sales::get_cart(&tx, &cart).unwrap();
    assert_eq!(cart.lines.len(), 1);
    assert_eq!(cart.lines[0].qty, Qty(2_000));
    assert_eq!(cart.updated_at, later(1));
}

#[test]
fn chk_01_without_merge_each_scan_is_a_new_line() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let milk = Id::new();
    let cart = cart_with(&mut conn, &till, &[]);

    let tx = conn.transaction().unwrap();
    sales::add_line(&tx, &cart, scanned(&milk, 1_000), false, now()).unwrap();
    sales::add_line(&tx, &cart, scanned(&milk, 1_000), false, now()).unwrap();

    let cart = sales::get_cart(&tx, &cart).unwrap();
    assert_eq!(cart.lines.len(), 2);
    assert_eq!((cart.lines[0].seq, cart.lines[1].seq), (1, 2));
}

#[test]
fn chk_08_price_label_lines_never_merge() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cheese = Id::new();
    let cart = cart_with(&mut conn, &till, &[]);
    let label = NewCartLine {
        product_id: cheese.clone(),
        qty: Qty(125),
        source: LineSource::ScaleLabel,
        embedded_price: Some(Money(15_000)),
    };

    let tx = conn.transaction().unwrap();
    sales::add_line(&tx, &cart, label.clone(), true, now()).unwrap();
    sales::add_line(&tx, &cart, label, true, now()).unwrap();

    let cart = sales::get_cart(&tx, &cart).unwrap();
    assert_eq!(cart.lines.len(), 2);
    assert_eq!(cart.lines[0].embedded_price, Some(Money(15_000)));
    assert_eq!(cart.lines[0].source, LineSource::ScaleLabel);
}

#[test]
fn chk_12_a_discounted_line_is_not_merged_into() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let soap = Id::new();
    let cart = cart_with(&mut conn, &till, &[(&soap, 1_000)]);

    let tx = conn.transaction().unwrap();
    let line = sales::get_cart(&tx, &cart).unwrap().lines[0].id.clone();
    sales::set_line_discount(&tx, &line, Some(Discount::Percent(Bps(1_000))), now()).unwrap();
    sales::add_line(&tx, &cart, scanned(&soap, 1_000), true, now()).unwrap();

    let cart = sales::get_cart(&tx, &cart).unwrap();
    assert_eq!(cart.lines.len(), 2);
    assert_eq!(
        cart.lines[0].manual_discount,
        Some(Discount::Percent(Bps(1_000)))
    );
    assert_eq!(cart.lines[1].manual_discount, None);
}

#[test]
fn chk_04_quantity_zero_removes_the_line() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(
        &mut conn,
        &till,
        &[(&Id::new(), 1_000), (&Id::new(), 2_000)],
    );

    let tx = conn.transaction().unwrap();
    let lines = sales::get_cart(&tx, &cart).unwrap().lines;
    sales::set_qty(&tx, &lines[1].id, Qty(3_000), now()).unwrap();
    sales::set_qty(&tx, &lines[0].id, Qty(0), now()).unwrap();

    let cart = sales::get_cart(&tx, &cart).unwrap();
    assert_eq!(cart.lines.len(), 1);
    assert_eq!(cart.lines[0].qty, Qty(3_000));
}

#[test]
fn chk_04_negative_quantities_are_refused() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);

    let tx = conn.transaction().unwrap();
    let line = sales::get_cart(&tx, &cart).unwrap().lines[0].id.clone();
    let error = sales::set_qty(&tx, &line, Qty(-1_000), now()).unwrap_err();
    assert_eq!(error.code(), Some("qty_not_positive"));
    let error = sales::add_line(&tx, &cart, scanned(&Id::new(), 0), false, now()).unwrap_err();
    assert_eq!(error.code(), Some("qty_not_positive"));
}

#[test]
fn chk_05_remove_line() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);

    let tx = conn.transaction().unwrap();
    let line = sales::get_cart(&tx, &cart).unwrap().lines[0].id.clone();
    sales::remove_line(&tx, &line, now()).unwrap();
    assert!(sales::get_cart(&tx, &cart).unwrap().lines.is_empty());
    assert!(matches!(
        sales::remove_line(&tx, &line, now()),
        Err(sales::SalesError::NotFound { .. })
    ));
}

#[test]
fn chk_05_cancel_returns_the_cart_and_uses_no_invoice_number() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(
        &mut conn,
        &till,
        &[(&Id::new(), 1_000), (&Id::new(), 2_000)],
    );

    let tx = conn.transaction().unwrap();
    let cancelled = sales::cancel(&tx, &cart).unwrap();
    tx.commit().unwrap();

    assert_eq!(cancelled.lines.len(), 2);
    assert!(
        sales::current_cart(&conn, &till.till, &till.user)
            .unwrap()
            .is_none()
    );
    // The next sale is still number 1.
    let sale = support::sell(&mut conn, &till, &[(&Id::new(), 1_000)]);
    assert_eq!(sale.invoice_no, "T1-000001");
}

#[test]
fn chk_09_hold_keeps_the_cart_with_its_note() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000), (&Id::new(), 500)]);

    let tx = conn.transaction().unwrap();
    sales::hold(&tx, &cart, Some("blue shirt, forgot rice".into()), later(5)).unwrap();

    assert!(
        sales::current_cart(&tx, &till.till, &till.user)
            .unwrap()
            .is_none()
    );
    let held = sales::held_carts(&tx, &till.till).unwrap();
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].cart_id, cart);
    assert_eq!(held[0].note.as_deref(), Some("blue shirt, forgot rice"));
    assert_eq!(held[0].line_count, 2);
    assert_eq!(held[0].held_at, later(5));
    assert_eq!(held[0].user_id, till.user);
}

#[test]
fn chk_09_an_empty_cart_cannot_be_held() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[]);
    let tx = conn.transaction().unwrap();
    assert_eq!(
        sales::hold(&tx, &cart, None, now()).unwrap_err().code(),
        Some("cart_empty")
    );
}

#[test]
fn chk_09_a_held_cart_cannot_be_changed_until_resumed() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);

    let tx = conn.transaction().unwrap();
    let line = sales::get_cart(&tx, &cart).unwrap().lines[0].id.clone();
    sales::hold(&tx, &cart, None, now()).unwrap();

    let error = sales::add_line(&tx, &cart, scanned(&Id::new(), 1_000), false, now()).unwrap_err();
    assert_eq!(error.code(), Some("cart_not_open"));
    assert_eq!(
        sales::set_qty(&tx, &line, Qty(2_000), now())
            .unwrap_err()
            .code(),
        Some("cart_not_open")
    );
    assert_eq!(
        sales::set_customer(&tx, &cart, None, now())
            .unwrap_err()
            .code(),
        Some("cart_not_open")
    );
}

#[test]
fn chk_10_another_cashier_resumes_a_held_cart() {
    let mut conn = test_db();
    let rahim = Till::new("T1");
    let cart = cart_with(&mut conn, &rahim, &[(&Id::new(), 1_000)]);
    let karim = Till {
        user: Id::new(),
        shift: Id::new(),
        ..rahim.clone()
    };

    let tx = conn.transaction().unwrap();
    sales::hold(&tx, &cart, None, now()).unwrap();
    sales::resume(&tx, &cart, &karim.user, &karim.shift, later(10)).unwrap();

    let current = sales::current_cart(&tx, &karim.till, &karim.user)
        .unwrap()
        .unwrap();
    assert_eq!(current.id, cart);
    assert_eq!(current.status, CartStatus::Open);
    assert_eq!(current.shift_id, karim.shift);
    assert!(sales::held_carts(&tx, &karim.till).unwrap().is_empty());
}

#[test]
fn chk_10_resume_throws_away_an_empty_current_cart_but_not_a_full_one() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let held = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    sales::hold(&tx, &held, None, now()).unwrap();
    let empty = sales::open_cart(&tx, &till.till, &till.shift, &till.user, now()).unwrap();

    sales::resume(&tx, &held, &till.user, &till.shift, now()).unwrap();
    assert!(matches!(
        sales::get_cart(&tx, &empty),
        Err(sales::SalesError::NotFound { .. })
    ));

    // Now the resumed cart has a line, so resuming another held cart is refused.
    let other = sales::open_cart(&tx, &till.till, &till.shift, &Id::new(), now()).unwrap();
    sales::add_line(&tx, &other, scanned(&Id::new(), 1_000), false, now()).unwrap();
    sales::hold(&tx, &other, None, now()).unwrap();
    let error = sales::resume(&tx, &other, &till.user, &till.shift, now()).unwrap_err();
    assert_eq!(error.code(), Some("cart_in_progress"));
}

#[test]
fn chk_10_only_a_held_cart_can_be_resumed() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    let error = sales::resume(&tx, &cart, &till.user, &till.shift, now()).unwrap_err();
    assert_eq!(error.code(), Some("cart_not_held"));
}

#[test]
fn chk_19_a_cashier_has_only_one_open_cart_per_till() {
    let mut conn = test_db();
    let till = Till::new("T1");
    cart_with(&mut conn, &till, &[]);
    let tx = conn.transaction().unwrap();
    let error = sales::open_cart(&tx, &till.till, &till.shift, &till.user, now()).unwrap_err();
    assert_eq!(error.code(), Some("cart_already_open"));
}

#[test]
fn chk_11_12_13_17_cart_settings_are_saved() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let customer = Id::new();
    let wholesale = Id::new();
    let manager = Id::new();

    let tx = conn.transaction().unwrap();
    let line = sales::get_cart(&tx, &cart).unwrap().lines[0].id.clone();
    sales::set_customer(&tx, &cart, Some(&customer), now()).unwrap();
    sales::set_price_list(&tx, &cart, Some(&wholesale), now()).unwrap();
    sales::set_invoice_discount(&tx, &cart, Some(Discount::Amount(Money(1_000))), now()).unwrap();
    let price_override = PriceOverride {
        price: Money(8_000),
        by: manager.clone(),
    };
    sales::set_price_override(&tx, &line, Some(price_override.clone()), now()).unwrap();
    sales::set_line_discount(&tx, &line, Some(Discount::Amount(Money(500))), now()).unwrap();

    let saved = sales::get_cart(&tx, &cart).unwrap();
    assert_eq!(saved.customer_id, Some(customer));
    assert_eq!(saved.price_list_id, Some(wholesale));
    assert_eq!(saved.invoice_discount, Some(Discount::Amount(Money(1_000))));
    assert_eq!(saved.lines[0].price_override, Some(price_override));
    assert_eq!(
        saved.lines[0].manual_discount,
        Some(Discount::Amount(Money(500)))
    );

    // Clearing them works too.
    sales::set_customer(&tx, &cart, None, now()).unwrap();
    sales::set_price_override(&tx, &line, None, now()).unwrap();
    sales::set_line_discount(&tx, &line, None, now()).unwrap();
    let saved = sales::get_cart(&tx, &cart).unwrap();
    assert_eq!(saved.customer_id, None);
    assert_eq!(saved.lines[0].price_override, None);
    assert_eq!(saved.lines[0].manual_discount, None);
}

#[test]
fn chk_12_impossible_discounts_are_refused() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    for bad in [
        Discount::Percent(Bps(10_001)),
        Discount::Percent(Bps(0)),
        Discount::Amount(Money(-5)),
    ] {
        let error = sales::set_invoice_discount(&tx, &cart, Some(bad), now()).unwrap_err();
        assert_eq!(error.code(), Some("discount_invalid"));
    }
}

#[test]
fn shf_06_open_carts_on_shift_counts_only_carts_with_lines() {
    let mut conn = test_db();
    let rahim = Till::new("T1");
    let karim = Till {
        user: Id::new(),
        ..rahim.clone()
    };
    cart_with(&mut conn, &rahim, &[(&Id::new(), 1_000)]);
    let karim_cart = cart_with(&mut conn, &karim, &[]);
    assert_eq!(sales::open_carts_on_shift(&conn, &rahim.shift).unwrap(), 1);

    let tx = conn.transaction().unwrap();
    sales::add_line(&tx, &karim_cart, scanned(&Id::new(), 1_000), false, now()).unwrap();
    sales::hold(&tx, &karim_cart, None, now()).unwrap();
    tx.commit().unwrap();
    // A held cart doesn't block closing the shift.
    assert_eq!(sales::open_carts_on_shift(&conn, &rahim.shift).unwrap(), 1);
}
