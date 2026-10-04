//! The stock ledger is append-only (ADR-010): the database refuses any UPDATE or
//! DELETE of a ledger row. Corrections are new rows. Requirement INV-01.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod support;

use pos_core::Id;
use support::{receive, test_db};

#[test]
fn inv_01_ledger_rows_cannot_be_changed() {
    let mut connection = test_db();
    receive(&mut connection, &Id::new(), 3);

    let error = connection
        .execute("UPDATE inventory_ledger SET qty_delta_milli = 5000", [])
        .unwrap_err();

    assert!(error.to_string().contains("append-only"), "{error}");
}

#[test]
fn inv_01_ledger_rows_cannot_be_deleted() {
    let mut connection = test_db();
    receive(&mut connection, &Id::new(), 3);

    let error = connection
        .execute("DELETE FROM inventory_ledger", [])
        .unwrap_err();

    assert!(error.to_string().contains("append-only"), "{error}");
    let rows: i64 = connection
        .query_row("SELECT COUNT(*) FROM inventory_ledger", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(rows, 1);
}
