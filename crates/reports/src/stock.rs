//! Stock reports (RPT-05): stock value, low stock, near expiry and stock movement.

use std::collections::BTreeMap;

use chrono::Days;
use pos_core::{BusinessDate, Cell, ColumnKind, Money, Qty, ReportParams, ReportTable, mul_qty};
use rusqlite::Connection;

use crate::sql::{query, text as text_param};
use crate::table::{column, new_table, optional_text, text};
use crate::{GroupBy, ReportsError};

/// One product's stock and value.
struct Stocked {
    name: String,
    group_key: String,
    group_name: String,
    on_hand: Qty,
    cost: Money,
    value: Money,
}

/// Stock value (INV-08): on-hand × the product's cost price, by product, category or brand.
/// Products with no stock are left out. Largest value first.
pub fn stock_value(conn: &Connection, group_by: GroupBy) -> Result<ReportTable, ReportsError> {
    let products = query(
        conn,
        "SELECT p.name, p.category_id, p.category_name, COALESCE(p.brand_id, ''), p.brand_name,
                l.on_hand_milli, p.cost_price_minor
         FROM inventory_v_levels l JOIN catalog_v_products p ON p.id = l.product_id
         WHERE l.on_hand_milli <> 0",
        &[],
        |row| {
            let (group_key, group_name): (String, Option<String>) = match group_by {
                GroupBy::Product => (row.get(0)?, row.get(0)?),
                GroupBy::Category => (row.get(1)?, row.get(2)?),
                GroupBy::Brand => (row.get(3)?, row.get(4)?),
            };
            Ok(Stocked {
                name: row.get(0)?,
                group_key,
                group_name: group_name.unwrap_or_else(|| "No brand".to_string()),
                on_hand: Qty(row.get(5)?),
                cost: Money(row.get(6)?),
                value: Money::zero(),
            })
        },
    )?;
    let mut valued = Vec::with_capacity(products.len());
    for mut product in products {
        product.value = mul_qty(product.cost, product.on_hand)?;
        valued.push(product);
    }
    let mut total = Money::zero();
    for product in &valued {
        total = total.checked_add(product.value)?;
    }
    let mut table = match group_by {
        GroupBy::Product => product_values(valued),
        GroupBy::Category => group_values("Category", valued)?,
        GroupBy::Brand => group_values("Brand", valued)?,
    };
    let mut totals = vec![text("Total")];
    totals.resize(table.columns.len() - 1, Cell::Empty);
    totals.push(Cell::Money(total));
    table.totals = Some(totals);
    Ok(table)
}

fn product_values(mut products: Vec<Stocked>) -> ReportTable {
    products.sort_by(|a, b| b.value.0.cmp(&a.value.0).then(a.name.cmp(&b.name)));
    let mut table = new_table(
        "Stock value by product",
        ReportParams::default(),
        vec![
            column("product", "Product", ColumnKind::Text),
            column("on_hand", "On hand", ColumnKind::Qty),
            column("cost", "Cost", ColumnKind::Money),
            column("value", "Value", ColumnKind::Money),
        ],
    );
    for product in products {
        table.rows.push(vec![
            text(&product.name),
            Cell::Qty(product.on_hand),
            Cell::Money(product.cost),
            Cell::Money(product.value),
        ]);
    }
    table
}

fn group_values(title: &str, products: Vec<Stocked>) -> Result<ReportTable, ReportsError> {
    // key → (name, number of products, value)
    let mut groups: BTreeMap<String, (String, i64, Money)> = BTreeMap::new();
    for product in products {
        let group =
            groups
                .entry(product.group_key)
                .or_insert((product.group_name, 0, Money::zero()));
        group.1 += 1;
        group.2 = group.2.checked_add(product.value)?;
    }
    let mut list: Vec<(String, i64, Money)> = groups.into_values().collect();
    list.sort_by(|a, b| b.2.0.cmp(&a.2.0).then(a.0.cmp(&b.0)));
    let mut table = new_table(
        &format!("Stock value by {}", title.to_lowercase()),
        ReportParams::default(),
        vec![
            column("group", title, ColumnKind::Text),
            column("products", "Products", ColumnKind::Int),
            column("value", "Value", ColumnKind::Money),
        ],
    );
    for (name, count, value) in list {
        table
            .rows
            .push(vec![text(&name), Cell::Int(count), Cell::Money(value)]);
    }
    Ok(table)
}

/// Low stock (INV-03): products below their reorder level, with the supplier who last
/// delivered them and a suggested order. The biggest shortage comes first.
pub fn low_stock(conn: &Connection) -> Result<ReportTable, ReportsError> {
    let mut table = new_table(
        "Low stock",
        ReportParams::default(),
        vec![
            column("product", "Product", ColumnKind::Text),
            column("sku", "SKU", ColumnKind::Text),
            column("on_hand", "On hand", ColumnKind::Qty),
            column("reorder_level", "Reorder level", ColumnKind::Qty),
            column("suggested", "Suggested order", ColumnKind::Qty),
            column("supplier", "Supplier", ColumnKind::Text),
        ],
    );
    table.rows = query(
        conn,
        "SELECT p.name, p.sku, l.on_hand_milli, l.reorder_level_milli, l.reorder_qty_milli,
           (SELECT s.name FROM purchasing_v_grn_lines g
              JOIN purchasing_v_payables s ON s.supplier_id = g.supplier_id
            WHERE g.product_id = l.product_id ORDER BY g.received_at DESC LIMIT 1)
         FROM inventory_v_levels l JOIN catalog_v_products p ON p.id = l.product_id
         WHERE l.reorder_level_milli IS NOT NULL AND l.on_hand_milli < l.reorder_level_milli
           AND p.archived_at IS NULL
         ORDER BY l.reorder_level_milli - l.on_hand_milli DESC, p.name",
        &[],
        |row| {
            let on_hand = Qty(row.get(2)?);
            let level = Qty(row.get(3)?);
            Ok(vec![
                Cell::Text(row.get(0)?),
                Cell::Text(row.get(1)?),
                Cell::Qty(on_hand),
                Cell::Qty(level),
                Cell::Qty(suggested_order(on_hand, level, Qty(row.get(4)?))),
                optional_text(row.get(5)?),
            ])
        },
    )?;
    Ok(table)
}

/// How much to order: the reorder quantity, or, when the product has none, enough to
/// reach the reorder level again. Example: level 10, on hand 5.5, no quantity → 4.5.
pub(crate) fn suggested_order(on_hand: Qty, level: Qty, reorder_qty: Qty) -> Qty {
    if reorder_qty.0 > 0 {
        return reorder_qty;
    }
    Qty(level.0.saturating_sub(on_hand.0).max(0))
}

/// Batches that expire within `days` of `today`, and batches already past their date
/// (INV-07). Soonest first. Value = batch cost × quantity left in the batch.
pub fn near_expiry(
    conn: &Connection,
    today: BusinessDate,
    days: u32,
) -> Result<ReportTable, ReportsError> {
    let last_day = today
        .0
        .checked_add_days(Days::new(u64::from(days)))
        .map_or(today, BusinessDate);
    let params = ReportParams {
        from: Some(today),
        to: Some(last_day),
        filters: Vec::new(),
    };
    let mut table = new_table(
        "Near expiry",
        params,
        vec![
            column("product", "Product", ColumnKind::Text),
            column("batch", "Batch", ColumnKind::Text),
            column("expiry_date", "Expiry date", ColumnKind::Date),
            column("days_left", "Days left", ColumnKind::Int),
            column("on_hand", "On hand", ColumnKind::Qty),
            column("value", "Value", ColumnKind::Money),
            column("status", "Status", ColumnKind::Text),
        ],
    );
    let batches = query(
        conn,
        "SELECT p.name, b.batch_no, b.expiry_date, b.on_hand_milli, b.unit_cost_minor
         FROM inventory_v_batches b JOIN catalog_v_products p ON p.id = b.product_id
         WHERE b.expiry_date IS NOT NULL AND b.on_hand_milli > 0 AND b.expiry_date <= :last_day
         ORDER BY b.expiry_date, p.name",
        &[(":last_day", text_param(&last_day.to_string()))],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                Qty(row.get(3)?),
                Money(row.get(4)?),
            ))
        },
    )?;
    let mut total = Money::zero();
    for (name, batch_no, expiry_text, on_hand, unit_cost) in batches {
        let expiry = BusinessDate::parse(&expiry_text)?;
        let days_left = (expiry.0 - today.0).num_days();
        let value = mul_qty(unit_cost, on_hand)?;
        total = total.checked_add(value)?;
        let status = if days_left < 0 {
            "Expired"
        } else {
            "Expires soon"
        };
        table.rows.push(vec![
            text(&name),
            optional_text(batch_no),
            Cell::Date(expiry),
            Cell::Int(days_left),
            Cell::Qty(on_hand),
            Cell::Money(value),
            text(status),
        ]);
    }
    let mut totals = vec![
        text("Total"),
        Cell::Empty,
        Cell::Empty,
        Cell::Empty,
        Cell::Empty,
    ];
    totals.push(Cell::Money(total));
    totals.push(Cell::Empty);
    table.totals = Some(totals);
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inv_03_suggested_order_uses_the_reorder_quantity_or_fills_up_to_the_level() {
        assert_eq!(
            suggested_order(Qty(20_000), Qty(25_000), Qty(30_000)),
            Qty(30_000)
        );
        assert_eq!(suggested_order(Qty(5_500), Qty(10_000), Qty(0)), Qty(4_500));
        // Below zero (ADR-020): order enough to cover what is owed too.
        assert_eq!(
            suggested_order(Qty(-2_000), Qty(10_000), Qty(0)),
            Qty(12_000)
        );
    }
}
