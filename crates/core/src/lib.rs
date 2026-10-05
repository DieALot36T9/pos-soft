//! Shared value types and helpers used by every crate.
//! No business rules, no database and no I/O live here.
//!
//! Page: `docs/05 Modules/Module - core.md`. Rules for every calculation:
//! `docs/03 Architecture/Money Tax and Rounding.md`.

// Money maths must never use floating-point numbers.
#![deny(clippy::float_arithmetic)]

mod allocate;
mod bitmap;
mod bps;
mod cost;
mod currency;
mod error;
mod id;
mod migration;
mod money;
mod permission;
mod qty;
mod report;
mod rounding;
mod secret;
mod tax_mode;
mod time;

pub use allocate::allocate;
pub use bitmap::Bitmap1;
pub use bps::Bps;
pub use cost::moving_average;
pub use currency::{CurrencyFormat, Grouping, ParseMoneyError, format_money, parse_money};
pub use error::AppError;
pub use id::Id;
pub use migration::{Migration, ModuleMigrations};
pub use money::{Money, mul_qty, percent_of};
pub use permission::Permission;
pub use qty::Qty;
pub use report::{Cell, Column, ColumnKind, ReportFilter, ReportParams, ReportTable};
pub use rounding::{RoundingAppliesTo, RoundingMode, RoundingRule, cash_round, round_half_away};
pub use secret::Secret;
pub use tax_mode::TaxMode;
pub use time::{BusinessDate, Clock, FixedClock, SystemClock, Timestamp, business_date};
