//! Carts, held sales, completed sales, payments, invoice numbers, returns and voids.
//!
//! Page: `docs/05 Modules/Module - sales.md`. Not built yet: see "Open items" there.

// Money maths must never use floating-point numbers.
#![deny(clippy::float_arithmetic)]

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "sales",
        migrations: MIGRATIONS,
    }
}

// Add each file in `migrations/` here, in order. Example:
// Migration { version: 1, name: "init", sql: include_str!("../migrations/001_init.sql") }
const MIGRATIONS: &[Migration] = &[];
