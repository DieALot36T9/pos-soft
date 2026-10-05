//! PUR-03 Receive goods (GRN) and PUR-04 Supplier returns.
// Helpers outside #[test] functions may unwrap too: a failing test should stop right there.
#![allow(clippy::unwrap_used)]

mod common;

use common::{add_supplier, ctx, date, direct_grn, grn_line, money, qty, test_db};
use pos_core::{Id, Money};
use purchasing::{LedgerKind, NewReturnLine, NewSupplierReturn, PurchasingError};

fn return_line(name: &str, units: i64, unit_cost_minor: i64) -> NewReturnLine {
    NewReturnLine {
        product_id: Id::new(),
        product_name: name.to_string(),
        qty: qty(units),
        unit_cost: money(unit_cost_minor),
        inventory_batch_id: None,
    }
}

#[test]
fn pur_03_grn_without_an_order_gets_a_number_and_totals() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);

    // 20 × 56.00 + 2.5 × 10.00 = 1,145.00
    let mut loose = grn_line("Rice", 0, 1000);
    loose.qty = pos_core::Qty(2500);
    let grn = direct_grn(&supplier, vec![grn_line("Milk", 20, 5600), loose], 114_500);
    let first = purchasing::record_grn(&tx, &grn, &ctx("2026-01-10")).unwrap();
    let second = purchasing::record_grn(&tx, &grn, &ctx("2026-01-10")).unwrap();

    assert_eq!(first.number, "GRN-000001");
    assert_eq!(second.number, "GRN-000002");
    assert_eq!(first.lines_total, Money(114_500));
    assert_eq!(first.invoice_total, Money(114_500));
    assert!(!first.total_differs);
    assert_eq!(first.order_status, None);
}

#[test]
fn pur_03_invoice_total_different_from_lines_is_flagged() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);

    let grn = direct_grn(&supplier, vec![grn_line("Milk", 10, 5000)], 49_000);
    let saved = purchasing::record_grn(&tx, &grn, &ctx("2026-01-10")).unwrap();

    assert_eq!(saved.lines_total, Money(50_000));
    assert_eq!(saved.invoice_total, Money(49_000));
    assert!(saved.total_differs);
}

#[test]
fn pur_03_grn_raises_the_balance_by_the_invoice_total_with_a_due_date() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 30);

    let grn = direct_grn(&supplier, vec![grn_line("Milk", 10, 5000)], 49_000);
    let saved = purchasing::record_grn(&tx, &grn, &ctx("2026-01-10")).unwrap();

    assert_eq!(purchasing::balance(&tx, &supplier).unwrap(), Money(49_000));
    let statement =
        purchasing::statement(&tx, &supplier, date("2026-01-01"), date("2026-01-31")).unwrap();
    let row = &statement.lines[0];
    assert_eq!(row.kind, LedgerKind::Invoice);
    assert_eq!(row.amount, Money(49_000));
    assert_eq!(row.reference.as_deref(), Some("INV-1"));
    assert_eq!(row.ref_type.as_deref(), Some("grn"));
    assert_eq!(row.ref_id.as_ref(), Some(&saved.id));
    assert_eq!(row.due_date, Some(date("2026-02-09")));
}

#[test]
fn pur_03_grn_keeps_batch_expiry_and_inventory_batch() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let batch = Id::new();
    let mut line = grn_line("Yoghurt", 6, 4000);
    line.batch_no = Some("B-77".to_string());
    line.expiry_date = Some(date("2026-02-01"));
    line.inventory_batch_id = Some(batch.clone());

    let saved = purchasing::record_grn(
        &tx,
        &direct_grn(&supplier, vec![line], 24_000),
        &ctx("2026-01-10"),
    )
    .unwrap();

    let (number, batch_no, expiry, batch_id, supplier_id): (
        String,
        String,
        String,
        String,
        String,
    ) = tx
        .query_row(
            "SELECT number, batch_no, expiry_date, inventory_batch_id, supplier_id
             FROM purchasing_v_grn_lines WHERE grn_id = ?1",
            [saved.id.as_str()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(number, "GRN-000001");
    assert_eq!(batch_no, "B-77");
    assert_eq!(expiry, "2026-02-01");
    assert_eq!(batch_id, batch.as_str());
    assert_eq!(supplier_id, supplier.as_str());
}

#[test]
fn pur_03_grn_checks_its_lines() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);

    let empty = direct_grn(&supplier, vec![], 0);
    let negative_cost = direct_grn(&supplier, vec![grn_line("Milk", 1, -5)], 0);
    let negative_invoice = direct_grn(&supplier, vec![grn_line("Milk", 1, 5)], -1);
    let unknown = direct_grn(&Id::new(), vec![grn_line("Milk", 1, 5)], 5);

    let today = ctx("2026-01-10");
    assert!(matches!(
        purchasing::record_grn(&tx, &empty, &today),
        Err(PurchasingError::Invalid {
            code: "lines_missing",
            ..
        })
    ));
    assert!(matches!(
        purchasing::record_grn(&tx, &negative_cost, &today),
        Err(PurchasingError::Invalid {
            code: "cost_negative",
            ..
        })
    ));
    assert!(matches!(
        purchasing::record_grn(&tx, &negative_invoice, &today),
        Err(PurchasingError::Invalid {
            code: "invoice_total_negative",
            ..
        })
    ));
    assert!(matches!(
        purchasing::record_grn(&tx, &unknown, &today),
        Err(PurchasingError::NotFound {
            entity: "supplier",
            ..
        })
    ));
}

#[test]
fn pur_04_supplier_return_lowers_the_balance_by_its_value() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let grn = purchasing::record_grn(
        &tx,
        &direct_grn(&supplier, vec![grn_line("Milk", 10, 5000)], 50_000),
        &ctx("2026-01-10"),
    )
    .unwrap();

    let sent_back = NewSupplierReturn {
        supplier_id: supplier.clone(),
        grn_id: Some(grn.id),
        reason: Some("Damaged".to_string()),
        lines: vec![return_line("Milk", 2, 5000)],
    };
    let first = purchasing::record_return(&tx, &sent_back, &ctx("2026-01-11")).unwrap();
    let second = purchasing::record_return(&tx, &sent_back, &ctx("2026-01-11")).unwrap();

    assert_eq!(first.number, "SR-000001");
    assert_eq!(second.number, "SR-000002");
    assert_eq!(first.total, Money(10_000));
    assert_eq!(purchasing::balance(&tx, &supplier).unwrap(), Money(30_000));
    let statement =
        purchasing::statement(&tx, &supplier, date("2026-01-11"), date("2026-01-11")).unwrap();
    assert_eq!(statement.lines[0].kind, LedgerKind::ReturnCredit);
    assert_eq!(statement.lines[0].amount, Money(-10_000));
    assert_eq!(
        statement.lines[0].ref_type.as_deref(),
        Some("supplier_return")
    );
    assert_eq!(statement.lines[0].ref_id.as_ref(), Some(&first.id));
}

#[test]
fn pur_04_return_against_a_grn_must_be_the_same_supplier() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let delivered_by = add_supplier(&tx, "Fresh Farms", 0);
    let someone_else = add_supplier(&tx, "Bakery", 0);
    let grn = purchasing::record_grn(
        &tx,
        &direct_grn(&delivered_by, vec![grn_line("Milk", 10, 5000)], 50_000),
        &ctx("2026-01-10"),
    )
    .unwrap();

    let wrong = NewSupplierReturn {
        supplier_id: someone_else,
        grn_id: Some(grn.id),
        reason: None,
        lines: vec![return_line("Milk", 1, 5000)],
    };

    assert!(matches!(
        purchasing::record_return(&tx, &wrong, &ctx("2026-01-11")),
        Err(PurchasingError::Invalid {
            code: "grn_other_supplier",
            ..
        })
    ));
}

#[test]
fn pur_04_return_needs_lines_with_a_positive_quantity() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let empty = NewSupplierReturn {
        supplier_id: supplier.clone(),
        grn_id: None,
        reason: None,
        lines: vec![],
    };
    let zero = NewSupplierReturn {
        lines: vec![return_line("Milk", 0, 5000)],
        ..empty.clone()
    };

    assert!(matches!(
        purchasing::record_return(&tx, &empty, &ctx("2026-01-11")),
        Err(PurchasingError::Invalid {
            code: "lines_missing",
            ..
        })
    ));
    assert!(matches!(
        purchasing::record_return(&tx, &zero, &ctx("2026-01-11")),
        Err(PurchasingError::Invalid {
            code: "qty_not_positive",
            ..
        })
    ));
}
