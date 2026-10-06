//! The document lists the purchasing screens show (C-M10): orders, deliveries and
//! supplier returns, newest first, and one delivery with its lines.
#![allow(clippy::unwrap_used)]

mod common;

use common::{add_supplier, ctx, direct_grn, grn_line, money, order_for, order_line, qty, test_db};
use purchasing::{NewReturnLine, NewSupplierReturn, OrderStatus};

#[test]
fn pur_02_orders_are_listed_newest_first_with_their_value_and_status() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let rahim = add_supplier(&tx, "Rahim Traders", 30);
    let first = purchasing::create_order(
        &tx,
        &order_for(
            &rahim,
            vec![order_line("Rice", 10, 5_000), order_line("Oil", 3, 12_345)],
        ),
        &ctx("2026-01-10"),
    )
    .unwrap();
    let second = purchasing::create_order(
        &tx,
        &order_for(&rahim, vec![order_line("Salt", 2, 2_000)]),
        &ctx("2026-01-11"),
    )
    .unwrap();
    purchasing::mark_sent(&tx, &second.id, pos_core::Timestamp(5)).unwrap();

    let all = purchasing::list_orders(&tx, None).unwrap();
    let numbers: Vec<&str> = all.iter().map(|row| row.number.as_str()).collect();
    assert_eq!(numbers, vec!["PO-000002", "PO-000001"]);
    assert_eq!(all[1].supplier_name, "Rahim Traders");
    assert_eq!(all[1].line_count, 2);
    assert_eq!(all[1].total, money(50_000 + 37_035));
    let sent = purchasing::list_orders(&tx, Some(OrderStatus::Sent)).unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].id, second.id);
    assert_eq!(sent[0].status, OrderStatus::Sent);
    assert!(
        purchasing::list_orders(&tx, Some(OrderStatus::Received))
            .unwrap()
            .is_empty()
    );
    let _ = first;
}

#[test]
fn pur_03_a_delivery_is_listed_and_read_back_with_its_lines() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let rahim = add_supplier(&tx, "Rahim Traders", 0);
    let karim = add_supplier(&tx, "Karim Foods", 0);
    let mut milk = grn_line("Milk", 4, 5_000);
    milk.batch_no = Some("B-7".into());
    milk.expiry_date = Some(common::date("2026-03-01"));
    let saved = purchasing::record_grn(
        &tx,
        &direct_grn(&rahim, vec![milk, grn_line("Bread", 2, 3_000)], 26_000),
        &ctx("2026-01-12"),
    )
    .unwrap();
    purchasing::record_grn(
        &tx,
        &direct_grn(&karim, vec![grn_line("Egg", 1, 1_000)], 1_000),
        &ctx("2026-01-13"),
    )
    .unwrap();

    assert_eq!(purchasing::list_grns(&tx, None).unwrap().len(), 2);
    let rahims = purchasing::list_grns(&tx, Some(&rahim)).unwrap();
    assert_eq!(rahims.len(), 1);
    assert_eq!(rahims[0].number, saved.number);
    assert_eq!(rahims[0].lines_total, money(26_000));
    assert_eq!(rahims[0].supplier_invoice_no.as_deref(), Some("INV-1"));

    let read = purchasing::get_grn(&tx, &saved.id).unwrap();
    let names: Vec<&str> = read
        .lines
        .iter()
        .map(|line| line.product_name.as_str())
        .collect();
    assert_eq!(names, vec!["Milk", "Bread"]);
    assert_eq!(read.lines[0].qty, qty(4));
    assert_eq!(read.lines[0].batch_no.as_deref(), Some("B-7"));
    assert!(purchasing::get_grn(&tx, &pos_core::Id::new()).is_err());
}

#[test]
fn pur_04_supplier_returns_are_listed_with_their_delivery() {
    let mut connection = test_db();
    let tx = connection.transaction().unwrap();
    let rahim = add_supplier(&tx, "Rahim Traders", 0);
    let grn = purchasing::record_grn(
        &tx,
        &direct_grn(&rahim, vec![grn_line("Milk", 4, 5_000)], 20_000),
        &ctx("2026-01-12"),
    )
    .unwrap();
    let line = purchasing::get_grn(&tx, &grn.id).unwrap().lines.remove(0);
    let sent_back = NewSupplierReturn {
        supplier_id: rahim,
        grn_id: Some(grn.id),
        reason: Some("Damaged".into()),
        lines: vec![NewReturnLine {
            product_id: line.product_id,
            product_name: line.product_name,
            qty: qty(1),
            unit_cost: money(5_000),
            inventory_batch_id: None,
        }],
    };
    purchasing::record_return(&tx, &sent_back, &ctx("2026-01-14")).unwrap();
    let rows = purchasing::list_returns(&tx).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].number, "SR-000001");
    assert_eq!(rows[0].grn_number.as_deref(), Some(grn.number.as_str()));
    assert_eq!(rows[0].total, money(5_000));
    assert_eq!(rows[0].line_count, 1);
}
