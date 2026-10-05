//! Completing sales: gapless invoice numbers, the totals check, payments, reading sales back.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use pos_core::{Id, Money, Qty};
use sales::{
    CustomerSnapshot, NewPayment, PaymentKind, PaymentMethodInput, PromotionSnapshot, SalesError,
    TaxPart,
};
use support::{Till, cart_with, now, priced_line, sale_of_cart, sell, test_db, totals_of};

fn assert_refused_as_bad_totals(result: Result<sales::SaleRef, SalesError>) {
    assert!(
        matches!(result, Err(SalesError::TotalsDontAddUp(_))),
        "expected the totals check to refuse, got {result:?}"
    );
}

#[test]
fn pay_08_invoice_numbers_are_gapless_over_1000_sales() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let milk = Id::new();
    for expected in 1..=1_000 {
        let sale = sell(&mut conn, &till, &[(&milk, 1_000)]);
        assert_eq!(sale.invoice_no, format!("T1-{expected:06}"));
    }
}

#[test]
fn pay_07_a_failed_transaction_does_not_use_a_number() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let milk = Id::new();
    sell(&mut conn, &till, &[(&milk, 1_000)]);

    // The sale completes, but a later step of the workflow fails, so it is rolled back.
    let cart = cart_with(&mut conn, &till, &[(&milk, 1_000)]);
    {
        let tx = conn.transaction().unwrap();
        let sale = sale_of_cart(&tx, &till, &cart);
        assert_eq!(sales::complete(&tx, sale).unwrap().invoice_no, "T1-000002");
        // Dropped without commit: rolled back.
    }
    // PAY-07: the cart stays as it was.
    assert_eq!(sales::get_cart(&conn, &cart).unwrap().lines.len(), 1);

    let tx = conn.transaction().unwrap();
    let sale = sale_of_cart(&tx, &till, &cart);
    assert_eq!(sales::complete(&tx, sale).unwrap().invoice_no, "T1-000002");
}

#[test]
fn pay_08_each_till_counts_on_its_own() {
    let mut conn = test_db();
    let first = Till::new("T1");
    let second = Till::new("T2");
    let milk = Id::new();
    sell(&mut conn, &first, &[(&milk, 1_000)]);
    sell(&mut conn, &first, &[(&milk, 1_000)]);
    assert_eq!(
        sell(&mut conn, &second, &[(&milk, 1_000)]).invoice_no,
        "T2-000001"
    );
    assert_eq!(
        sell(&mut conn, &first, &[(&milk, 1_000)]).invoice_no,
        "T1-000003"
    );
}

#[test]
fn pay_07_complete_saves_the_snapshot_and_deletes_the_cart() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let milk = Id::new();
    let cart = cart_with(&mut conn, &till, &[(&milk, 2_000)]);

    let tx = conn.transaction().unwrap();
    let mut sale = sale_of_cart(&tx, &till, &cart);
    sale.customer = Some(CustomerSnapshot {
        id: Id::new(),
        name: "Rahim".into(),
        phone: "+8801711123456".into(),
    });
    sale.lines[0].promotion = Some(PromotionSnapshot {
        id: Id::new(),
        name: "Buy 2 get 1".into(),
    });
    sale.lines[0].tax_components = vec![
        TaxPart {
            name: "CGST".into(),
            bps: pos_core::Bps(750),
            tax: Money(1_174),
        },
        TaxPart {
            name: "SGST".into(),
            bps: pos_core::Bps(750),
            tax: Money(1_174),
        },
    ];
    let sale_ref = sales::complete(&tx, sale.clone()).unwrap();
    tx.commit().unwrap();

    assert!(matches!(
        sales::get_cart(&conn, &cart),
        Err(SalesError::NotFound { .. })
    ));
    let saved = sales::get_sale(&conn, &sale_ref.id).unwrap();
    assert_eq!(saved.invoice_no, "T1-000001");
    assert_eq!(saved.status, sales::SaleStatus::Completed);
    assert_eq!(saved.totals, sale.totals);
    assert_eq!(saved.customer, sale.customer);
    assert_eq!(saved.lines.len(), 1);
    assert_eq!(saved.lines[0].snapshot, sale.lines[0]);
    assert_eq!(saved.lines[0].seq, 1);
    // Cost = 54.00 × 2.
    assert_eq!(saved.lines[0].cost, Money(10_800));
    assert_eq!(saved.payments.len(), 1);
    assert_eq!(saved.payments[0].kind, PaymentKind::Cash);
    assert_eq!(saved.payments[0].amount, Money(18_000));
    assert_eq!(saved.void, None);
}

#[test]
fn pay_07_complete_refuses_header_totals_that_dont_match_the_lines() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(
        &mut conn,
        &till,
        &[(&Id::new(), 1_000), (&Id::new(), 1_000)],
    );
    let tx = conn.transaction().unwrap();
    let good = sale_of_cart(&tx, &till, &cart);

    let mut sale = good.clone();
    sale.totals.tax.0 += 1;
    assert_refused_as_bad_totals(sales::complete(&tx, sale));

    let mut sale = good.clone();
    sale.totals.cost.0 -= 1;
    assert_refused_as_bad_totals(sales::complete(&tx, sale));

    let mut sale = good.clone();
    sale.totals.rounding = Money(-50);
    assert_refused_as_bad_totals(sales::complete(&tx, sale));

    // Nothing was saved and no number was used.
    assert!(!sales::has_any_sale(&tx).unwrap());
    assert_eq!(sales::complete(&tx, good).unwrap().invoice_no, "T1-000001");
}

#[test]
fn pay_07_complete_refuses_a_line_that_doesnt_add_up() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    let good = sale_of_cart(&tx, &till, &cart);

    // net + tax must be gross.
    let mut sale = good.clone();
    sale.lines[0].net.0 += 1;
    sale.totals.net.0 += 1;
    assert_refused_as_bad_totals(sales::complete(&tx, sale));

    // base − discounts must be the taxed amount.
    let mut sale = good.clone();
    sale.lines[0].promo_discount = Money(100);
    sale.totals.promo_discount = Money(100);
    assert_refused_as_bad_totals(sales::complete(&tx, sale));

    // Tax parts must add up to the tax.
    let mut sale = good.clone();
    sale.lines[0].tax_components = vec![TaxPart {
        name: "CGST".into(),
        bps: pos_core::Bps(750),
        tax: Money(1),
    }];
    assert_refused_as_bad_totals(sales::complete(&tx, sale));

    // CHK-12: a discount can't make a line negative.
    let mut sale = good.clone();
    let line = &mut sale.lines[0];
    line.manual_discount = Money(line.base.0 + 100);
    line.gross = Money(-100);
    line.tax = Money(0);
    line.net = Money(-100);
    sale.totals = totals_of(&sale.lines);
    sale.payments.clear();
    assert_refused_as_bad_totals(sales::complete(&tx, sale));
}

#[test]
fn pay_07_complete_refuses_lines_that_dont_match_the_cart() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    let mut sale = sale_of_cart(&tx, &till, &cart);
    sale.lines[0] = priced_line(&sale.lines[0].product_id, Qty(2_000), Money(4_500));
    sale.totals = totals_of(&sale.lines);
    assert_refused_as_bad_totals(sales::complete(&tx, sale));
}

#[test]
fn pay_07_complete_refuses_a_held_or_missing_cart() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    let sale = sale_of_cart(&tx, &till, &cart);
    sales::hold(&tx, &cart, None, now()).unwrap();
    assert_eq!(
        sales::complete(&tx, sale.clone()).unwrap_err().code(),
        Some("cart_not_open")
    );

    let mut missing = sale;
    missing.cart_id = Id::new();
    assert!(matches!(
        sales::complete(&tx, missing),
        Err(SalesError::NotFound { .. })
    ));
}

#[test]
fn pay_03_complete_refuses_a_short_payment() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    let mut sale = sale_of_cart(&tx, &till, &cart);
    sale.payments[0].amount.0 -= 100;
    sale.payments[0].tendered = Some(sale.payments[0].amount);
    let error = sales::complete(&tx, sale).unwrap_err();
    assert_eq!(error.code(), Some("payment_not_enough"));
}

#[test]
fn pay_01_complete_stores_cash_tendered_and_change() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    let mut sale = sale_of_cart(&tx, &till, &cart);
    // Due 90.00, cash 100.00 → change 10.00.
    sale.payments[0].tendered = Some(Money(10_000));
    sale.totals.change = Money(1_000);
    let sale_ref = sales::complete(&tx, sale.clone()).unwrap();
    let saved = sales::get_sale(&tx, &sale_ref.id).unwrap();
    assert_eq!(saved.totals.change, Money(1_000));
    assert_eq!(saved.payments[0].tendered, Some(Money(10_000)));
}

#[test]
fn pay_01_change_that_doesnt_match_the_cash_is_refused() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    let mut sale = sale_of_cart(&tx, &till, &cart);
    sale.payments[0].tendered = Some(Money(10_000));
    sale.totals.change = Money(500);
    assert_refused_as_bad_totals(sales::complete(&tx, sale));
}

#[test]
fn pay_02_a_method_that_needs_a_reference_refuses_a_payment_without_one() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    let bkash = sales::save_payment_method(
        &tx,
        PaymentMethodInput {
            id: None,
            name: "bKash".into(),
            kind: PaymentKind::Wallet,
            requires_reference: true,
            active: true,
            sort: 5,
        },
    )
    .unwrap();
    let mut sale = sale_of_cart(&tx, &till, &cart);
    sale.payments = vec![NewPayment {
        method_id: bkash,
        amount: sale.totals.total,
        tendered: None,
        reference: Some("  ".into()),
    }];
    let error = sales::complete(&tx, sale.clone()).unwrap_err();
    assert_eq!(error.code(), Some("payment_reference_missing"));

    sale.payments[0].reference = Some("TX-8891".into());
    let sale_ref = sales::complete(&tx, sale).unwrap();
    let saved = sales::get_sale(&tx, &sale_ref.id).unwrap();
    assert_eq!(saved.payments[0].reference.as_deref(), Some("TX-8891"));
    assert_eq!(saved.payments[0].kind, PaymentKind::Wallet);
}

#[test]
fn pay_03_non_cash_payments_have_no_tendered_amount() {
    let mut conn = test_db();
    let till = Till::new("T1");
    let cart = cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    let credit = sales::system_payment_method(&tx, PaymentKind::Credit).unwrap();
    let mut sale = sale_of_cart(&tx, &till, &cart);
    sale.payments = vec![NewPayment {
        method_id: credit.id,
        amount: sale.totals.total,
        tendered: Some(Money(sale.totals.total.0 + 1_000)),
        reference: None,
    }];
    sale.totals.change = Money(1_000);
    assert_refused_as_bad_totals(sales::complete(&tx, sale));
}
