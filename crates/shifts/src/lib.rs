//! Shifts, cash movements, cash counts and shift summaries.
//!
//! Page: `docs/05 Modules/Module - shifts.md`.

mod error;
mod model;
mod movement;
mod rules;
mod shift;
mod store;
mod summary;

pub use error::ShiftsError;
pub use model::{
    CASH, CloseResult, DenominationCount, MethodTotal, MovementKind, MovementTotal, Shift,
    ShiftMovement, ShiftStatus, ShiftSummary,
};
pub use movement::{expected_cash, record, reverse_ref};
pub use shift::{close, current, open};
pub use summary::summary;

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "shifts",
        migrations: MIGRATIONS,
    }
}

// Add each new file in `migrations/` here, in order. Never edit a released one.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "init",
        sql: include_str!("../migrations/001_init.sql"),
    },
    Migration {
        version: 2,
        name: "counts_view",
        sql: include_str!("../migrations/002_counts_view.sql"),
    },
];
