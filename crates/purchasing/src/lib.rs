//! Suppliers, purchase orders, goods received, supplier returns and the payables ledger.
//!
//! Page: `docs/05 Modules/Module - purchasing.md`. Not built yet: see "Open items" there.

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "purchasing",
        migrations: MIGRATIONS,
    }
}

// Add each file in `migrations/` here, in order. Example:
// Migration { version: 1, name: "init", sql: include_str!("../migrations/001_init.sql") }
const MIGRATIONS: &[Migration] = &[];
