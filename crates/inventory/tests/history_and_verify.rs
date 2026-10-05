//! Stock history with a running balance, and the self-check.
//! Requirements INV-07 and INV-10.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use inventory::{HistoryFilter, MoveKind, Movement, Page};
use pos_core::{BusinessDate, Id, Qty};
use support::{batch, ctx_at, date, pieces, receive, receive_into, test_db};

#[test]
fn inv_07_history_shows_a_running_balance_newest_first() {
    let mut connection = test_db();
    let product = Id::new();
    let transaction = connection.transaction().unwrap();
    let steps = [
        (pieces(10), MoveKind::Receive, 1_000),
        (pieces(-3), MoveKind::Sale, 2_000),
        (pieces(-1), MoveKind::Sale, 3_000),
    ];
    for (qty, kind, at) in steps {
        inventory::record(
            &transaction,
            Movement::new(&product, qty, kind, ctx_at(at)),
            true,
        )
        .unwrap();
    }
    transaction.commit().unwrap();

    let all = HistoryFilter {
        from: None,
        to: None,
        kinds: Vec::new(),
        page: Page::first(50),
    };
    let rows = inventory::history(&connection, &product, &all).unwrap();
    let balances: Vec<Qty> = rows.iter().map(|row| row.balance_after).collect();
    assert_eq!(balances, vec![pieces(6), pieces(7), pieces(10)]);
    assert_eq!(rows[0].entry.kind, MoveKind::Sale);
    assert_eq!(rows[2].entry.qty_delta, pieces(10));

    // Filtering by kind keeps the true balance on each row.
    let receives_only = HistoryFilter {
        kinds: vec![MoveKind::Receive],
        ..all
    };
    let rows = inventory::history(&connection, &product, &receives_only).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].balance_after, pieces(10));
}

#[test]
fn inv_07_history_filters_by_business_date_and_links_to_the_document() {
    let mut connection = test_db();
    let product = Id::new();
    let invoice = Id::new();
    let transaction = connection.transaction().unwrap();
    let mut earlier = ctx_at(1_000);
    earlier.business_date = BusinessDate(date("2026-03-01"));
    inventory::record(
        &transaction,
        Movement::new(&product, pieces(5), MoveKind::Receive, earlier),
        true,
    )
    .unwrap();
    let sale = Movement {
        ref_: Some(("sale".to_string(), invoice.clone())),
        ..Movement::new(&product, pieces(-2), MoveKind::Sale, ctx_at(2_000))
    };
    inventory::record(&transaction, sale, true).unwrap();
    transaction.commit().unwrap();

    let second_day = HistoryFilter {
        from: Some(BusinessDate(date("2026-03-02"))),
        to: Some(BusinessDate(date("2026-03-02"))),
        kinds: Vec::new(),
        page: Page::first(50),
    };
    let rows = inventory::history(&connection, &product, &second_day).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].entry.ref_, Some(("sale".to_string(), invoice)));
    assert_eq!(rows[0].balance_after, pieces(3));
}

#[test]
fn inv_10_verify_finds_nothing_wrong_after_normal_use() {
    let mut connection = test_db();
    let product = Id::new();
    let dated = batch(&mut connection, &product, Some("2026-05-01"));
    receive_into(&mut connection, &product, &dated, 3);
    receive(&mut connection, &product, 2);
    assert!(inventory::verify(&connection).unwrap().is_empty());
}

#[test]
fn inv_10_verify_reports_a_wrong_level_and_a_wrong_batch_without_fixing_them() {
    let mut connection = test_db();
    let product = Id::new();
    let dated = batch(&mut connection, &product, Some("2026-05-01"));
    receive_into(&mut connection, &product, &dated, 3);
    // Damage the caches by hand. Only the ledger is protected by triggers.
    connection
        .execute_batch(
            "UPDATE inventory_levels SET on_hand_milli = 1;
             UPDATE inventory_batches SET on_hand_milli = 2;",
        )
        .unwrap();

    let mismatches = inventory::verify(&connection).unwrap();

    assert_eq!(mismatches.len(), 2);
    let level = mismatches
        .iter()
        .find(|row| row.batch_id.is_none())
        .unwrap();
    assert_eq!(level.product_id, product);
    assert_eq!(level.cached, Qty(1));
    assert_eq!(level.ledger_sum, pieces(3));
    let batch_row = mismatches
        .iter()
        .find(|row| row.batch_id.is_some())
        .unwrap();
    assert_eq!(batch_row.batch_id, Some(dated));
    assert_eq!(batch_row.cached, Qty(2));
    // Nothing was fixed.
    assert_eq!(inventory::on_hand(&connection, &product).unwrap(), Qty(1));
}

#[test]
fn inv_10_verify_notices_a_ledger_row_with_no_level() {
    let mut connection = test_db();
    let product = Id::new();
    receive(&mut connection, &product, 3);
    connection
        .execute_batch("DELETE FROM inventory_levels")
        .unwrap();

    let mismatches = inventory::verify(&connection).unwrap();

    assert_eq!(mismatches.len(), 1);
    assert_eq!(mismatches[0].cached, Qty(0));
    assert_eq!(mismatches[0].ledger_sum, pieces(3));
}
