//! Taking payment and completing a sale (Req - Payments): change, other methods with
//! references, split payments, rounding, the one-transaction completion and gapless
//! invoice numbers.

mod common;

use app::commands::checkout as till;
use app::commands::payment_methods as methods;
use app::commands::settings as settings_commands;
use app::state::AppState;
use app::workflows::checkout::MethodKind;
use app::workflows::complete_sale::PaymentEntry;
use app::workflows::payment_methods::PaymentMethodSave;
use app::workflows::settings_change::SettingsChange;
use common::*;
use pos_core::{AppError, Id, Money, Qty};
use settings::{InventorySettings, Setting};

fn shop_selling(price_minor: i64) -> (tempfile::TempDir, AppState, Id) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let item = till_product(
        &state,
        "Item",
        "ITEM-1",
        price_minor,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    stock_in(&state, &item, Qty(1_000_000));
    open_shift(&state, 100_000);
    (folder, state, item)
}

fn method(state: &AppState, name: &str, kind: MethodKind, requires_reference: bool) -> Id {
    let input = PaymentMethodSave {
        id: None,
        name: name.into(),
        kind,
        requires_reference,
        active: true,
        sort: 10,
    };
    methods::payment_method_save(state, &input).expect("method saved")
}

fn paying(method_id: &Id, amount_minor: i64, reference: Option<&str>) -> PaymentEntry {
    PaymentEntry {
        method_id: method_id.clone(),
        amount: Money(amount_minor),
        reference: reference.map(str::to_string),
    }
}

fn code_of<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
    match result {
        Err(AppError::Validation { code, .. }) | Err(AppError::Conflict { code, .. }) => code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn sale_count(state: &AppState) -> u32 {
    state
        .db
        .read(|conn| Ok(sales::sales_summary(conn)?))
        .expect("summary")
        .sale_count
}

#[test]
fn pay_01_cash_gives_change_and_quick_notes() {
    let (_folder, state, _item) = shop_selling(47_000);
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    let preview = till::checkout_payment_preview(&state, &[]).expect("preview");
    assert_eq!(
        (preview.due, preview.cash_to_pay),
        (Money(47_000), Money(47_000))
    );
    assert_eq!(
        preview.quick_cash,
        vec![Money(50_000), Money(60_000), Money(100_000)]
    );
    let preview = till::checkout_payment_preview(&state, &[cash(&state, 50_000)]).expect("preview");
    assert_eq!(
        (preview.remaining, preview.change),
        (Money(0), Money(3_000))
    );
    let done = till::checkout_complete(&state, devices(), &[cash(&state, 50_000)]).expect("sold");
    assert_eq!(done.invoice_no, "T1-000001");
    assert_eq!((done.total, done.change), (Money(47_000), Money(3_000)));
    assert!(done.cash_taken);
    assert!(
        done.cart.lines.is_empty(),
        "a fresh cart for the next customer"
    );
}

#[test]
fn pay_02_a_method_can_require_a_reference() {
    let (_folder, state, _item) = shop_selling(10_000);
    let card = method(&state, "Card", MethodKind::Card, true);
    let view = till::checkout_state(&state).expect("state");
    assert!(
        view.payment_methods
            .iter()
            .any(|each| each.id == card && each.requires_reference)
    );
    assert!(
        view.payment_methods
            .iter()
            .all(|each| each.kind != MethodKind::Credit)
    );
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    assert_eq!(
        code_of(till::checkout_complete(
            &state,
            devices(),
            &[paying(&card, 10_000, None)]
        )),
        "payment_reference_missing"
    );
    let done = till::checkout_complete(&state, devices(), &[paying(&card, 10_000, Some("TX-77"))])
        .expect("sold");
    assert!(!done.cash_taken);
    let sale = state
        .db
        .read(|conn| Ok(sales::get_sale(conn, &done.sale_id)?))
        .expect("sale");
    assert_eq!(sale.payments[0].reference.as_deref(), Some("TX-77"));
}

#[test]
fn pay_03_split_payments_and_only_cash_goes_over() {
    let (_folder, state, _item) = shop_selling(100_000);
    let wallet = method(&state, "bKash", MethodKind::Wallet, false);
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    assert_eq!(
        code_of(till::checkout_payment_preview(
            &state,
            &[paying(&wallet, 110_000, None)]
        )),
        "payment_over_remaining"
    );
    let short =
        till::checkout_payment_preview(&state, &[paying(&wallet, 40_000, None)]).expect("short");
    assert_eq!(short.remaining, Money(60_000));
    assert_eq!(
        code_of(till::checkout_complete(
            &state,
            devices(),
            &[paying(&wallet, 40_000, None)]
        )),
        "payment_not_enough"
    );
    let done = till::checkout_complete(
        &state,
        devices(),
        &[paying(&wallet, 40_000, None), cash(&state, 70_000)],
    )
    .expect("sold");
    assert_eq!(done.change, Money(10_000));
    let expected = state
        .db
        .read(|conn| Ok(shifts::expected_cash(conn, &current_shift(conn))?))
        .expect("expected cash");
    // Float 1,000.00 plus the 600.00 kept in cash; the wallet part isn't in the drawer.
    assert_eq!(expected, Money(160_000));
}

#[test]
fn pay_04_pay_05_credit_and_points_need_a_customer() {
    let (_folder, state, _item) = shop_selling(10_000);
    let credit = state
        .db
        .read(|conn| {
            Ok(sales::system_payment_method(
                conn,
                sales::PaymentKind::Credit,
            )?)
        })
        .expect("credit")
        .id;
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    assert_eq!(
        code_of(till::checkout_complete(
            &state,
            devices(),
            &[paying(&credit, 10_000, None)]
        )),
        "payment_needs_customer"
    );
}

#[test]
fn pay_06_cash_only_rounding() {
    let (_folder, state, _item) = shop_selling(137_750);
    let card = method(&state, "Card", MethodKind::Card, false);
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    let cart = till::checkout_state(&state)
        .expect("state")
        .cart
        .expect("cart");
    assert_eq!(
        (cart.totals.rounding, cart.totals.total),
        (Money(50), Money(137_800))
    );
    let by_card =
        till::checkout_payment_preview(&state, &[paying(&card, 137_750, None)]).expect("card");
    assert_eq!((by_card.rounding, by_card.remaining), (Money(0), Money(0)));
    let done = till::checkout_complete(&state, devices(), &[cash(&state, 140_000)]).expect("cash");
    assert_eq!((done.total, done.change), (Money(137_800), Money(2_200)));
    let sale = state
        .db
        .read(|conn| Ok(sales::get_sale(conn, &done.sale_id)?))
        .expect("sale");
    assert_eq!(
        (sale.totals.gross, sale.totals.rounding),
        (Money(137_750), Money(50))
    );

    till::checkout_scan(&state, "ITEM-1").expect("scan");
    let done =
        till::checkout_complete(&state, devices(), &[paying(&card, 137_750, None)]).expect("card");
    assert_eq!(done.total, Money(137_750));
}

#[test]
fn pay_07_completion_saves_everything_or_nothing() {
    let (_folder, state, item) = shop_selling(10_000);
    till::checkout_scan(&state, "3*ITEM-1").expect("scan");
    let done = till::checkout_complete(&state, devices(), &[cash(&state, 30_000)]).expect("sold");
    let on_hand = |state: &AppState| {
        state
            .db
            .read(|conn| Ok(inventory::on_hand(conn, &item)?))
            .expect("on hand")
    };
    assert_eq!(on_hand(&state), Qty(997_000));
    let sale = state
        .db
        .read(|conn| Ok(sales::get_sale(conn, &done.sale_id)?))
        .expect("sale");
    assert_eq!(sale.lines[0].snapshot.name, "Item");
    assert_eq!(sale.lines[0].snapshot.unit_cost, Money(6_000));

    // A step fails inside the transaction: stock runs out after the item was scanned.
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    let mut rules = InventorySettings::default();
    rules.allow_negative = false;
    settings_commands::settings_save(&state, &SettingsChange::Inventory(rules)).expect("saved");
    let mut out = app::workflows::stock::AdjustInput {
        product_id: item.clone(),
        qty_delta: Qty(-997_000),
        reason_code: "damage".into(),
        note: None,
        batch: app::workflows::stock::AdjustBatch::Auto,
    };
    app::commands::inventory::stock_adjust(&state, &out).expect("all gone");
    let before = till::checkout_state(&state)
        .expect("state")
        .cart
        .expect("cart");
    assert_eq!(
        code_of(till::checkout_complete(
            &state,
            devices(),
            &[cash(&state, 10_000)]
        )),
        "not_enough_stock"
    );
    assert_eq!(sale_count(&state), 1, "nothing saved");
    assert_eq!(
        till::checkout_state(&state)
            .expect("state")
            .cart
            .expect("cart"),
        before
    );

    out.qty_delta = Qty(5_000);
    out.reason_code = "opening".into();
    app::commands::inventory::stock_adjust(&state, &out).expect("back in");
    let done = till::checkout_complete(&state, devices(), &[cash(&state, 10_000)]).expect("sold");
    assert_eq!(
        done.invoice_no, "T1-000002",
        "the failed try used no number"
    );
}

#[test]
fn pay_08_invoice_numbers_are_gapless_on_the_till() {
    let (_folder, state, _item) = shop_selling(1_000);
    for number in 1..=25 {
        till::checkout_scan(&state, "ITEM-1").expect("scan");
        let done =
            till::checkout_complete(&state, devices(), &[cash(&state, 1_000)]).expect("sold");
        assert_eq!(done.invoice_no, format!("T1-{number:06}"));
    }
}

#[test]
fn chk_05_an_empty_cart_cannot_be_completed() {
    let (_folder, state, _item) = shop_selling(1_000);
    assert_eq!(
        code_of(till::checkout_complete(&state, devices(), &[])),
        "cart_empty"
    );
}

#[test]
fn set_15_payment_methods_are_listed_saved_and_audited() {
    let (_folder, state, _item) = shop_selling(1_000);
    let wallet = method(&state, "Nagad", MethodKind::Wallet, true);
    let list = methods::payment_method_list(&state).expect("list");
    assert!(list.iter().any(|each| each.id == wallet && !each.is_system));
    assert!(
        list.iter()
            .any(|each| each.kind == MethodKind::Cash && each.is_system)
    );
    assert_eq!(audit_count(&state, "payment_method.change"), 1);
    let reserved = PaymentMethodSave {
        id: None,
        name: "Points".into(),
        kind: MethodKind::Points,
        requires_reference: false,
        active: true,
        sort: 0,
    };
    assert_eq!(
        code_of(methods::payment_method_save(&state, &reserved)),
        "payment_kind_reserved"
    );
}

fn current_shift(conn: &rusqlite::Connection) -> Id {
    let till: settings::TillSettings = settings::get(conn).expect("till");
    shifts::current(conn, &till.till_id.expect("till id"))
        .expect("shift")
        .expect("open")
        .id
}
