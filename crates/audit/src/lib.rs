//! The append-only audit log of sensitive actions (USR-08).
//!
//! Page: `docs/05 Modules/Module - audit.md`.

mod error;
mod query;
mod record;

pub use error::AuditError;
pub use query::{AuditFilter, AuditPage, AuditRow, query};
pub use record::{AuditCtx, AuditEvent, record};

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "audit",
        migrations: MIGRATIONS,
    }
}

// Add each file in `migrations/` here, in order. Never edit a file after release.
const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/001_init.sql"),
}];
