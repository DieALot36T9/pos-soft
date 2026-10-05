//! Read-only report queries over published views, each giving a ReportTable.
//!
//! Page: `docs/05 Modules/Module - reports.md`. This module owns no tables. It reads
//! only other modules' `*_v_*` views, through the reader connection `app` passes in.

mod aging;
mod daily;
mod error;
mod line_totals;
mod overrides;
mod sales_by;
mod shift;
mod sql;
mod stock;
mod summary;
mod table;
mod tax;
mod types;

pub use aging::{payables_aging, receivables_aging};
pub use daily::daily_summary;
pub use error::ReportsError;
pub use overrides::discounts_overrides;
pub use sales_by::{profit, sales_by};
pub use shift::shift_report;
pub use stock::{low_stock, near_expiry, stock_movement, stock_value};
pub use tax::tax_report;
pub use types::{
    ACTION_CART_CANCEL, ACTION_DRAWER_NO_SALE, ACTION_SALE_VOID, DateRange, GroupBy,
    SalesDimension, SalesFilters, ShopContext,
};

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations: none, because reports own no tables.
/// `app` can still pass them to `db::migrate` like every other module's.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "reports",
        migrations: MIGRATIONS,
    }
}

const MIGRATIONS: &[Migration] = &[];
