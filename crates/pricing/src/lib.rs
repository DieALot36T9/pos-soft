//! Price lists, tiers and promotions, and the pricing engine.
//!
//! Page: `docs/05 Modules/Module - pricing.md`. Not built yet: see "Open items" there.

// Money maths must never use floating-point numbers.
#![deny(clippy::float_arithmetic)]

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "pricing",
        migrations: MIGRATIONS,
    }
}

// Add each file in `migrations/` here, in order. Example:
// Migration { version: 1, name: "init", sql: include_str!("../migrations/001_init.sql") }
const MIGRATIONS: &[Migration] = &[];
