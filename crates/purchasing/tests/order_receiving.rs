//! PUR-03 Receiving goods against a purchase order.
// Helpers outside #[test] functions may unwrap too: a failing test should stop right there.
#![allow(clippy::unwrap_used)]

mod common;

use common::{add_supplier, ctx, grn_line, order_for, order_line, qty, receive, test_db};
use pos_core::{Id, Money};
use purchasing::{NewGrn, PurchasingError};
use rusqlite::Transaction;

#[test]
fn pur_03_receiving_form_is_prefilled_with_remaining_quantities() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let order = purchasing::create_order(
        &tx,
        &order_for(
            &supplier,
            vec![order_line("Milk", 10, 5000), order_line("Eggs", 30, 1200)],
        ),
        &ctx("2026-01-10"),
    )
    .unwrap();
    let milk = purchasing::get_order(&tx, &order.id).unwrap().lines[0]
        .id
        .clone();
    receive(&tx, &order.id, &[(milk, 4)]);

    let remaining = purchasing::order_for_receiving(&tx, &order.id).unwrap();

    assert_eq!(remaining.len(), 2);
    assert_eq!(remaining[0].product_name, "Milk");
    assert_eq!(remaining[0].ordered, qty(10));
    assert_eq!(remaining[0].received, qty(4));
    assert_eq!(remaining[0].remaining, qty(6));
    assert_eq!(remaining[0].unit_cost, Money(5000));
    assert_eq!(remaining[1].remaining, qty(30));
}

#[test]
fn pur_03_cancelled_or_received_orders_cannot_be_received_again() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let order = purchasing::create_order(
        &tx,
        &order_for(&supplier, vec![order_line("Milk", 10, 5000)]),
        &ctx("2026-01-10"),
    )
    .unwrap();
    purchasing::cancel_order(&tx, &order.id).unwrap();

    let mut line = grn_line("Milk", 1, 5000);
    line.order_line_id = Some(
        purchasing::get_order(&tx, &order.id).unwrap().lines[0]
            .id
            .clone(),
    );
    let grn = NewGrn {
        supplier_id: supplier,
        order_id: Some(order.id),
        supplier_invoice_no: None,
        invoice_total: Money(5000),
        notes: None,
        lines: vec![line],
    };
    let result = purchasing::record_grn(&tx, &grn, &ctx("2026-01-12"));

    assert!(matches!(
        result,
        Err(PurchasingError::Conflict {
            code: "order_closed"
        })
    ));
}

#[test]
fn pur_03_grn_lines_must_belong_to_the_order() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let make = |tx: &Transaction| {
        purchasing::create_order(
            tx,
            &order_for(&supplier, vec![order_line("Milk", 10, 5000)]),
            &ctx("2026-01-10"),
        )
        .unwrap()
    };
    let first = make(&tx);
    let other = make(&tx);
    let other_line = purchasing::get_order(&tx, &other.id).unwrap().lines[0]
        .id
        .clone();

    let mut line = grn_line("Milk", 1, 5000);
    line.order_line_id = Some(other_line);
    let grn = NewGrn {
        supplier_id: supplier,
        order_id: Some(first.id),
        supplier_invoice_no: None,
        invoice_total: Money(5000),
        notes: None,
        lines: vec![line],
    };

    assert!(matches!(
        purchasing::record_grn(&tx, &grn, &ctx("2026-01-12")),
        Err(PurchasingError::Invalid {
            code: "order_line_not_on_order",
            ..
        })
    ));
}

#[test]
fn pur_03_order_must_be_from_the_same_supplier() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let ordered_from = add_supplier(&tx, "Fresh Farms", 0);
    let someone_else = add_supplier(&tx, "Bakery", 0);
    let order = purchasing::create_order(
        &tx,
        &order_for(&ordered_from, vec![order_line("Milk", 10, 5000)]),
        &ctx("2026-01-10"),
    )
    .unwrap();

    let grn = NewGrn {
        supplier_id: someone_else,
        order_id: Some(order.id),
        supplier_invoice_no: None,
        invoice_total: Money(5000),
        notes: None,
        lines: vec![grn_line("Milk", 1, 5000)],
    };

    assert!(matches!(
        purchasing::record_grn(&tx, &grn, &ctx("2026-01-12")),
        Err(PurchasingError::Invalid {
            code: "order_other_supplier",
            ..
        })
    ));
}

#[test]
fn pur_03_order_line_without_an_order_is_refused() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let mut line = grn_line("Milk", 1, 5000);
    line.order_line_id = Some(Id::new());
    let grn = NewGrn {
        supplier_id: supplier,
        order_id: None,
        supplier_invoice_no: None,
        invoice_total: Money(5000),
        notes: None,
        lines: vec![line],
    };

    assert!(matches!(
        purchasing::record_grn(&tx, &grn, &ctx("2026-01-12")),
        Err(PurchasingError::Invalid {
            code: "order_missing",
            ..
        })
    ));
}
