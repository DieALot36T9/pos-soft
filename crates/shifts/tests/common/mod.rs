//! Helpers shared by the test files.

// Test helpers may stop with unwrap, like tests do. Not every test file uses every helper.
#![allow(clippy::unwrap_used, dead_code)]

use pos_core::{BusinessDate, Id, Money, Timestamp};
use rusqlite::{Connection, Transaction};
use shifts::{MovementKind, ShiftMovement};

/// An in-memory database built by this module's own migrations (rule R6).
pub fn test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    for migration in shifts::migrations().migrations {
        conn.execute_batch(migration.sql).unwrap();
    }
    conn
}

/// Runs `work` in a transaction and commits it, like an `app` workflow does.
pub fn write<T>(conn: &mut Connection, work: impl FnOnce(&Transaction) -> T) -> T {
    let tx = conn.transaction().unwrap();
    let result = work(&tx);
    tx.commit().unwrap();
    result
}

/// Whole taka as money in paisa. Example: `taka(12_450)` is 12,450.00.
pub fn taka(whole: i64) -> Money {
    Money(whole * 100)
}

pub fn today() -> BusinessDate {
    BusinessDate::parse("2026-03-02").unwrap()
}

pub const NOW: Timestamp = Timestamp(1_772_400_000_000);

/// Opens a shift on `till` with `float` and no denomination count.
pub fn open_shift(conn: &mut Connection, till: &Id, user: &Id, float: Money) -> Id {
    write(conn, |tx| {
        shifts::open(tx, till, user, float, &[], today(), NOW).unwrap()
    })
}

/// A movement with the usual fields filled in. Change what a test cares about.
pub fn movement(kind: MovementKind, method_kind: &str, amount: Money, user: &Id) -> ShiftMovement {
    ShiftMovement {
        kind,
        method_id: None,
        method_kind: method_kind.to_string(),
        amount,
        ref_type: None,
        ref_id: None,
        reason: None,
        at: NOW,
        user_id: user.clone(),
    }
}

/// A movement that carries a reason, as cash in, cash out and cash drops must.
pub fn with_reason(mut movement: ShiftMovement, reason: &str) -> ShiftMovement {
    movement.reason = Some(reason.to_string());
    movement
}

/// A movement that belongs to a document. Example: a sale payment for sale `id`.
pub fn for_ref(mut movement: ShiftMovement, ref_type: &str, ref_id: &Id) -> ShiftMovement {
    movement.ref_type = Some(ref_type.to_string());
    movement.ref_id = Some(ref_id.clone());
    movement
}

/// Records one movement and commits it.
pub fn record(conn: &mut Connection, shift: &Id, movement: ShiftMovement) {
    write(conn, |tx| shifts::record(tx, shift, movement).unwrap());
}
