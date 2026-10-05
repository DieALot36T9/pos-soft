//! Daily summary (RPT-01).

use pos_core::{BusinessDate, Id, ReportTable};
use rusqlite::Connection;

use crate::summary::{Scope, sales_summary};
use crate::table::{day_params, filter};
use crate::{ReportsError, ShopContext};

/// The day's sales, discounts, returns, tax by rate, payments by method, voids and
/// cancelled carts, for one business date on one till (or all tills when `till` is None).
pub fn daily_summary(
    conn: &Connection,
    date: BusinessDate,
    till: Option<&Id>,
    shop: &ShopContext,
) -> Result<ReportTable, ReportsError> {
    let scope = Scope::Day { date, till };
    let mut report = sales_summary(conn, "Daily summary", &scope, shop)?;
    report.params = day_params(date);
    if let Some(till) = till {
        report
            .params
            .filters
            .push(filter("Till", &shop.till_name(till.as_str())));
    }
    Ok(report)
}
