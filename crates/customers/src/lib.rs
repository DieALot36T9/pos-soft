//! Customers, the points ledger, the credit ledger and balances.
//!
//! Page: `docs/05 Modules/Module - customers.md`. Rules: `Req - Customers and Loyalty`
//! (CUS-01 to CUS-08) and `Req - Payments` (PAY-04, PAY-05).

// Points and money maths must never use floating-point numbers.
#![deny(clippy::float_arithmetic)]

mod credit;
mod error;
mod kinds;
mod lookup;
mod phone;
mod points;
mod records;
mod reverse;
mod rows;
mod statement;
mod types;

pub use credit::{add_credit, oldest_unpaid_at, pay_with_credit, split_credit_payment};
pub use error::CustomersError;
pub use lookup::lookup;
pub use phone::normalize_phone;
pub use points::{
    add_points, eligible_for_points, max_points_to_redeem, points_earned, points_value,
};
pub use records::{create, get, set_archived, update};
pub use reverse::reverse_ref;
pub use statement::statement;
pub use types::{
    CreditKind, CreditSplit, Customer, CustomerDetails, CustomerHit, CustomerWithBalances,
    LedgerCtx, LedgerRef, LoyaltySettingsView, PaymentMethod, PointsKind, Statement, StatementLine,
};

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "customers",
        migrations: MIGRATIONS,
    }
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/001_init.sql"),
}];
