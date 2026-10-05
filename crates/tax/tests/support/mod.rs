//! Helpers shared by this crate's tests.

use pos_core::{Bps, Timestamp};
use rusqlite::Connection;
use tax::{NewRate, RateComponent};

/// An empty in-memory database with only this module's migrations (Module Rules R6).
pub fn test_db() -> Connection {
    let connection = Connection::open_in_memory().expect("in-memory database must open");
    // The app turns foreign keys on for every connection, so tests do too.
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .expect("foreign keys must turn on");
    for migration in tax::migrations().migrations {
        connection
            .execute_batch(migration.sql)
            .expect("tax migrations must run");
    }
    connection
}

/// 2026-03-05 10:00 UTC, the time used by the shared test data.
pub const NOW: Timestamp = Timestamp(1_772_704_800_000);

/// A rate with no components. Example: `rate("Standard 15%", 1500)`.
pub fn rate(name: &str, rate_bps: u32) -> NewRate {
    NewRate {
        name: name.to_string(),
        rate_bps: Bps(rate_bps),
        components: Vec::new(),
        sort: 0,
    }
}

/// A named part of a rate. Example: `component("CGST", 900)`.
pub fn component(name: &str, rate_bps: u32) -> RateComponent {
    RateComponent {
        name: name.to_string(),
        rate_bps: Bps(rate_bps),
    }
}
