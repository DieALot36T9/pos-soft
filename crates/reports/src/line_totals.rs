//! Sale lines minus return lines, added up per group (product, category, hour, …).
//! Shared by `sales_by` (RPT-03) and `profit` (RPT-04).
//!
//! Sale lines count on the sale's business date and return lines on the return's own
//! business date. Voided sales are left out. Sale lines group by the category and brand
//! saved on the line; return lines have none saved, so they use the product's current
//! category and brand from the catalog.

use std::collections::BTreeMap;

use pos_core::{AppError, Money};
use rusqlite::Connection;
use rusqlite::types::Value;

use crate::sql::{Param, optional_text, query, query_one, text};
use crate::table::{add, sub};
use crate::{DateRange, ReportsError, SalesDimension, SalesFilters};

/// One group's totals. Amounts are sales minus returns, except `discount` (sales only)
/// and `sales` (the number of sales with a line in this group).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct GroupTotals {
    /// The group's key. Example: a category id, or "15" for the hour 15:00.
    pub(crate) key: String,
    /// The name on the lines, used when the catalog has no name for the key.
    pub(crate) line_name: String,
    pub(crate) qty: i64,
    pub(crate) net: Money,
    pub(crate) tax: Money,
    pub(crate) discount: Money,
    pub(crate) cost: Money,
    pub(crate) sales: i64,
}

/// The SQL key for sale lines (alias `l`) and for return lines (alias `r`, with the
/// product as `p`). Hours use local time: `:offset_seconds` from UTC.
fn key_sql(dimension: SalesDimension) -> (&'static str, &'static str) {
    match dimension {
        SalesDimension::Product => ("l.product_id", "r.product_id"),
        SalesDimension::Category => ("l.category_id", "COALESCE(p.category_id, '')"),
        SalesDimension::Brand => ("COALESCE(l.brand_id, '')", "COALESCE(p.brand_id, '')"),
        SalesDimension::Cashier => ("l.user_id", "r.user_id"),
        SalesDimension::Hour => (
            "printf('%02d', ((l.completed_at / 1000 + :offset_seconds) % 86400 + 86400) % 86400 / 3600)",
            "printf('%02d', ((r.at / 1000 + :offset_seconds) % 86400 + 86400) % 86400 / 3600)",
        ),
        SalesDimension::Weekday => (
            "strftime('%w', l.business_date)",
            "strftime('%w', r.business_date)",
        ),
        SalesDimension::Date => ("l.business_date", "r.business_date"),
    }
}

const SALE_FILTERS: &str = "l.status = 'completed' AND l.business_date BETWEEN :from AND :to
  AND (:category IS NULL OR l.category_id = :category)
  AND (:brand IS NULL OR l.brand_id = :brand)
  AND (:cashier IS NULL OR l.user_id = :cashier)";

const RETURN_FILTERS: &str = "r.business_date BETWEEN :from AND :to
  AND (:category IS NULL OR p.category_id = :category)
  AND (:brand IS NULL OR p.brand_id = :brand)
  AND (:cashier IS NULL OR r.user_id = :cashier)";

fn params(range: DateRange, filters: &SalesFilters, offset_minutes: i32) -> Vec<Param> {
    vec![
        (":from", text(&range.from.to_string())),
        (":to", text(&range.to.to_string())),
        (
            ":category",
            optional_text(filters.category_id.as_ref().map(|id| id.as_str())),
        ),
        (
            ":brand",
            optional_text(filters.brand_id.as_ref().map(|id| id.as_str())),
        ),
        (
            ":cashier",
            optional_text(filters.cashier_id.as_ref().map(|id| id.as_str())),
        ),
        (
            ":offset_seconds",
            Value::Integer(i64::from(offset_minutes) * 60),
        ),
    ]
}

/// Sales minus returns per group, in key order.
pub(crate) fn group_totals(
    conn: &Connection,
    dimension: SalesDimension,
    range: DateRange,
    filters: &SalesFilters,
    offset_minutes: i32,
) -> Result<Vec<GroupTotals>, ReportsError> {
    let params = params(range, filters, offset_minutes);
    let (sale_key, return_key) = key_sql(dimension);
    let mut groups: BTreeMap<String, GroupTotals> = BTreeMap::new();
    let sales_sql = format!(
        "SELECT {sale_key}, MAX(l.name), SUM(l.qty_milli), SUM(l.net_minor), SUM(l.tax_minor),
           SUM(l.promo_discount_minor + l.tier_discount_minor + l.manual_discount_minor
               + l.invoice_discount_minor),
           SUM(l.cost_minor), COUNT(DISTINCT l.sale_id)
         FROM sales_v_lines l WHERE {SALE_FILTERS} GROUP BY 1"
    );
    for group in query(conn, &sales_sql, &params, |row| {
        Ok(GroupTotals {
            key: row.get(0)?,
            line_name: row.get(1)?,
            qty: row.get(2)?,
            net: Money(row.get(3)?),
            tax: Money(row.get(4)?),
            discount: Money(row.get(5)?),
            cost: Money(row.get(6)?),
            sales: row.get(7)?,
        })
    })? {
        groups.insert(group.key.clone(), group);
    }
    let returns_sql = format!(
        "SELECT {return_key}, MAX(r.name), SUM(r.qty_milli), SUM(r.net_minor), SUM(r.tax_minor),
           SUM(r.cost_minor)
         FROM sales_v_return_lines r LEFT JOIN catalog_v_products p ON p.id = r.product_id
         WHERE {RETURN_FILTERS} GROUP BY 1"
    );
    for returned in query(conn, &returns_sql, &params, |row| {
        Ok(GroupTotals {
            key: row.get(0)?,
            line_name: row.get(1)?,
            qty: row.get(2)?,
            net: Money(row.get(3)?),
            tax: Money(row.get(4)?),
            cost: Money(row.get(5)?),
            ..GroupTotals::default()
        })
    })? {
        let group = groups
            .entry(returned.key.clone())
            .or_insert_with(|| GroupTotals {
                key: returned.key.clone(),
                line_name: returned.line_name.clone(),
                ..GroupTotals::default()
            });
        take_off(group, &returned)?;
    }
    Ok(groups.into_values().collect())
}

/// Subtracts a group's returns from its sales.
fn take_off(group: &mut GroupTotals, returned: &GroupTotals) -> Result<(), AppError> {
    group.qty = sub(group.qty, returned.qty)?;
    group.net = group.net.checked_sub(returned.net)?;
    group.tax = group.tax.checked_sub(returned.tax)?;
    group.cost = group.cost.checked_sub(returned.cost)?;
    Ok(())
}

/// The totals of all groups. `sales` is counted again, because one sale can have
/// lines in several groups.
pub(crate) fn overall(
    conn: &Connection,
    groups: &[GroupTotals],
    range: DateRange,
    filters: &SalesFilters,
) -> Result<GroupTotals, ReportsError> {
    let mut total = GroupTotals::default();
    for group in groups {
        total.qty = add(total.qty, group.qty)?;
        total.net = total.net.checked_add(group.net)?;
        total.tax = total.tax.checked_add(group.tax)?;
        total.discount = total.discount.checked_add(group.discount)?;
        total.cost = total.cost.checked_add(group.cost)?;
    }
    let sql = format!("SELECT COUNT(DISTINCT l.sale_id) FROM sales_v_lines l WHERE {SALE_FILTERS}");
    total.sales = query_one(conn, &sql, &params(range, filters, 0), |row| row.get(0))?;
    Ok(total)
}
