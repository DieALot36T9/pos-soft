//! Reading stock: levels, reorder rules, low stock, batches and near expiry.
//! Requirements INV-01, INV-03 and INV-05.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use inventory::{BatchChoice, MoveKind, Movement, Page, ReorderRule};
use pos_core::{AppError, Id, Qty};
use support::{batch, ctx, date, pieces, receive, receive_into, test_db};

#[test]
fn inv_01_a_product_never_moved_has_no_stock() {
    let connection = test_db();
    assert_eq!(inventory::on_hand(&connection, &Id::new()).unwrap(), Qty(0));
}

#[test]
fn inv_01_on_hand_many_answers_for_every_id_asked() {
    let mut connection = test_db();
    let stocked = Id::new();
    let empty = Id::new();
    receive(&mut connection, &stocked, 4);

    let levels = inventory::on_hand_many(&connection, &[stocked.clone(), empty.clone()]).unwrap();

    assert_eq!(levels.len(), 2);
    assert_eq!(levels[&stocked], pieces(4));
    assert_eq!(levels[&empty], Qty(0));
}

#[test]
fn inv_03_reorder_level_10_and_stock_9_shows_in_low_stock() {
    let mut connection = test_db();
    let product = Id::new();
    receive(&mut connection, &product, 10);
    let transaction = connection.transaction().unwrap();
    inventory::set_reorder_rule(&transaction, &product, pieces(10), pieces(24)).unwrap();
    transaction.commit().unwrap();
    // At the reorder level itself the product is not low yet.
    assert!(
        inventory::low_stock(&connection, Page::first(50))
            .unwrap()
            .is_empty()
    );
    assert_eq!(inventory::low_stock_count(&connection).unwrap(), 0);

    let transaction = connection.transaction().unwrap();
    inventory::record(
        &transaction,
        Movement::new(&product, pieces(-1), MoveKind::Sale, ctx()),
        true,
    )
    .unwrap();
    transaction.commit().unwrap();

    let low = inventory::low_stock(&connection, Page::first(50)).unwrap();
    assert_eq!(low.len(), 1);
    assert_eq!(low[0].product_id, product);
    assert_eq!(low[0].on_hand, pieces(9));
    assert_eq!(low[0].reorder_level, pieces(10));
    assert_eq!(low[0].reorder_qty, pieces(24));
    assert_eq!(inventory::low_stock_count(&connection).unwrap(), 1);
}

#[test]
fn inv_03_a_product_with_a_rule_but_no_stock_yet_is_low() {
    let mut connection = test_db();
    let product = Id::new();
    let transaction = connection.transaction().unwrap();
    inventory::set_reorder_rule(&transaction, &product, pieces(5), Qty(0)).unwrap();
    transaction.commit().unwrap();

    let low = inventory::low_stock(&connection, Page::first(50)).unwrap();
    assert_eq!(low.len(), 1);
    assert_eq!(low[0].on_hand, Qty(0));
}

#[test]
fn inv_03_low_stock_lists_the_biggest_shortfall_first_and_pages() {
    let mut connection = test_db();
    let a_little_short = Id::new();
    let very_short = Id::new();
    let transaction = connection.transaction().unwrap();
    inventory::set_reorder_rule(&transaction, &a_little_short, pieces(1), Qty(0)).unwrap();
    inventory::set_reorder_rule(&transaction, &very_short, pieces(50), Qty(0)).unwrap();
    transaction.commit().unwrap();

    let first_page = inventory::low_stock(&connection, Page::first(1)).unwrap();
    let second_page = inventory::low_stock(
        &connection,
        Page {
            offset: 1,
            limit: 1,
        },
    )
    .unwrap();

    assert_eq!(first_page[0].product_id, very_short);
    assert_eq!(second_page[0].product_id, a_little_short);
}

#[test]
fn inv_03_reorder_rules_can_be_read_changed_and_removed() {
    let mut connection = test_db();
    let product = Id::new();
    let transaction = connection.transaction().unwrap();
    inventory::set_reorder_rule(&transaction, &product, pieces(5), pieces(10)).unwrap();
    inventory::set_reorder_rule(&transaction, &product, pieces(6), pieces(12)).unwrap();
    assert_eq!(
        inventory::reorder_rule(&transaction, &product).unwrap(),
        Some(ReorderRule {
            level: pieces(6),
            qty: pieces(12)
        })
    );
    inventory::remove_reorder_rule(&transaction, &product).unwrap();
    assert_eq!(
        inventory::reorder_rule(&transaction, &product).unwrap(),
        None
    );

    let error =
        inventory::set_reorder_rule(&transaction, &product, pieces(-1), Qty(0)).unwrap_err();
    assert_eq!(
        AppError::from(error),
        AppError::validation("reorder_below_zero")
    );
}

#[test]
fn inv_05_near_expiry_lists_batches_expiring_within_n_days() {
    let mut connection = test_db();
    let product = Id::new();
    let expired = batch(&mut connection, &product, Some("2026-02-20"));
    let soon = batch(&mut connection, &product, Some("2026-03-10"));
    let later = batch(&mut connection, &product, Some("2026-06-01"));
    let _empty = batch(&mut connection, &product, Some("2026-03-05"));
    let undated = batch(&mut connection, &product, None);
    for id in [&expired, &soon, &later, &undated] {
        receive_into(&mut connection, &product, id, 2);
    }

    let near = inventory::near_expiry(&connection, date("2026-03-02"), 30).unwrap();

    let ids: Vec<Id> = near.into_iter().map(|row| row.id).collect();
    // Already expired first; empty batches and batches without a date are left out.
    assert_eq!(ids, vec![expired, soon]);
}

#[test]
fn inv_05_availability_warns_about_an_expired_batch_with_stock() {
    let mut connection = test_db();
    let product = Id::new();
    let dated = batch(&mut connection, &product, Some("2026-03-01"));
    receive_into(&mut connection, &product, &dated, 2);

    // A batch is still good on its expiry date.
    let on_the_day = inventory::availability(&connection, &product, date("2026-03-01")).unwrap();
    let day_after = inventory::availability(&connection, &product, date("2026-03-02")).unwrap();

    assert_eq!(on_the_day.on_hand, pieces(2));
    assert!(!on_the_day.has_expired_batch);
    assert!(day_after.has_expired_batch);
}

#[test]
fn inv_05_create_batch_keeps_its_details() {
    let mut connection = test_db();
    let product = Id::new();
    let transaction = connection.transaction().unwrap();
    let id = inventory::create_batch(
        &transaction,
        &product,
        Some("L-42"),
        Some(date("2026-12-31")),
        pos_core::Money(7_500),
        pos_core::Timestamp(9),
    )
    .unwrap();
    let movement = Movement {
        batch: BatchChoice::Specific(id.clone()),
        ..Movement::new(&product, pieces(1), MoveKind::Receive, ctx())
    };
    inventory::record(&transaction, movement, true).unwrap();
    transaction.commit().unwrap();

    let rows = inventory::batches_of(&connection, &product).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, id);
    assert_eq!(rows[0].batch_no.as_deref(), Some("L-42"));
    assert_eq!(rows[0].expiry_date, Some(date("2026-12-31")));
    assert_eq!(rows[0].unit_cost, pos_core::Money(7_500));
    assert_eq!(rows[0].received_at, pos_core::Timestamp(9));
}
