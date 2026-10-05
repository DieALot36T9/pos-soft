//! Sales analysis (RPT-03) and profit and margin (RPT-04).

use std::collections::BTreeMap;

use pos_core::{BusinessDate, Cell, ColumnKind, Qty, ReportFilter, ReportTable};
use rusqlite::Connection;

use crate::line_totals::{GroupTotals, group_totals, overall};
use crate::sql::query;
use crate::table::{column, filter, new_table, range_params, share, text};
use crate::{DateRange, GroupBy, ReportsError, SalesDimension, SalesFilters, ShopContext};

/// Sales by product, category, brand, cashier, hour, weekday or date for a date range:
/// quantity, net sales (without tax), tax, discount, number of sales and share of the
/// total net sales. Returns are taken off on their own business date. Rows are sorted
/// by net sales (largest first), or in time order for hour, weekday and date.
pub fn sales_by(
    conn: &Connection,
    dimension: SalesDimension,
    range: DateRange,
    filters: &SalesFilters,
    shop: &ShopContext,
) -> Result<ReportTable, ReportsError> {
    let range = range.check()?;
    let groups = group_totals(conn, dimension, range, filters, shop.utc_offset_minutes)?;
    let total = overall(conn, &groups, range, filters)?;
    let names = Names::load(conn, dimension, shop)?;

    let mut params = range_params(range);
    params.filters = filter_list(conn, filters, shop)?;
    let (key_title, key_kind) = key_column(dimension);
    let mut table = new_table(
        &format!("Sales by {}", key_title.to_lowercase()),
        params,
        vec![
            column("key", key_title, key_kind),
            column("quantity", "Quantity", ColumnKind::Qty),
            column("net_sales", "Net sales", ColumnKind::Money),
            column("tax", "Tax", ColumnKind::Money),
            column("discount", "Discount", ColumnKind::Money),
            column("sales", "Number of sales", ColumnKind::Int),
            column("share", "Share of total", ColumnKind::Percent),
        ],
    );
    for group in sorted(dimension, groups) {
        table.rows.push(vec![
            names.cell(dimension, &group),
            Cell::Qty(Qty(group.qty)),
            Cell::Money(group.net),
            Cell::Money(group.tax),
            Cell::Money(group.discount),
            Cell::Int(group.sales),
            share(group.net, total.net)?,
        ]);
    }
    if let Some(top) = filters.top {
        table.rows.truncate(top as usize);
    }
    // A date column can only hold dates, so its totals row has no "Total" label.
    let total_label = match key_kind {
        ColumnKind::Date => Cell::Empty,
        _ => text("Total"),
    };
    table.totals = Some(vec![
        total_label,
        Cell::Qty(Qty(total.qty)),
        Cell::Money(total.net),
        Cell::Money(total.tax),
        Cell::Money(total.discount),
        Cell::Int(total.sales),
        share(total.net, total.net)?,
    ]);
    Ok(table)
}

/// Profit and margin by product, category or brand (RPT-04), using the cost saved on
/// each sale line. Returns take off their own saved cost. Margin = profit ÷ net sales;
/// it is empty when net sales are 0 or less, or the profit is negative.
pub fn profit(
    conn: &Connection,
    group_by: GroupBy,
    range: DateRange,
) -> Result<ReportTable, ReportsError> {
    let range = range.check()?;
    let dimension = SalesDimension::from(group_by);
    let filters = SalesFilters::default();
    let shop = ShopContext::default();
    let groups = group_totals(conn, dimension, range, &filters, 0)?;
    let total = overall(conn, &groups, range, &filters)?;
    let names = Names::load(conn, dimension, &shop)?;
    let (key_title, key_kind) = key_column(dimension);
    let mut table = new_table(
        &format!("Profit by {}", key_title.to_lowercase()),
        range_params(range),
        vec![
            column("key", key_title, key_kind),
            column("net_sales", "Net sales", ColumnKind::Money),
            column("cost", "Cost", ColumnKind::Money),
            column("gross_profit", "Gross profit", ColumnKind::Money),
            column("margin", "Margin", ColumnKind::Percent),
        ],
    );
    for group in sorted(dimension, groups) {
        table
            .rows
            .push(profit_row(names.cell(dimension, &group), &group)?);
    }
    table.totals = Some(profit_row(text("Total"), &total)?);
    Ok(table)
}

fn profit_row(first: Cell, group: &GroupTotals) -> Result<Vec<Cell>, ReportsError> {
    let profit = group.net.checked_sub(group.cost)?;
    Ok(vec![
        first,
        Cell::Money(group.net),
        Cell::Money(group.cost),
        Cell::Money(profit),
        share(profit, group.net)?,
    ])
}

fn key_column(dimension: SalesDimension) -> (&'static str, ColumnKind) {
    match dimension {
        SalesDimension::Product => ("Product", ColumnKind::Text),
        SalesDimension::Category => ("Category", ColumnKind::Text),
        SalesDimension::Brand => ("Brand", ColumnKind::Text),
        SalesDimension::Cashier => ("Cashier", ColumnKind::Text),
        SalesDimension::Hour => ("Hour", ColumnKind::Text),
        SalesDimension::Weekday => ("Weekday", ColumnKind::Text),
        SalesDimension::Date => ("Date", ColumnKind::Date),
    }
}

/// Largest net sales first (then by key), or time order for hour, weekday and date.
fn sorted(dimension: SalesDimension, mut groups: Vec<GroupTotals>) -> Vec<GroupTotals> {
    match dimension {
        SalesDimension::Hour | SalesDimension::Date => groups.sort_by(|a, b| a.key.cmp(&b.key)),
        SalesDimension::Weekday => groups.sort_by_key(|group| weekday_position(&group.key)),
        _ => groups.sort_by(|a, b| b.net.0.cmp(&a.net.0).then(a.key.cmp(&b.key))),
    }
    groups
}

const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// SQLite numbers weekdays from Sunday = 0. The week starts on Monday here.
fn weekday_position(key: &str) -> usize {
    let number: usize = key.parse().unwrap_or(0);
    (number + 6) % 7
}

/// The names behind the group keys: product, category and brand names from the
/// catalog views, cashier names from `app`.
struct Names<'a> {
    catalog: BTreeMap<String, String>,
    shop: &'a ShopContext,
}

impl<'a> Names<'a> {
    fn load(
        conn: &Connection,
        dimension: SalesDimension,
        shop: &'a ShopContext,
    ) -> Result<Names<'a>, ReportsError> {
        let sql = match dimension {
            SalesDimension::Product => "SELECT id, name FROM catalog_v_products",
            SalesDimension::Category => "SELECT id, name FROM catalog_v_categories",
            SalesDimension::Brand => {
                "SELECT DISTINCT brand_id, brand_name FROM catalog_v_products WHERE brand_id IS NOT NULL"
            }
            _ => "SELECT '', '' WHERE 0",
        };
        let catalog = query(conn, sql, &[], |row| Ok((row.get(0)?, row.get(1)?)))?
            .into_iter()
            .collect();
        Ok(Names { catalog, shop })
    }

    fn cell(&self, dimension: SalesDimension, group: &GroupTotals) -> Cell {
        let key = group.key.as_str();
        match dimension {
            SalesDimension::Product | SalesDimension::Category => text(
                self.catalog
                    .get(key)
                    .map_or(group.line_name.as_str(), String::as_str),
            ),
            SalesDimension::Brand if key.is_empty() => text("No brand"),
            SalesDimension::Brand => text(self.catalog.get(key).map_or(key, String::as_str)),
            SalesDimension::Cashier => text(&self.shop.user_name(key)),
            SalesDimension::Hour => text(&format!("{key}:00")),
            SalesDimension::Weekday => {
                let number: usize = key.parse().unwrap_or(0);
                text(WEEKDAYS.get(number).copied().unwrap_or(key))
            }
            SalesDimension::Date => BusinessDate::parse(key).map_or_else(|_| text(key), Cell::Date),
        }
    }
}

/// The filters in plain words, for the report header. Example: "Cashier: Rina".
fn filter_list(
    conn: &Connection,
    filters: &SalesFilters,
    shop: &ShopContext,
) -> Result<Vec<ReportFilter>, ReportsError> {
    let mut list = Vec::new();
    if let Some(category) = &filters.category_id {
        let sql = "SELECT name FROM catalog_v_categories WHERE id = :id";
        list.push(filter(
            "Category",
            &catalog_name(conn, sql, category.as_str())?,
        ));
    }
    if let Some(brand) = &filters.brand_id {
        let sql = "SELECT brand_name FROM catalog_v_products WHERE brand_id = :id LIMIT 1";
        list.push(filter("Brand", &catalog_name(conn, sql, brand.as_str())?));
    }
    if let Some(cashier) = &filters.cashier_id {
        list.push(filter("Cashier", &shop.user_name(cashier.as_str())));
    }
    if let Some(top) = filters.top {
        list.push(filter("Top", &top.to_string()));
    }
    Ok(list)
}

/// A name from a catalog view, or the id when the catalog doesn't have it.
fn catalog_name(conn: &Connection, sql: &str, id: &str) -> Result<String, ReportsError> {
    let found = query(conn, sql, &[(":id", crate::sql::text(id))], |row| {
        row.get(0)
    })?;
    Ok(found.into_iter().next().unwrap_or_else(|| id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_week_starts_on_monday() {
        assert_eq!(weekday_position("1"), 0);
        assert_eq!(weekday_position("0"), 6);
    }
}
