//! Small helpers shared by this crate's tests.

#![allow(dead_code)] // each test file uses a different part of this file

use customers::{CustomerDetails, LedgerCtx, LedgerRef};
use pos_core::{BusinessDate, Id, Timestamp};
use rusqlite::{Connection, Transaction};

/// The shop's country in these tests: Bangladesh.
pub const COUNTRY: &str = "BD";

/// An in-memory database built only from this module's own migrations (Module Rules R6).
pub fn test_db() -> Connection {
    let connection = Connection::open_in_memory().expect("open an in-memory database");
    connection
        .execute_batch("PRAGMA foreign_keys = ON;")
        .expect("turn on foreign keys");
    for migration in customers::migrations().migrations {
        connection
            .execute_batch(migration.sql)
            .expect("run a migration");
    }
    connection
}

/// A ledger context on the given day, at the given moment.
pub fn ctx_on(day: &str, at: i64) -> LedgerCtx {
    LedgerCtx {
        at: Timestamp(at),
        business_date: BusinessDate::parse(day).expect("a test date"),
        user_id: None,
    }
}

/// A plain ledger context for tests that don't care about dates.
pub fn ctx() -> LedgerCtx {
    ctx_on("2026-10-04", 1_000)
}

/// A reference to a sale with a fresh id.
pub fn sale_ref() -> LedgerRef {
    LedgerRef {
        ref_type: "sale".to_string(),
        ref_id: Id::new(),
    }
}

/// Creates a customer with only a phone and a name.
pub fn quick_customer(tx: &Transaction, phone: &str, name: &str) -> Id {
    customers::create(
        tx,
        CustomerDetails::quick(phone, name),
        COUNTRY,
        Timestamp(1),
    )
    .expect("create a test customer")
}
