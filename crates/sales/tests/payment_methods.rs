//! Payment methods (SET-15, PAY-02).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use pos_core::Id;
use sales::{PaymentKind, PaymentMethodInput, SalesError};
use support::test_db;

fn wallet(name: &str) -> PaymentMethodInput {
    PaymentMethodInput {
        id: None,
        name: name.to_string(),
        kind: PaymentKind::Wallet,
        requires_reference: true,
        active: true,
        sort: 10,
    }
}

#[test]
fn set_15_a_new_database_has_the_system_methods() {
    let conn = test_db();
    let methods = sales::payment_methods(&conn).unwrap();
    let kinds: Vec<PaymentKind> = methods.iter().map(|method| method.kind).collect();
    assert_eq!(
        kinds,
        vec![
            PaymentKind::Cash,
            PaymentKind::Credit,
            PaymentKind::Points,
            PaymentKind::ExchangeCredit
        ]
    );
    assert!(
        methods
            .iter()
            .all(|method| method.is_system && method.active)
    );
    let cash = sales::system_payment_method(&conn, PaymentKind::Cash).unwrap();
    assert_eq!(cash.name, "Cash");
}

#[test]
fn set_15_the_admin_adds_and_changes_a_method() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let bkash = sales::save_payment_method(&tx, wallet("  bKash ")).unwrap();
    let saved = sales::payment_method(&tx, &bkash).unwrap();
    assert_eq!(saved.name, "bKash");
    assert!(saved.requires_reference);
    assert!(!saved.is_system);

    let mut change = wallet("bKash wallet");
    change.id = Some(bkash.clone());
    change.active = false;
    change.sort = 2;
    sales::save_payment_method(&tx, change).unwrap();
    let saved = sales::payment_method(&tx, &bkash).unwrap();
    assert_eq!(saved.name, "bKash wallet");
    assert!(!saved.active);
    // Sorted by `sort`: cash (1), then bKash (2).
    let order: Vec<Id> = sales::payment_methods(&tx)
        .unwrap()
        .into_iter()
        .map(|method| method.id)
        .collect();
    assert_eq!(order[1], bkash);
}

#[test]
fn set_15_a_method_needs_a_name() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let error = sales::save_payment_method(&tx, wallet("   ")).unwrap_err();
    assert_eq!(error.code(), Some("payment_method_name_missing"));
}

#[test]
fn set_15_system_kinds_are_reserved_and_fixed() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let mut extra_credit = wallet("More credit");
    extra_credit.kind = PaymentKind::Credit;
    assert_eq!(
        sales::save_payment_method(&tx, extra_credit)
            .unwrap_err()
            .code(),
        Some("payment_kind_reserved")
    );

    let points = sales::system_payment_method(&tx, PaymentKind::Points).unwrap();
    let rename = PaymentMethodInput {
        id: Some(points.id.clone()),
        name: "Reward points".into(),
        kind: PaymentKind::Points,
        requires_reference: false,
        active: false,
        sort: points.sort,
    };
    sales::save_payment_method(&tx, rename.clone()).unwrap();
    assert_eq!(
        sales::payment_method(&tx, &points.id).unwrap().name,
        "Reward points"
    );

    let change_kind = PaymentMethodInput {
        kind: PaymentKind::Card,
        ..rename
    };
    assert_eq!(
        sales::save_payment_method(&tx, change_kind)
            .unwrap_err()
            .code(),
        Some("payment_kind_fixed")
    );
}

#[test]
fn set_15_cash_stays_on_because_change_is_given_in_cash() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let cash = sales::system_payment_method(&tx, PaymentKind::Cash).unwrap();
    let switch_off = PaymentMethodInput {
        id: Some(cash.id),
        name: cash.name,
        kind: PaymentKind::Cash,
        requires_reference: false,
        active: false,
        sort: cash.sort,
    };
    assert_eq!(
        sales::save_payment_method(&tx, switch_off)
            .unwrap_err()
            .code(),
        Some("payment_cash_required")
    );
}

#[test]
fn set_15_an_unknown_method_is_not_found() {
    let conn = test_db();
    assert!(matches!(
        sales::payment_method(&conn, &Id::new()),
        Err(SalesError::NotFound { .. })
    ));
}

#[test]
fn pay_02_an_inactive_method_cannot_take_a_payment() {
    let mut conn = test_db();
    let till = support::Till::new("T1");
    let cart = support::cart_with(&mut conn, &till, &[(&Id::new(), 1_000)]);
    let tx = conn.transaction().unwrap();
    let mut card = wallet("Card");
    card.kind = PaymentKind::Card;
    card.requires_reference = false;
    card.active = false;
    let card = sales::save_payment_method(&tx, card).unwrap();
    let mut sale = support::sale_of_cart(&tx, &till, &cart);
    sale.payments[0].method_id = card;
    sale.payments[0].tendered = None;
    assert_eq!(
        sales::complete(&tx, sale).unwrap_err().code(),
        Some("payment_method_inactive")
    );
}
