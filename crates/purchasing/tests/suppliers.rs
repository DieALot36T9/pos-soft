//! PUR-01 Suppliers.
// Helpers outside #[test] functions may unwrap too: a failing test should stop right there.
#![allow(clippy::unwrap_used)]

mod common;

use common::{add_supplier, ctx, order_for, order_line, supplier_named, test_db};
use pos_core::{Id, Timestamp};
use purchasing::{PurchasingError, SupplierInput};

#[test]
fn pur_01_create_supplier_keeps_every_field() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let input = SupplierInput {
        name: "Fresh Farms".to_string(),
        contact_person: Some("Rahim".to_string()),
        phone: Some("+8801711123456".to_string()),
        email: Some("orders@freshfarms.test".to_string()),
        address: Some("Dhaka".to_string()),
        tax_number: Some("BIN-123".to_string()),
        payment_terms_days: 30,
        notes: Some("Delivers on Mondays".to_string()),
    };

    let id = purchasing::create_supplier(&tx, &input, Timestamp(1000)).unwrap();
    let saved = purchasing::get_supplier(&tx, &id).unwrap();

    assert_eq!(saved.id, id);
    assert_eq!(saved.name, "Fresh Farms");
    assert_eq!(saved.contact_person.as_deref(), Some("Rahim"));
    assert_eq!(saved.phone.as_deref(), Some("+8801711123456"));
    assert_eq!(saved.email.as_deref(), Some("orders@freshfarms.test"));
    assert_eq!(saved.address.as_deref(), Some("Dhaka"));
    assert_eq!(saved.tax_number.as_deref(), Some("BIN-123"));
    assert_eq!(saved.payment_terms_days, 30);
    assert_eq!(saved.notes.as_deref(), Some("Delivers on Mondays"));
    assert_eq!(saved.archived_at, None);
    assert_eq!(saved.created_at, Timestamp(1000));
}

#[test]
fn pur_01_supplier_name_is_required() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();

    let result = purchasing::create_supplier(&tx, &supplier_named("   ", 0), Timestamp(1));

    assert!(matches!(
        result,
        Err(PurchasingError::Invalid {
            field: "name",
            code: "supplier_name_missing"
        })
    ));
}

#[test]
fn pur_01_update_supplier_changes_its_fields() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let id = add_supplier(&tx, "Fresh Farms", 0);

    let changes = SupplierInput {
        phone: Some("+8801811000000".to_string()),
        ..supplier_named("Fresh Farms Ltd", 15)
    };
    purchasing::update_supplier(&tx, &id, &changes, Timestamp(2000)).unwrap();

    let saved = purchasing::get_supplier(&tx, &id).unwrap();
    assert_eq!(saved.name, "Fresh Farms Ltd");
    assert_eq!(saved.payment_terms_days, 15);
    assert_eq!(saved.phone.as_deref(), Some("+8801811000000"));
    assert_eq!(saved.updated_at, Timestamp(2000));
}

#[test]
fn pur_01_update_of_unknown_supplier_is_not_found() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();

    let result =
        purchasing::update_supplier(&tx, &Id::new(), &supplier_named("X", 0), Timestamp(1));

    assert!(matches!(
        result,
        Err(PurchasingError::NotFound {
            entity: "supplier",
            ..
        })
    ));
}

#[test]
fn pur_01_archive_hides_the_supplier_but_keeps_it() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let kept = add_supplier(&tx, "Bakery", 0);
    let archived = add_supplier(&tx, "Old Dairy", 0);

    purchasing::archive_supplier(&tx, &archived, Timestamp(5000)).unwrap();

    let active = purchasing::list_suppliers(&tx, false).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, kept);
    let everyone = purchasing::list_suppliers(&tx, true).unwrap();
    assert_eq!(everyone.len(), 2);
    let old = purchasing::get_supplier(&tx, &archived).unwrap();
    assert_eq!(old.archived_at, Some(Timestamp(5000)));
}

#[test]
fn pur_01_suppliers_are_listed_by_name() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    add_supplier(&tx, "Zed Traders", 0);
    add_supplier(&tx, "Acme", 0);

    let names: Vec<String> = purchasing::list_suppliers(&tx, false)
        .unwrap()
        .into_iter()
        .map(|supplier| supplier.name)
        .collect();

    assert_eq!(names, vec!["Acme".to_string(), "Zed Traders".to_string()]);
}

#[test]
fn pur_01_archived_supplier_gets_no_new_orders() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Old Dairy", 0);
    purchasing::archive_supplier(&tx, &supplier, Timestamp(5)).unwrap();

    let order = order_for(&supplier, vec![order_line("Milk", 10, 5000)]);
    let result = purchasing::create_order(&tx, &order, &ctx("2026-01-10"));

    assert!(matches!(
        result,
        Err(PurchasingError::Conflict {
            code: "supplier_archived"
        })
    ));
}
