//! Helpers shared by the test files: a fresh database and ready-made inputs.
// Each test file uses only some of these helpers.
#![allow(dead_code)]

use chrono::NaiveDate;
use inventory::{BatchChoice, MoveCtx, MoveKind, Movement};
use pos_core::{BusinessDate, Id, Money, Qty, Timestamp};
use rusqlite::Connection;

/// An empty in-memory database with only this module's migrations (Module Rules R6).
pub fn test_db() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .unwrap();
    for migration in inventory::migrations().migrations {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
}

/// Whole pieces as a quantity: `pieces(3)` is `Qty(3000)`.
pub fn pieces(count: i64) -> Qty {
    Qty(count * 1000)
}

/// A calendar date from "YYYY-MM-DD".
pub fn date(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
}

/// The till, user and time used by most tests.
pub fn ctx() -> MoveCtx {
    ctx_at(1_000)
}

/// Like `ctx()`, but at a chosen moment, so history order can be tested.
pub fn ctx_at(at_ms: i64) -> MoveCtx {
    MoveCtx {
        at: Timestamp(at_ms),
        business_date: BusinessDate(date("2026-03-02")),
        till_id: Id::parse("01HX5Z3J8Q9W2E4R6T8Y0V1K2M").unwrap(),
        user_id: Some(Id::parse("01HX5Z3J8Q9W2E4R6T8Y0V1K2N").unwrap()),
    }
}

/// The user who counts and posts stock counts in tests.
pub fn user() -> Id {
    Id::parse("01HX5Z3J8Q9W2E4R6T8Y0V1K2N").unwrap()
}

/// Receives `count` pieces of a product into a specific batch.
pub fn receive_into(connection: &mut Connection, product: &Id, batch: &Id, count: i64) {
    let transaction = connection.transaction().unwrap();
    let movement = Movement {
        batch: BatchChoice::Specific(batch.clone()),
        unit_cost: Money(5_000),
        ..Movement::new(product, pieces(count), MoveKind::Receive, ctx())
    };
    inventory::record(&transaction, movement, true).unwrap();
    transaction.commit().unwrap();
}

/// Receives `count` pieces of a product with no batch.
pub fn receive(connection: &mut Connection, product: &Id, count: i64) {
    let transaction = connection.transaction().unwrap();
    let movement = Movement::new(product, pieces(count), MoveKind::Receive, ctx());
    inventory::record(&transaction, movement, true).unwrap();
    transaction.commit().unwrap();
}

/// Makes a batch for a product with an optional expiry date ("YYYY-MM-DD").
pub fn batch(connection: &mut Connection, product: &Id, expiry: Option<&str>) -> Id {
    let transaction = connection.transaction().unwrap();
    let id = inventory::create_batch(
        &transaction,
        product,
        None,
        expiry.map(date),
        Money(5_000),
        Timestamp(500),
    )
    .unwrap();
    transaction.commit().unwrap();
    id
}
