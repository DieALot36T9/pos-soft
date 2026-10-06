//! The stock ledger, stock levels, batches, adjustments, reorder rules and stock counts.
//!
//! Page: `docs/05 Modules/Module - inventory.md`. Every stock change goes through
//! `record` (or `reverse`), which writes ledger rows and updates the cached stock
//! in the same transaction (ADR-010).

mod batches;
mod convert;
mod count_post;
mod counts;
mod error;
mod fefo;
mod history;
mod ledger;
mod levels;
mod reasons;
mod record;
mod types;
mod verify;

pub use batches::{BatchRow, batches_of, create_batch, get_batch, near_expiry};
pub use count_post::{VarianceRow, count_variances, post_count};
pub use counts::{
    CountEntry, CountRef, CountRow, CountScope, CountStatus, NewCount, cancel_count, create_count,
    get_count, list_counts, record_count_line,
};
pub use error::InventoryError;
pub use history::{HistoryFilter, LedgerRowWithBalance, history};
pub use levels::{
    Availability, LowStockRow, ReorderRule, availability, is_product_used, low_stock,
    low_stock_count, on_hand, on_hand_many, remove_reorder_rule, reorder_rule, set_reorder_rule,
};
pub use reasons::{
    Reason, ReasonDirection, add_reason, list_reasons, rename_reason, set_reason_active,
};
pub use record::{record, reverse};
pub use types::{BatchChoice, LedgerRow, MoveCtx, MoveKind, Movement, Page, RecordedRow};
pub use verify::{Mismatch, verify};

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "inventory",
        migrations: MIGRATIONS,
    }
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/001_init.sql"),
}];
