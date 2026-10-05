//! Tax rates and their components, and the tax calculation.
//! This is the only place in the system that calculates tax.
//!
//! Page: `docs/05 Modules/Module - tax.md`. Rules: `docs/03 Architecture/Money Tax and Rounding.md`.

// Money maths must never use floating-point numbers.
#![deny(clippy::float_arithmetic)]

mod calc;
mod error;
mod rates;
mod types;

use pos_core::{Migration, ModuleMigrations};

pub use calc::{calc_line, split_components};
pub use error::{Result, TaxError};
pub use rates::{create_rate, deactivate, get, get_many, list, update_rate};
pub use types::{LineTax, NewRate, RateComponent, RateUpdate, RateWithComponents, TaxMode};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "tax",
        migrations: MIGRATIONS,
    }
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/001_init.sql"),
}];
