//! Sale lines minus return lines, added up per group (product, category, hour, …).
//! Shared by `sales_by` (RPT-03) and `profit` (RPT-04).
//!
//! Sale lines count on the sale's business date and return lines on the return's own
//! business date. Voided sales are left out. Sale lines group by the category and brand
//! saved on the line; return lines have none saved, so they use the product's current
//! category and brand from the catalog.
//!
//! Speed (RPT-11): over a year there are about 1.6 million sale lines. They are read
//! once and added up here, not with SQL `GROUP BY`, because the database would first
//! sort every line (over 6 s on the reference data). Hours and weekdays are worked out
//! here too, which is cheaper than asking the database to format every line.

use std::collections::HashMap;
use std::fmt::Write as _;

use chrono::Datelike;
use pos_core::{AppError, BusinessDate, Id, Money};
use rusqlite::Connection;
use rusqlite::types::Value;

use crate::sql::{Param, optional_text, query, text};
use crate::table::{add, sub};
use crate::{DateRange, ReportsError, SalesDimension, SalesFilters};

/// One group's totals. Amounts are sales minus returns, except `discount` (sales only)
/// and `sales` (the number of sales with a line in this group).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct GroupTotals {
    /// The group's key. Example: a category id, "15" for the hour 15:00, or "4" for Thursday.
    pub(crate) key: String,
    pub(crate) qty: i64,
    pub(crate) net: Money,
    pub(crate) tax: Money,
    pub(crate) discount: Money,
    pub(crate) cost: Money,
    pub(crate) sales: i64,
}

/// Sales minus returns per group (in key order), and the number of sales overall.
pub(crate) struct LineTotals {
    pub(crate) groups: Vec<GroupTotals>,
    /// Sales with at least one line in any group. Counted apart, because one sale can
    /// have lines in several groups.
    pub(crate) sales: i64,
}

/// What the sale-line query reads for the key: a column of alias `l`.
/// Hours and weekdays are turned into their key by `SaleKey`.
fn sale_key_column(dimension: SalesDimension) -> &'static str {
    match dimension {
        SalesDimension::Product => "l.product_id",
        SalesDimension::Category => "l.category_id",
        SalesDimension::Brand => "COALESCE(l.brand_id, '')",
        SalesDimension::Cashier => "l.user_id",
        SalesDimension::Hour => "l.completed_at",
        SalesDimension::Weekday | SalesDimension::Date => "l.business_date",
    }
}

/// The key for return lines (alias `r`, with the product as `p`). Returns are few,
/// so the database can work out hours and weekdays for them.
fn return_key_sql(dimension: SalesDimension) -> &'static str {
    match dimension {
        SalesDimension::Product => "r.product_id",
        SalesDimension::Category => "COALESCE(p.category_id, '')",
        SalesDimension::Brand => "COALESCE(p.brand_id, '')",
        SalesDimension::Cashier => "r.user_id",
        SalesDimension::Hour => {
            "printf('%02d', ((r.at / 1000 + :offset_seconds) % 86400 + 86400) % 86400 / 3600)"
        }
        SalesDimension::Weekday => "strftime('%w', r.business_date)",
        SalesDimension::Date => "r.business_date",
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
    let id_text = |id: &Option<Id>| optional_text(id.as_ref().map(Id::as_str));
    vec![
        (":from", text(&range.from.to_string())),
        (":to", text(&range.to.to_string())),
        (":category", id_text(&filters.category_id)),
        (":brand", id_text(&filters.brand_id)),
        (":cashier", id_text(&filters.cashier_id)),
        (
            ":offset_seconds",
            Value::Integer(i64::from(offset_minutes) * 60),
        ),
    ]
}

/// Adds up sale lines per group, then takes off the returns.
///
/// The lines come sorted by date and sale (the date index already gives the date order,
/// so the database only sorts within each day). All lines of one sale arrive together,
/// so each sale is counted exactly once per group.
pub(crate) fn group_totals(
    conn: &Connection,
    dimension: SalesDimension,
    range: DateRange,
    filters: &SalesFilters,
    offset_minutes: i32,
) -> Result<LineTotals, ReportsError> {
    let params = params(range, filters, offset_minutes);
    let mut adder = GroupAdder::default();
    let mut key = SaleKey::new(dimension, offset_minutes);
    let sales_sql = format!(
        "SELECT {key_column}, l.sale_id, l.qty_milli, l.net_minor, l.tax_minor,
           l.promo_discount_minor + l.tier_discount_minor + l.manual_discount_minor
             + l.invoice_discount_minor,
           l.cost_minor
         FROM sales_v_lines l WHERE {SALE_FILTERS}
         ORDER BY l.business_date, l.sale_id",
        key_column = sale_key_column(dimension)
    );
    query(conn, &sales_sql, &params, |row| {
        adder.next_line_of(row.get_ref(1)?.as_str()?);
        let group = adder.line_group(key.of(row)?);
        group.qty = checked(group.qty, row.get(2)?)?;
        group.net = Money(checked(group.net.0, row.get(3)?)?);
        group.tax = Money(checked(group.tax.0, row.get(4)?)?);
        group.discount = Money(checked(group.discount.0, row.get(5)?)?);
        group.cost = Money(checked(group.cost.0, row.get(6)?)?);
        Ok(())
    })?;
    let returns_sql = format!(
        "SELECT {return_key}, SUM(r.qty_milli), SUM(r.net_minor), SUM(r.tax_minor),
           SUM(r.cost_minor)
         FROM sales_v_return_lines r LEFT JOIN catalog_v_products p ON p.id = r.product_id
         WHERE {RETURN_FILTERS} GROUP BY 1",
        return_key = return_key_sql(dimension)
    );
    for returned in query(conn, &returns_sql, &params, |row| {
        Ok(GroupTotals {
            key: row.get(0)?,
            qty: row.get(1)?,
            net: Money(row.get(2)?),
            tax: Money(row.get(3)?),
            cost: Money(row.get(4)?),
            ..GroupTotals::default()
        })
    })? {
        take_off(&mut adder.group(&returned.key).totals, &returned)?;
    }
    let sales = adder.sales_seen as i64;
    let mut groups: Vec<GroupTotals> = adder.groups.into_iter().map(|slot| slot.totals).collect();
    groups.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(LineTotals { groups, sales })
}

/// Turns the key column of a sale line into its group key, reusing one text buffer.
struct SaleKey {
    dimension: SalesDimension,
    offset_seconds: i64,
    buffer: String,
    /// Rows come in date order, so the weekday of the last date is kept.
    last_date: String,
}

impl SaleKey {
    fn new(dimension: SalesDimension, offset_minutes: i32) -> SaleKey {
        SaleKey {
            dimension,
            offset_seconds: i64::from(offset_minutes) * 60,
            buffer: String::new(),
            last_date: String::new(),
        }
    }

    fn of(&mut self, row: &rusqlite::Row) -> rusqlite::Result<&str> {
        match self.dimension {
            SalesDimension::Hour => {
                let completed_at: i64 = row.get(0)?;
                let hour = local_hour(completed_at, self.offset_seconds);
                self.buffer.clear();
                let _ = write!(self.buffer, "{hour:02}");
            }
            SalesDimension::Weekday => {
                let date = row.get_ref(0)?.as_str()?;
                if date != self.last_date {
                    self.last_date.clear();
                    self.last_date.push_str(date);
                    let day = BusinessDate::parse(date)
                        .map_or(0, |date| date.0.weekday().num_days_from_sunday());
                    self.buffer.clear();
                    let _ = write!(self.buffer, "{day}");
                }
            }
            _ => {
                self.buffer.clear();
                self.buffer.push_str(row.get_ref(0)?.as_str()?);
            }
        }
        Ok(&self.buffer)
    }
}

/// The local hour (0–23) of a moment, `offset_seconds` from UTC.
fn local_hour(at_ms: i64, offset_seconds: i64) -> i64 {
    (at_ms / 1000 + offset_seconds).rem_euclid(86_400) / 3_600
}

/// Adds lines into groups and counts each sale once per group.
#[derive(Default)]
struct GroupAdder {
    groups: Vec<Slot>,
    group_index: HashMap<String, usize>,
    /// The sale id of the line being added, and how many sales have been seen
    /// (1 for the first sale; 0 means none yet).
    current_sale: String,
    sales_seen: u64,
}

/// One group, and the number of the last sale counted in it. Kept together, so adding
/// a line reads one place in memory (it matters with 50,000 products).
#[derive(Default)]
struct Slot {
    totals: GroupTotals,
    last_sale_counted: u64,
}

impl GroupAdder {
    /// Notes the sale of the next line. Lines of one sale arrive one after another.
    fn next_line_of(&mut self, sale: &str) {
        if self.sales_seen == 0 || self.current_sale != sale {
            self.current_sale.clear();
            self.current_sale.push_str(sale);
            self.sales_seen += 1;
        }
    }

    /// The group for `key`, made the first time it is seen.
    fn group(&mut self, key: &str) -> &mut Slot {
        let index = match self.group_index.get(key) {
            Some(index) => *index,
            None => {
                let index = self.groups.len();
                let mut slot = Slot::default();
                slot.totals.key = key.to_string();
                self.groups.push(slot);
                self.group_index.insert(key.to_string(), index);
                index
            }
        };
        &mut self.groups[index]
    }

    /// The group for `key`, with the current sale counted once in it, for adding a line.
    fn line_group(&mut self, key: &str) -> &mut GroupTotals {
        let sales_seen = self.sales_seen;
        let slot = self.group(key);
        if slot.last_sale_counted != sales_seen {
            slot.last_sale_counted = sales_seen;
            slot.totals.sales += 1;
        }
        &mut slot.totals
    }
}

/// `a + b`, as an error on overflow (inside the row reader, which returns database errors).
fn checked(a: i64, b: i64) -> rusqlite::Result<i64> {
    a.checked_add(b)
        .ok_or(rusqlite::Error::IntegralValueOutOfRange(0, b))
}

/// Subtracts a group's returns from its sales.
fn take_off(group: &mut GroupTotals, returned: &GroupTotals) -> Result<(), AppError> {
    group.qty = sub(group.qty, returned.qty)?;
    group.net = group.net.checked_sub(returned.net)?;
    group.tax = group.tax.checked_sub(returned.tax)?;
    group.cost = group.cost.checked_sub(returned.cost)?;
    Ok(())
}

/// The totals of all groups, with the number of sales counted overall.
pub(crate) fn overall(totals: &LineTotals) -> Result<GroupTotals, ReportsError> {
    let mut total = GroupTotals::default();
    for group in &totals.groups {
        total.qty = add(total.qty, group.qty)?;
        total.net = total.net.checked_add(group.net)?;
        total.tax = total.tax.checked_add(group.tax)?;
        total.discount = total.discount.checked_add(group.discount)?;
        total.cost = total.cost.checked_add(group.cost)?;
    }
    total.sales = totals.sales;
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sale_counts_once_per_group_however_many_lines_it_has() {
        let mut adder = GroupAdder::default();
        for (sale, group) in [
            ("S1", "milk"),
            ("S1", "milk"),
            ("S1", "rice"),
            ("S2", "milk"),
        ] {
            adder.next_line_of(sale);
            adder.line_group(group);
        }
        assert_eq!(adder.group("milk").totals.sales, 2);
        assert_eq!(adder.group("rice").totals.sales, 1);
        assert_eq!(adder.sales_seen, 2);
    }

    #[test]
    fn local_hour_wraps_around_midnight() {
        // 2026-03-05 03:00 UTC is 09:00 in Dhaka, 22:00 the day before in New York.
        assert_eq!(local_hour(1_772_679_600_000, 6 * 3_600), 9);
        assert_eq!(local_hour(1_772_679_600_000, -5 * 3_600), 22);
    }
}
