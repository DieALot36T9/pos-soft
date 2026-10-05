//! PUR-02 Purchase orders.
// Helpers outside #[test] functions may unwrap too: a failing test should stop right there.
#![allow(clippy::unwrap_used)]

mod common;

use common::{add_supplier, ctx, order_for, order_line, qty, receive, test_db};
use pos_core::{Id, Money, Qty, Timestamp};
use purchasing::{LowStockItem, OrderStatus, PurchasingError};
use rusqlite::Transaction;

#[test]
fn pur_02_new_order_is_a_draft_with_a_po_number() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);

    let first = purchasing::create_order(
        &tx,
        &order_for(&supplier, vec![order_line("Milk", 10, 5000)]),
        &ctx("2026-01-10"),
    )
    .unwrap();
    let second = purchasing::create_order(
        &tx,
        &order_for(&supplier, vec![order_line("Eggs", 30, 1200)]),
        &ctx("2026-01-10"),
    )
    .unwrap();

    assert_eq!(first.number, "PO-000001");
    assert_eq!(second.number, "PO-000002");
    let saved = purchasing::get_order(&tx, &first.id).unwrap();
    assert_eq!(saved.status, OrderStatus::Draft);
    assert_eq!(saved.lines.len(), 1);
    assert_eq!(saved.lines[0].product_name, "Milk");
    assert_eq!(saved.lines[0].qty, qty(10));
    assert_eq!(saved.lines[0].unit_cost, Money(5000));
    assert_eq!(saved.lines[0].received, Qty(0));
}

#[test]
fn pur_02_po_numbers_are_gapless_after_a_rollback() {
    let mut connection = test_db();
    let supplier = {
        let tx = connection.transaction().unwrap();
        let id = add_supplier(&tx, "Fresh Farms", 0);
        tx.commit().unwrap();
        id
    };
    {
        // The caller gives up: nothing is saved and the number is not used up.
        let tx = connection.transaction().unwrap();
        let order = order_for(&supplier, vec![order_line("Milk", 1, 100)]);
        purchasing::create_order(&tx, &order, &ctx("2026-01-10")).unwrap();
        tx.rollback().unwrap();
    }
    let tx = connection.transaction().unwrap();
    let order = order_for(&supplier, vec![order_line("Milk", 1, 100)]);
    let saved = purchasing::create_order(&tx, &order, &ctx("2026-01-10")).unwrap();

    assert_eq!(saved.number, "PO-000001");
}

#[test]
fn pur_02_order_needs_lines_with_a_positive_quantity() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);

    let empty = purchasing::create_order(&tx, &order_for(&supplier, vec![]), &ctx("2026-01-10"));
    let zero = purchasing::create_order(
        &tx,
        &order_for(&supplier, vec![order_line("Milk", 0, 100)]),
        &ctx("2026-01-10"),
    );

    assert!(matches!(
        empty,
        Err(PurchasingError::Invalid {
            code: "lines_missing",
            ..
        })
    ));
    assert!(matches!(
        zero,
        Err(PurchasingError::Invalid {
            code: "qty_not_positive",
            ..
        })
    ));
}

#[test]
fn pur_02_draft_order_can_be_changed_but_a_sent_one_cannot() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let order = purchasing::create_order(
        &tx,
        &order_for(&supplier, vec![order_line("Milk", 10, 5000)]),
        &ctx("2026-01-10"),
    )
    .unwrap();

    let changed = order_for(
        &supplier,
        vec![order_line("Milk", 12, 5000), order_line("Butter", 4, 9000)],
    );
    purchasing::update_order(&tx, &order.id, &changed).unwrap();
    assert_eq!(
        purchasing::get_order(&tx, &order.id).unwrap().lines.len(),
        2
    );

    purchasing::mark_sent(&tx, &order.id, Timestamp(7000)).unwrap();
    let after_send = purchasing::update_order(&tx, &order.id, &changed);

    assert!(matches!(
        after_send,
        Err(PurchasingError::Conflict {
            code: "order_not_draft"
        })
    ));
}

#[test]
fn pur_02_send_marks_the_order_sent() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let order = purchasing::create_order(
        &tx,
        &order_for(&supplier, vec![order_line("Milk", 10, 5000)]),
        &ctx("2026-01-10"),
    )
    .unwrap();

    purchasing::mark_sent(&tx, &order.id, Timestamp(7000)).unwrap();
    // Printing it again is fine and keeps the first send time.
    purchasing::mark_sent(&tx, &order.id, Timestamp(9000)).unwrap();

    let saved = purchasing::get_order(&tx, &order.id).unwrap();
    assert_eq!(saved.status, OrderStatus::Sent);
    assert_eq!(saved.sent_at, Some(Timestamp(7000)));
}

#[test]
fn pur_02_cancel_works_until_goods_arrive() {
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
    let untouched = make(&tx);
    let started = make(&tx);
    let started_line = purchasing::get_order(&tx, &started.id).unwrap().lines[0]
        .id
        .clone();
    receive(&tx, &started.id, &[(started_line, 4)]);

    purchasing::cancel_order(&tx, &untouched.id).unwrap();
    let refused = purchasing::cancel_order(&tx, &started.id);

    assert_eq!(
        purchasing::get_order(&tx, &untouched.id).unwrap().status,
        OrderStatus::Cancelled
    );
    assert!(matches!(
        refused,
        Err(PurchasingError::Conflict {
            code: "order_has_receipts"
        })
    ));
}

#[test]
fn pur_02_suggest_from_low_stock_uses_this_suppliers_products() {
    let ours = Id::new();
    let theirs = Id::new();
    let item = |name: &str, supplier: Option<Id>, reorder_units: i64| LowStockItem {
        product_id: Id::new(),
        product_name: name.to_string(),
        sku: name.to_string(),
        preferred_supplier_id: supplier,
        reorder_qty: qty(reorder_units),
        unit_cost: Money(2500),
    };
    let low_stock = vec![
        item("Milk", Some(ours.clone()), 24),
        item("Bread", Some(theirs), 10),
        item("Salt", None, 5),
        item("Ghee", Some(ours.clone()), 0),
    ];

    let lines = purchasing::suggest_order_lines(&ours, &low_stock);

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].product_name, "Milk");
    assert_eq!(lines[0].qty, qty(24));
    assert_eq!(lines[0].unit_cost, Money(2500));
}

#[test]
fn pur_02_status_goes_partial_then_received() {
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
    purchasing::mark_sent(&tx, &order.id, Timestamp(1)).unwrap();
    let lines = purchasing::get_order(&tx, &order.id).unwrap().lines;
    let (milk, eggs) = (lines[0].id.clone(), lines[1].id.clone());

    let first = receive(&tx, &order.id, &[(milk.clone(), 10), (eggs.clone(), 10)]);
    assert_eq!(first.order_status, Some(OrderStatus::Partial));
    assert_eq!(
        purchasing::get_order(&tx, &order.id).unwrap().status,
        OrderStatus::Partial
    );

    let second = receive(&tx, &order.id, &[(eggs, 20)]);
    assert_eq!(second.order_status, Some(OrderStatus::Received));
    assert_eq!(
        purchasing::get_order(&tx, &order.id).unwrap().status,
        OrderStatus::Received
    );
    assert!(second.over_received.is_empty());
}

#[test]
fn pur_02_over_receiving_is_allowed_but_flagged() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let supplier = add_supplier(&tx, "Fresh Farms", 0);
    let order = purchasing::create_order(
        &tx,
        &order_for(&supplier, vec![order_line("Milk", 10, 5000)]),
        &ctx("2026-01-10"),
    )
    .unwrap();
    let milk = purchasing::get_order(&tx, &order.id).unwrap().lines[0]
        .id
        .clone();

    let grn = receive(&tx, &order.id, &[(milk.clone(), 12)]);

    assert_eq!(grn.order_status, Some(OrderStatus::Received));
    assert_eq!(grn.over_received, vec![milk]);
    let remaining = purchasing::order_for_receiving(&tx, &order.id).unwrap();
    assert_eq!(remaining[0].received, qty(12));
    assert_eq!(remaining[0].remaining, Qty(0));
}
