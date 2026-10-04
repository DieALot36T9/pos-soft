//! Stock counts: create, count, review differences, post or cancel. Requirement INV-06.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use std::collections::HashMap;

use inventory::{CountEntry, CountScope, CountStatus, MoveKind, Movement, NewCount};
use pos_core::{AppError, Id, Money, Qty, Timestamp};
use rusqlite::Connection;
use support::{batch, ctx, pieces, receive, receive_into, test_db, user};

fn new_count(connection: &mut Connection, scope: CountScope) -> Id {
    let transaction = connection.transaction().unwrap();
    let count = NewCount {
        name: "March count".to_string(),
        scope,
        scope_details: serde_json::json!({ "note": "Aisle 3" }),
    };
    let created = inventory::create_count(&transaction, &count, Timestamp(2_000), &user()).unwrap();
    transaction.commit().unwrap();
    created.id
}

fn count_line(
    connection: &mut Connection,
    count: &Id,
    product: &Id,
    batch_id: Option<&Id>,
    qty: Qty,
) {
    let transaction = connection.transaction().unwrap();
    let entry = CountEntry {
        product_id: product.clone(),
        batch_id: batch_id.cloned(),
        qty,
    };
    inventory::record_count_line(&transaction, count, &entry, Timestamp(3_000), &user()).unwrap();
    transaction.commit().unwrap();
}

fn sell(connection: &mut Connection, product: &Id, count: i64) {
    let transaction = connection.transaction().unwrap();
    let movement = Movement::new(product, pieces(-count), MoveKind::Sale, ctx());
    inventory::record(&transaction, movement, true).unwrap();
    transaction.commit().unwrap();
}

fn post(connection: &mut Connection, count: &Id, costs: &HashMap<Id, Money>) {
    let transaction = connection.transaction().unwrap();
    inventory::post_count(&transaction, count, costs, &ctx()).unwrap();
    transaction.commit().unwrap();
}

fn count_row_total(connection: &Connection, count: &Id) -> (i64, i64) {
    connection
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(qty_delta_milli), 0) FROM inventory_ledger
             WHERE kind = 'count' AND ref_type = 'count' AND ref_id = ?1",
            [count.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
}

#[test]
fn inv_06_counts_get_gapless_numbers() {
    let mut connection = test_db();
    let first = new_count(&mut connection, CountScope::Partial);
    let second = new_count(&mut connection, CountScope::Full);
    let one = inventory::get_count(&connection, &first).unwrap();
    let two = inventory::get_count(&connection, &second).unwrap();
    assert_eq!(one.number, "SC-000001");
    assert_eq!(two.number, "SC-000002");
    assert_eq!(one.status, CountStatus::Open);
    assert_eq!(one.scope_details, serde_json::json!({ "note": "Aisle 3" }));
    let listed = inventory::list_counts(&connection, inventory::Page::first(10)).unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].number, "SC-000002");
}

#[test]
fn inv_06_expected_is_captured_at_the_first_count_and_scans_add_up() {
    let mut connection = test_db();
    let product = Id::new();
    receive(&mut connection, &product, 10);
    let count = new_count(&mut connection, CountScope::Partial);

    count_line(&mut connection, &count, &product, None, pieces(4));
    // A sale between two scans of the same product.
    sell(&mut connection, &product, 3);
    count_line(&mut connection, &count, &product, None, pieces(2));

    let variances = inventory::count_variances(&connection, &count, &HashMap::new()).unwrap();
    assert_eq!(variances.len(), 1);
    assert_eq!(variances[0].expected, pieces(10));
    assert_eq!(variances[0].counted, pieces(6));
    assert_eq!(variances[0].difference, pieces(-4));
    assert!(variances[0].was_counted);
}

#[test]
fn inv_06_sales_after_counting_do_not_create_a_difference() {
    let mut connection = test_db();
    let product = Id::new();
    receive(&mut connection, &product, 10);
    let count = new_count(&mut connection, CountScope::Partial);
    count_line(&mut connection, &count, &product, None, pieces(10));

    sell(&mut connection, &product, 4);
    post(&mut connection, &count, &HashMap::new());

    assert_eq!(count_row_total(&connection, &count), (0, 0));
    assert_eq!(
        inventory::on_hand(&connection, &product).unwrap(),
        pieces(6)
    );
}

#[test]
fn inv_06_posting_creates_count_rows_only_for_differences() {
    let mut connection = test_db();
    let exact = Id::new();
    let short = Id::new();
    let extra = Id::new();
    receive(&mut connection, &exact, 5);
    receive(&mut connection, &short, 5);
    receive(&mut connection, &extra, 5);
    let count = new_count(&mut connection, CountScope::Partial);
    count_line(&mut connection, &count, &exact, None, pieces(5));
    count_line(&mut connection, &count, &short, None, pieces(3));
    count_line(&mut connection, &count, &extra, None, pieces(8));

    let costs = HashMap::from([(short.clone(), Money(2_000)), (extra.clone(), Money(1_000))]);
    let variances = inventory::count_variances(&connection, &count, &costs).unwrap();
    let short_row = variances
        .iter()
        .find(|row| row.product_id == short)
        .unwrap();
    // Two pieces missing at 20.00 each.
    assert_eq!(short_row.cost_value, Money(-4_000));

    post(&mut connection, &count, &costs);

    assert_eq!(count_row_total(&connection, &count), (2, 1_000));
    assert_eq!(inventory::on_hand(&connection, &exact).unwrap(), pieces(5));
    assert_eq!(inventory::on_hand(&connection, &short).unwrap(), pieces(3));
    assert_eq!(inventory::on_hand(&connection, &extra).unwrap(), pieces(8));
    let posted = inventory::get_count(&connection, &count).unwrap();
    assert_eq!(posted.status, CountStatus::Posted);
    assert_eq!(posted.posted_by, Some(user()));
    assert!(inventory::verify(&connection).unwrap().is_empty());
}

#[test]
fn inv_06_a_full_count_zeroes_products_that_were_never_counted() {
    let mut connection = test_db();
    let counted = Id::new();
    let forgotten = Id::new();
    let forgotten_with_batches = Id::new();
    receive(&mut connection, &counted, 5);
    receive(&mut connection, &forgotten, 4);
    let dated = batch(&mut connection, &forgotten_with_batches, Some("2026-05-01"));
    receive_into(&mut connection, &forgotten_with_batches, &dated, 3);
    receive(&mut connection, &forgotten_with_batches, 1);
    let count = new_count(&mut connection, CountScope::Full);
    count_line(&mut connection, &count, &counted, None, pieces(5));

    // The review shows the forgotten products before posting.
    let variances = inventory::count_variances(&connection, &count, &HashMap::new()).unwrap();
    let forgotten_row = variances
        .iter()
        .find(|row| row.product_id == forgotten)
        .unwrap();
    assert!(!forgotten_row.was_counted);
    assert_eq!(forgotten_row.difference, pieces(-4));

    post(&mut connection, &count, &HashMap::new());

    assert_eq!(
        inventory::on_hand(&connection, &counted).unwrap(),
        pieces(5)
    );
    assert_eq!(inventory::on_hand(&connection, &forgotten).unwrap(), Qty(0));
    assert_eq!(
        inventory::on_hand(&connection, &forgotten_with_batches).unwrap(),
        Qty(0)
    );
    assert!(
        inventory::batches_of(&connection, &forgotten_with_batches)
            .unwrap()
            .is_empty()
    );
    assert!(inventory::verify(&connection).unwrap().is_empty());
}

#[test]
fn inv_06_a_partial_count_leaves_uncounted_products_alone() {
    let mut connection = test_db();
    let counted = Id::new();
    let other = Id::new();
    receive(&mut connection, &counted, 5);
    receive(&mut connection, &other, 4);
    let count = new_count(&mut connection, CountScope::Partial);
    count_line(&mut connection, &count, &counted, None, pieces(2));

    post(&mut connection, &count, &HashMap::new());

    assert_eq!(inventory::on_hand(&connection, &other).unwrap(), pieces(4));
}

#[test]
fn inv_06_counting_by_batch_corrects_that_batch() {
    let mut connection = test_db();
    let product = Id::new();
    let first = batch(&mut connection, &product, Some("2026-05-01"));
    let second = batch(&mut connection, &product, Some("2026-06-01"));
    receive_into(&mut connection, &product, &first, 3);
    receive_into(&mut connection, &product, &second, 3);
    let count = new_count(&mut connection, CountScope::Partial);
    count_line(&mut connection, &count, &product, Some(&first), pieces(1));
    count_line(&mut connection, &count, &product, Some(&second), pieces(3));

    post(&mut connection, &count, &HashMap::new());

    let batches = inventory::batches_of(&connection, &product).unwrap();
    let first_row = batches.iter().find(|row| row.id == first).unwrap();
    assert_eq!(first_row.on_hand, pieces(1));
    assert_eq!(
        inventory::on_hand(&connection, &product).unwrap(),
        pieces(4)
    );
}

#[test]
fn inv_06_a_product_is_counted_by_batch_or_as_a_whole_not_both() {
    let mut connection = test_db();
    let product = Id::new();
    let first = batch(&mut connection, &product, Some("2026-05-01"));
    let count = new_count(&mut connection, CountScope::Partial);
    count_line(&mut connection, &count, &product, Some(&first), pieces(1));

    let transaction = connection.transaction().unwrap();
    let whole = CountEntry {
        product_id: product.clone(),
        batch_id: None,
        qty: pieces(1),
    };
    let error =
        inventory::record_count_line(&transaction, &count, &whole, Timestamp(3_000), &user())
            .unwrap_err();
    let AppError::Conflict { code, .. } = AppError::from(error) else {
        panic!("expected a conflict");
    };
    assert_eq!(code, "count_mixes_batch_and_whole");
}

#[test]
fn inv_06_a_scan_can_be_taken_back_but_not_below_zero() {
    let mut connection = test_db();
    let product = Id::new();
    let count = new_count(&mut connection, CountScope::Partial);
    count_line(&mut connection, &count, &product, None, pieces(2));
    count_line(&mut connection, &count, &product, None, pieces(-1));

    let transaction = connection.transaction().unwrap();
    let too_much = CountEntry {
        product_id: product.clone(),
        batch_id: None,
        qty: pieces(-2),
    };
    let error =
        inventory::record_count_line(&transaction, &count, &too_much, Timestamp(3_000), &user())
            .unwrap_err();
    assert_eq!(
        AppError::from(error),
        AppError::validation("counted_below_zero")
    );
    drop(transaction);

    let variances = inventory::count_variances(&connection, &count, &HashMap::new()).unwrap();
    assert_eq!(variances[0].counted, pieces(1));
}

#[test]
fn inv_06_a_posted_or_cancelled_count_takes_no_more_changes() {
    let mut connection = test_db();
    let product = Id::new();
    let posted = new_count(&mut connection, CountScope::Partial);
    post(&mut connection, &posted, &HashMap::new());
    let cancelled = new_count(&mut connection, CountScope::Partial);
    let transaction = connection.transaction().unwrap();
    inventory::cancel_count(&transaction, &cancelled).unwrap();
    transaction.commit().unwrap();
    assert_eq!(
        inventory::get_count(&connection, &cancelled)
            .unwrap()
            .status,
        CountStatus::Cancelled
    );

    let transaction = connection.transaction().unwrap();
    for count in [&posted, &cancelled] {
        let entry = CountEntry {
            product_id: product.clone(),
            batch_id: None,
            qty: pieces(1),
        };
        let error =
            inventory::record_count_line(&transaction, count, &entry, Timestamp(1), &user())
                .unwrap_err();
        assert!(matches!(AppError::from(error), AppError::Conflict { .. }));
        let error =
            inventory::post_count(&transaction, count, &HashMap::new(), &ctx()).unwrap_err();
        assert!(matches!(AppError::from(error), AppError::Conflict { .. }));
        let error = inventory::cancel_count(&transaction, count).unwrap_err();
        assert!(matches!(AppError::from(error), AppError::Conflict { .. }));
    }
}

#[test]
fn inv_06_a_missing_count_is_not_found() {
    let connection = test_db();
    let error = inventory::get_count(&connection, &Id::new()).unwrap_err();
    assert!(matches!(AppError::from(error), AppError::NotFound { .. }));
}

#[test]
fn inv_06_count_rows_use_the_cost_given() {
    let mut connection = test_db();
    let product = Id::new();
    receive(&mut connection, &product, 5);
    let count = new_count(&mut connection, CountScope::Partial);
    count_line(&mut connection, &count, &product, None, pieces(4));
    post(
        &mut connection,
        &count,
        &HashMap::from([(product.clone(), Money(1_250))]),
    );

    let cost: i64 = connection
        .query_row(
            "SELECT unit_cost_minor FROM inventory_ledger WHERE kind = 'count'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(cost, 1_250);
}
