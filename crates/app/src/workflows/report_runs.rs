//! The Reports screen (Req - Reports RPT-01 to RPT-11): run any report, export it to
//! Excel, print it on the receipt printer. The A4 page and PDF are drawn by the screen
//! from the same `ReportTable` and go through the system print dialog (ADR-013).
//!
//! Reports only read. Nothing here writes to the database.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, FixedOffset, NaiveDateTime};
use io::{ExportHeader, ExportLabels};
use pos_core::{AppError, BusinessDate, CurrencyFormat, Id, ReportTable, Timestamp};
use reports::{DateRange, GroupBy, SalesDimension, SalesFilters, ShopContext};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use settings::{ShopProfile, TillSettings};
use ts_rs::TS;

use crate::state::AppState;
use crate::workflows::products::Choice;
use crate::workflows::stock;
use auth::SessionUser;

/// Which report to run, with its date range and filters. Every report on the screen
/// is one of these. Dates are business dates, both included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "report", rename_all = "snake_case")]
#[ts(export)]
pub enum ReportRequest {
    /// RPT-01, for one business day, all tills or one.
    DailySummary {
        date: BusinessDate,
        till_id: Option<Id>,
    },
    /// RPT-02: the X report (shift still open) or Z report (closed).
    Shift { shift_id: Id },
    /// RPT-03: sales by product, category, brand, cashier, hour, weekday or date.
    SalesBy {
        by: SalesBy,
        from: BusinessDate,
        to: BusinessDate,
        category_id: Option<Id>,
        brand_id: Option<Id>,
        cashier_id: Option<Id>,
        /// Keep only the first N rows. Example: 10 for "top 10".
        top: Option<u32>,
    },
    /// RPT-04: profit and margin from the cost saved on each sale line.
    Profit {
        by: ReportGroup,
        from: BusinessDate,
        to: BusinessDate,
    },
    /// RPT-05: on hand × cost.
    StockValue { by: ReportGroup },
    /// RPT-05: products below their reorder level.
    LowStock,
    /// RPT-05: batches that expire within `days` from today, and expired ones.
    NearExpiry { days: u32 },
    /// RPT-05: opening, received, sold, returned, adjusted and closing stock.
    StockMovement {
        from: BusinessDate,
        to: BusinessDate,
    },
    /// RPT-06 (TAX-09): tax by rate and component.
    Tax {
        from: BusinessDate,
        to: BusinessDate,
    },
    /// RPT-07: what we owe suppliers, by age.
    PayablesAging,
    /// RPT-07: what customers owe us, by age.
    ReceivablesAging,
    /// RPT-07: one supplier's statement.
    SupplierStatement {
        supplier_id: Id,
        from: BusinessDate,
        to: BusinessDate,
    },
    /// RPT-07: one customer's credit statement.
    CustomerStatement {
        customer_id: Id,
        from: BusinessDate,
        to: BusinessDate,
    },
    /// RPT-08: manual discounts, price overrides, voids and more, per cashier.
    DiscountsOverrides {
        from: BusinessDate,
        to: BusinessDate,
    },
}

/// What a sales analysis groups by (RPT-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SalesBy {
    Product,
    Category,
    Brand,
    Cashier,
    Hour,
    Weekday,
    Date,
}

/// What profit and stock value group by (RPT-04, RPT-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ReportGroup {
    Product,
    Category,
    Brand,
}

/// Everything the report screen offers to pick from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ReportChoices {
    /// Today's business date at this till, for the date presets.
    pub today: BusinessDate,
    pub tills: Vec<Choice>,
    pub cashiers: Vec<Choice>,
    pub categories: Vec<Choice>,
    pub brands: Vec<Choice>,
    pub suppliers: Vec<Choice>,
    /// How the shop writes money, for the money cells.
    pub currency: CurrencyFormat,
    /// The shop's name, printed at the top of the A4 page.
    pub shop_name: String,
}

/// One shift in the list of X and Z reports (RPT-02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ShiftChoice {
    pub id: Id,
    pub business_date: BusinessDate,
    pub till: String,
    pub cashier: String,
    /// Local time the shift opened. Example: "2026-03-02 08:00".
    pub opened: String,
    pub closed: bool,
}

/// The shifts a user may see, with what the screen needs to show their reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ShiftList {
    pub shifts: Vec<ShiftChoice>,
    pub currency: CurrencyFormat,
    pub shop_name: String,
}

/// How many shifts the X and Z list shows.
const SHIFT_LIST_LIMIT: u32 = 200;

/// How many customers the statement picker shows for a search.
const CUSTOMER_SEARCH_LIMIT: u32 = 20;

/// Runs one report on the reader connection.
pub fn run(state: &AppState, request: &ReportRequest) -> Result<ReportTable, AppError> {
    let offset = state.clock.local_offset_minutes();
    let today = stock::today(state)?;
    state
        .db
        .read(|conn| run_on(conn, offset, BusinessDate(today), request))
}

/// The report as an Excel file with the shop name, period and time made (RPT-09).
pub fn export_xlsx(state: &AppState, request: &ReportRequest, file: &Path) -> Result<(), AppError> {
    let table = run(state, request)?;
    let (shop, currency) = state.db.read(|conn| {
        let shop: ShopProfile = settings::get(conn)?;
        let currency: CurrencyFormat = settings::get(conn)?;
        Ok((shop, currency))
    })?;
    let header = ExportHeader {
        shop_name: shop.name,
        generated_at: local_date_time(state.clock.now(), state.clock.local_offset_minutes()),
        money_minor_digits: currency.minor_digits,
        labels: export_words(),
    };
    io::report_to_xlsx(file, &table, &header)?;
    Ok(())
}

/// The lists the report screen picks from.
pub fn choices(state: &AppState) -> Result<ReportChoices, AppError> {
    let today = stock::today(state)?;
    state.db.read(|conn| {
        let till: TillSettings = settings::get(conn)?;
        let shop: ShopProfile = settings::get(conn)?;
        let tills = till
            .till_id
            .map(|id| Choice {
                id,
                name: till.till_code.clone(),
            })
            .into_iter()
            .collect();
        let cashiers = auth::list_users(conn)?
            .into_iter()
            .map(|user| Choice {
                id: user.id,
                name: user.display_name,
            })
            .collect();
        let categories = catalog::list_categories(conn, false)?
            .into_iter()
            .map(|category| Choice {
                id: category.id,
                name: category.name,
            })
            .collect();
        let brands = catalog::list_brands(conn, false)?
            .into_iter()
            .map(|brand| Choice {
                id: brand.id,
                name: brand.name,
            })
            .collect();
        let suppliers = purchasing::list_suppliers(conn, true)?
            .into_iter()
            .map(|supplier| Choice {
                id: supplier.id,
                name: supplier.name,
            })
            .collect();
        Ok(ReportChoices {
            today: BusinessDate(today),
            tills,
            cashiers,
            categories,
            brands,
            suppliers,
            currency: settings::get(conn)?,
            shop_name: shop.name,
        })
    })
}

/// Customers whose name or phone matches what was typed, for the statement (RPT-07).
pub fn find_customers(state: &AppState, typed: &str) -> Result<Vec<Choice>, AppError> {
    state.db.read(|conn| {
        Ok(customers::lookup(conn, typed, CUSTOMER_SEARCH_LIMIT)?
            .into_iter()
            .map(|hit| Choice {
                id: hit.id,
                name: format!("{} ({})", hit.name, hit.phone),
            })
            .collect())
    })
}

/// The newest shifts first. With `only_user`, only that cashier's (RPT-10).
pub fn shifts(state: &AppState, only_user: Option<&Id>) -> Result<ShiftList, AppError> {
    let offset = state.clock.local_offset_minutes();
    state.db.read(|conn| {
        let context = shop_context(conn, offset)?;
        let shop: ShopProfile = settings::get(conn)?;
        let shifts = shifts::list(conn, only_user, SHIFT_LIST_LIMIT)?
            .into_iter()
            .map(|shift| ShiftChoice {
                till: name_or_id(&context.till_names, &shift.till_id),
                cashier: name_or_id(&context.user_names, &shift.user_id),
                opened: local_date_time(shift.opened_at, offset)
                    .format("%Y-%m-%d %H:%M")
                    .to_string(),
                closed: shift.status == shifts::ShiftStatus::Closed,
                business_date: shift.business_date,
                id: shift.id,
            })
            .collect();
        Ok(ShiftList {
            shifts,
            currency: settings::get(conn)?,
            shop_name: shop.name,
        })
    })
}

/// True when `request` is the X or Z report of a shift `user` owns. A cashier may see
/// only those (RPT-10); every other report needs `ReportsViewAll`.
pub fn is_own_shift(
    state: &AppState,
    user: &SessionUser,
    request: &ReportRequest,
) -> Result<bool, AppError> {
    let ReportRequest::Shift { shift_id } = request else {
        return Ok(false);
    };
    let shift = state.db.read(|conn| Ok(shifts::get(conn, shift_id)?))?;
    Ok(shift.user_id == user.id)
}

/// Runs a report on a connection the caller has open. Receipt printing uses this too.
pub fn run_on(
    conn: &Connection,
    utc_offset_minutes: i32,
    today: BusinessDate,
    request: &ReportRequest,
) -> Result<ReportTable, AppError> {
    let table = match request {
        ReportRequest::DailySummary { date, till_id } => {
            let context = shop_context(conn, utc_offset_minutes)?;
            reports::daily_summary(conn, *date, till_id.as_ref(), &context)?
        }
        ReportRequest::Shift { shift_id } => {
            let context = shop_context(conn, utc_offset_minutes)?;
            reports::shift_report(conn, shift_id, &context)?
        }
        ReportRequest::SalesBy {
            by,
            from,
            to,
            category_id,
            brand_id,
            cashier_id,
            top,
        } => {
            let context = shop_context(conn, utc_offset_minutes)?;
            let filters = SalesFilters {
                category_id: category_id.clone(),
                brand_id: brand_id.clone(),
                cashier_id: cashier_id.clone(),
                top: *top,
            };
            reports::sales_by(conn, dimension(*by), range(*from, *to), &filters, &context)?
        }
        ReportRequest::Profit { by, from, to } => {
            reports::profit(conn, group(*by), range(*from, *to))?
        }
        ReportRequest::StockValue { by } => reports::stock_value(conn, group(*by))?,
        ReportRequest::LowStock => reports::low_stock(conn)?,
        ReportRequest::NearExpiry { days } => reports::near_expiry(conn, today, *days)?,
        ReportRequest::StockMovement { from, to } => {
            reports::stock_movement(conn, range(*from, *to))?
        }
        ReportRequest::Tax { from, to } => reports::tax_report(conn, range(*from, *to))?,
        ReportRequest::PayablesAging => reports::payables_aging(conn, today)?,
        ReportRequest::ReceivablesAging => reports::receivables_aging(conn, today)?,
        ReportRequest::SupplierStatement {
            supplier_id,
            from,
            to,
        } => reports::supplier_statement(conn, supplier_id, range(*from, *to))?,
        ReportRequest::CustomerStatement {
            customer_id,
            from,
            to,
        } => reports::customer_statement(conn, customer_id, range(*from, *to))?,
        ReportRequest::DiscountsOverrides { from, to } => {
            let context = shop_context(conn, utc_offset_minutes)?;
            reports::discounts_overrides(conn, range(*from, *to), &context)?
        }
    };
    Ok(table)
}

/// The names reports show but don't own: users from `auth`, this till from `settings`,
/// payment methods from `sales`, and the local time offset.
pub(crate) fn shop_context(
    conn: &Connection,
    utc_offset_minutes: i32,
) -> Result<ShopContext, AppError> {
    let mut till_names = BTreeMap::new();
    let till: TillSettings = settings::get(conn)?;
    if let Some(till_id) = till.till_id {
        till_names.insert(till_id, till.till_code);
    }
    Ok(ShopContext {
        user_names: auth::list_users(conn)?
            .into_iter()
            .map(|user| (user.id, user.display_name))
            .collect(),
        till_names,
        payment_method_names: sales::payment_methods(conn)?
            .into_iter()
            .map(|method| (method.id, method.name))
            .collect(),
        utc_offset_minutes,
    })
}

/// The words written into an exported report, in English (ADR-022).
fn export_words() -> ExportLabels {
    ExportLabels {
        period: "Period".into(),
        period_to: "to".into(),
        generated: "Generated".into(),
        total: "Total".into(),
    }
}

fn range(from: BusinessDate, to: BusinessDate) -> DateRange {
    DateRange { from, to }
}

fn dimension(by: SalesBy) -> SalesDimension {
    match by {
        SalesBy::Product => SalesDimension::Product,
        SalesBy::Category => SalesDimension::Category,
        SalesBy::Brand => SalesDimension::Brand,
        SalesBy::Cashier => SalesDimension::Cashier,
        SalesBy::Hour => SalesDimension::Hour,
        SalesBy::Weekday => SalesDimension::Weekday,
        SalesBy::Date => SalesDimension::Date,
    }
}

fn group(by: ReportGroup) -> GroupBy {
    match by {
        ReportGroup::Product => GroupBy::Product,
        ReportGroup::Category => GroupBy::Category,
        ReportGroup::Brand => GroupBy::Brand,
    }
}

fn name_or_id(names: &BTreeMap<Id, String>, id: &Id) -> String {
    names
        .get(id)
        .cloned()
        .unwrap_or_else(|| id.as_str().to_string())
}

/// The shop's local date and time for a moment. A broken offset falls back to UTC.
fn local_date_time(at: Timestamp, utc_offset_minutes: i32) -> NaiveDateTime {
    let utc = DateTime::from_timestamp_millis(at.0).unwrap_or_default();
    match FixedOffset::east_opt(utc_offset_minutes * 60) {
        Some(offset) => utc.with_timezone(&offset).naive_local(),
        None => utc.naive_utc(),
    }
}
