//! Users, roles, permissions, passwords and PINs, admin approvals and recovery.
//!
//! Page: `docs/05 Modules/Module - auth.md`. Not built yet: see "Open items" there.

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "auth",
        migrations: MIGRATIONS,
    }
}

// Add each file in `migrations/` here, in order. Example:
// Migration { version: 1, name: "init", sql: include_str!("../migrations/001_init.sql") }
const MIGRATIONS: &[Migration] = &[];
