//! Recording stock changes: the ledger, FEFO batches and negative stock.
//! Requirements INV-01, INV-04 and INV-05.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use inventory::{BatchChoice, InventoryError, MoveKind, Movement, RecordedRow};
use pos_core::{AppError, Id, Money, Qty};
use rusqlite::Connection;
use support::{batch, ctx, pieces, receive, receive_into, test_db};

/// Sells `count` pieces, taking stock first-expiry-first-out.
fn sell_fefo(
    connection: &mut Connection,
    product: &Id,
    count: i64,
    allow_negative: bool,
) -> Result<Vec<RecordedRow>, InventoryError> {
    let transaction = connection.transaction().unwrap();
    let movement = Movement {
        batch: BatchChoice::Fefo,
        ref_: Some(("sale".to_string(), Id::new())),
        ..Movement::new(product, pieces(-count), MoveKind::Sale, ctx())
    };
    let rows = inventory::record(&transaction, movement, allow_negative)?;
    transaction.commit().unwrap();
    Ok(rows)
}

fn ledger_row_count(connection: &Connection) -> i64 {
    connection
        .query_row("SELECT COUNT(*) FROM inventory_ledger", [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn inv_01_selling_3_of_10_leaves_7_and_a_sale_row_pointing_at_the_invoice() {
    let mut connection = test_db();
    let product = Id::new();
    receive(&mut connection, &product, 10);
    let invoice = Id::new();

    let transaction = connection.transaction().unwrap();
    let movement = Movement {
        ref_: Some(("sale".to_string(), invoice.clone())),
        unit_cost: Money(4_500),
        ..Movement::new(&product, pieces(-3), MoveKind::Sale, ctx())
    };
    let rows = inventory::record(&transaction, movement, true).unwrap();
    transaction.commit().unwrap();

    assert_eq!(
        inventory::on_hand(&connection, &product).unwrap(),
        pieces(7)
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].qty_delta, pieces(-3));
    let (kind, delta, ref_type, ref_id, cost, user): (String, i64, String, String, i64, String) =
        connection
            .query_row(
                "SELECT kind, qty_delta_milli, ref_type, ref_id, unit_cost_minor, user_id
                 FROM inventory_ledger WHERE id = ?1",
                [rows[0].id.as_str()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .unwrap();
    assert_eq!(kind, "sale");
    assert_eq!(delta, -3_000);
    assert_eq!(ref_type, "sale");
    assert_eq!(ref_id, invoice.as_str());
    assert_eq!(cost, 4_500);
    assert_eq!(user, support::user().as_str());
}

#[test]
fn inv_01_zero_quantity_is_refused() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    let movement = Movement::new(&Id::new(), Qty(0), MoveKind::Receive, ctx());
    let error = inventory::record(&transaction, movement, true).unwrap_err();
    assert_eq!(AppError::from(error), AppError::validation("qty_zero"));
}

#[test]
fn inv_01_each_kind_only_goes_its_own_way() {
    let mut connection = test_db();
    let product = Id::new();
    let transaction = connection.transaction().unwrap();
    let wrong_way = [
        (MoveKind::Sale, pieces(1)),
        (MoveKind::SupplierReturn, pieces(1)),
        (MoveKind::Receive, pieces(-1)),
        (MoveKind::Return, pieces(-1)),
        (MoveKind::SaleVoid, pieces(-1)),
        (MoveKind::Opening, pieces(-1)),
    ];
    for (kind, qty) in wrong_way {
        let error = inventory::record(
            &transaction,
            Movement::new(&product, qty, kind, ctx()),
            true,
        )
        .unwrap_err();
        assert_eq!(
            AppError::from(error),
            AppError::validation("qty_wrong_sign")
        );
    }
}

#[test]
fn inv_05_fefo_takes_the_earliest_expiry_first_across_three_batches() {
    let mut connection = test_db();
    let product = Id::new();
    // Made in a different order from their expiry dates on purpose.
    let late = batch(&mut connection, &product, Some("2026-09-01"));
    let early = batch(&mut connection, &product, Some("2026-05-01"));
    let middle = batch(&mut connection, &product, Some("2026-07-01"));
    receive_into(&mut connection, &product, &late, 5);
    receive_into(&mut connection, &product, &early, 2);
    receive_into(&mut connection, &product, &middle, 3);

    let rows = sell_fefo(&mut connection, &product, 6, false).unwrap();

    let taken: Vec<(Option<Id>, Qty)> = rows
        .into_iter()
        .map(|row| (row.batch_id, row.qty_delta))
        .collect();
    assert_eq!(
        taken,
        vec![
            (Some(early), pieces(-2)),
            (Some(middle), pieces(-3)),
            (Some(late.clone()), pieces(-1)),
        ]
    );
    assert_eq!(
        inventory::on_hand(&connection, &product).unwrap(),
        pieces(4)
    );
    let batches = inventory::batches_of(&connection, &product).unwrap();
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].id, late);
    assert_eq!(batches[0].on_hand, pieces(4));
}

#[test]
fn inv_05_batches_without_expiry_are_taken_last() {
    let mut connection = test_db();
    let product = Id::new();
    let no_expiry = batch(&mut connection, &product, None);
    let dated = batch(&mut connection, &product, Some("2027-01-01"));
    receive_into(&mut connection, &product, &no_expiry, 2);
    receive_into(&mut connection, &product, &dated, 2);

    let rows = sell_fefo(&mut connection, &product, 1, true).unwrap();

    assert_eq!(rows[0].batch_id, Some(dated));
}

#[test]
fn inv_04_running_past_the_batches_with_negative_stock_allowed_records_the_rest_with_no_batch() {
    let mut connection = test_db();
    let product = Id::new();
    let first = batch(&mut connection, &product, Some("2026-05-01"));
    let second = batch(&mut connection, &product, Some("2026-06-01"));
    let third = batch(&mut connection, &product, Some("2026-07-01"));
    receive_into(&mut connection, &product, &first, 1);
    receive_into(&mut connection, &product, &second, 1);
    receive_into(&mut connection, &product, &third, 1);

    let rows = sell_fefo(&mut connection, &product, 5, true).unwrap();

    let taken: Vec<(Option<Id>, Qty)> = rows
        .into_iter()
        .map(|row| (row.batch_id, row.qty_delta))
        .collect();
    assert_eq!(
        taken,
        vec![
            (Some(first), pieces(-1)),
            (Some(second), pieces(-1)),
            (Some(third), pieces(-1)),
            (None, pieces(-2)),
        ]
    );
    assert_eq!(
        inventory::on_hand(&connection, &product).unwrap(),
        pieces(-2)
    );
    assert!(inventory::verify(&connection).unwrap().is_empty());
}

#[test]
fn inv_04_running_past_the_batches_with_negative_stock_refused_saves_nothing() {
    let mut connection = test_db();
    let product = Id::new();
    let first = batch(&mut connection, &product, Some("2026-05-01"));
    let second = batch(&mut connection, &product, Some("2026-06-01"));
    let third = batch(&mut connection, &product, Some("2026-07-01"));
    receive_into(&mut connection, &product, &first, 1);
    receive_into(&mut connection, &product, &second, 1);
    receive_into(&mut connection, &product, &third, 1);
    let rows_before = ledger_row_count(&connection);

    let error = sell_fefo(&mut connection, &product, 5, false).unwrap_err();

    // The error shows how much is available: 3 pieces.
    let AppError::Conflict { code, params } = AppError::from(error) else {
        panic!("expected a conflict");
    };
    assert_eq!(code, "not_enough_stock");
    assert_eq!(
        params.get("available_milli").map(String::as_str),
        Some("3000")
    );
    assert_eq!(ledger_row_count(&connection), rows_before);
    assert_eq!(
        inventory::on_hand(&connection, &product).unwrap(),
        pieces(3)
    );
}

#[test]
fn inv_04_with_negative_stock_refused_selling_exactly_what_is_there_is_fine() {
    let mut connection = test_db();
    let product = Id::new();
    receive(&mut connection, &product, 3);

    sell_fefo(&mut connection, &product, 3, false).unwrap();

    assert_eq!(inventory::on_hand(&connection, &product).unwrap(), Qty(0));
}

#[test]
fn inv_04_with_negative_stock_refused_a_specific_batch_cannot_go_below_zero() {
    let mut connection = test_db();
    let product = Id::new();
    let small = batch(&mut connection, &product, Some("2026-05-01"));
    let big = batch(&mut connection, &product, Some("2026-06-01"));
    receive_into(&mut connection, &product, &small, 1);
    receive_into(&mut connection, &product, &big, 9);

    let transaction = connection.transaction().unwrap();
    let movement = Movement {
        batch: BatchChoice::Specific(small),
        reason_code: Some("damage".to_string()),
        ..Movement::new(&product, pieces(-2), MoveKind::Adjust, ctx())
    };
    let error = inventory::record(&transaction, movement, false).unwrap_err();
    let AppError::Conflict { code, .. } = AppError::from(error) else {
        panic!("expected a conflict");
    };
    assert_eq!(code, "not_enough_stock_in_batch");
}

#[test]
fn inv_05_a_batch_of_another_product_is_refused() {
    let mut connection = test_db();
    let product = Id::new();
    let other_batch = batch(&mut connection, &Id::new(), Some("2026-05-01"));
    let transaction = connection.transaction().unwrap();
    let movement = Movement {
        batch: BatchChoice::Specific(other_batch),
        ..Movement::new(&product, pieces(1), MoveKind::Receive, ctx())
    };
    let error = inventory::record(&transaction, movement, true).unwrap_err();
    assert_eq!(
        AppError::from(error),
        AppError::validation("batch_of_other_product")
    );
}

#[test]
fn inv_05_a_missing_batch_is_not_found() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    let movement = Movement {
        batch: BatchChoice::Specific(Id::new()),
        ..Movement::new(&Id::new(), pieces(1), MoveKind::Receive, ctx())
    };
    let error = inventory::record(&transaction, movement, true).unwrap_err();
    assert!(matches!(AppError::from(error), AppError::NotFound { .. }));
}

#[test]
fn inv_05_stock_coming_in_for_an_expiry_tracked_product_needs_a_dated_batch() {
    let mut connection = test_db();
    let product = Id::new();
    let undated = batch(&mut connection, &product, None);
    let dated = batch(&mut connection, &product, Some("2026-05-01"));
    let transaction = connection.transaction().unwrap();

    for choice in [BatchChoice::None, BatchChoice::Specific(undated)] {
        let movement = Movement {
            batch: choice,
            tracks_expiry: true,
            ..Movement::new(&product, pieces(1), MoveKind::Receive, ctx())
        };
        let error = inventory::record(&transaction, movement, true).unwrap_err();
        assert_eq!(AppError::from(error), AppError::validation("expiry_needed"));
    }

    let movement = Movement {
        batch: BatchChoice::Specific(dated),
        tracks_expiry: true,
        ..Movement::new(&product, pieces(1), MoveKind::Receive, ctx())
    };
    inventory::record(&transaction, movement, true).unwrap();
}

#[test]
fn inv_05_fefo_is_only_for_stock_going_out() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    let movement = Movement {
        batch: BatchChoice::Fefo,
        ..Movement::new(&Id::new(), pieces(1), MoveKind::Receive, ctx())
    };
    let error = inventory::record(&transaction, movement, true).unwrap_err();
    assert_eq!(AppError::from(error), AppError::validation("batch_needed"));
}

#[test]
fn inv_09_reversing_a_document_with_no_stock_rows_does_nothing() {
    // A sale of only non-stock products or kits has no rows of its own (INV-09).
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    let rows =
        inventory::reverse(&transaction, "sale", &Id::new(), MoveKind::SaleVoid, &ctx()).unwrap();
    assert!(rows.is_empty());
}
