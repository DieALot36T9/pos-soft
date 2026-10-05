//! Discounts and overrides per cashier (RPT-08). Helps spot abuse.

use std::collections::BTreeMap;

use pos_core::{Cell, ColumnKind, Money, ReportTable};
use rusqlite::Connection;

use crate::sql::{query, text as text_param};
use crate::table::{column, new_table, range_params, text};
use crate::{
    ACTION_CART_CANCEL, ACTION_DRAWER_NO_SALE, ACTION_SALE_VOID, DateRange, ReportsError,
    ShopContext,
};

/// What one cashier did in the period.
#[derive(Default)]
struct CashierCounts {
    discounted_sales: i64,
    discount_amount: Money,
    price_overrides: i64,
    voids: i64,
    cancelled_carts: i64,
    no_sales: i64,
    overrides_approved: i64,
}

/// Per cashier: sales with a manual discount and the amount, price overrides, voids,
/// cancelled carts, no-sale drawer openings and overrides approved for them. A section
/// lists who approved each kind of override. Voided sales' lines are not counted as
/// discounts (the void itself is counted).
pub fn discounts_overrides(
    conn: &Connection,
    range: DateRange,
    shop: &ShopContext,
) -> Result<ReportTable, ReportsError> {
    let range = range.check()?;
    let params = [
        (":from", text_param(&range.from.to_string())),
        (":to", text_param(&range.to.to_string())),
        (":void", text_param(ACTION_SALE_VOID)),
        (":cancel", text_param(ACTION_CART_CANCEL)),
        (":no_sale", text_param(ACTION_DRAWER_NO_SALE)),
    ];
    let mut cashiers: BTreeMap<String, CashierCounts> = BTreeMap::new();
    for (user, discounted, amount, overrides) in query(
        conn,
        "SELECT user_id,
           COUNT(DISTINCT CASE WHEN manual_discount_minor + invoice_discount_minor > 0 THEN sale_id END),
           SUM(manual_discount_minor + invoice_discount_minor), SUM(price_overridden)
         FROM sales_v_lines
         WHERE status = 'completed' AND business_date BETWEEN :from AND :to
         GROUP BY user_id",
        &params,
        |row| Ok((row.get::<_, String>(0)?, row.get(1)?, Money(row.get(2)?), row.get(3)?)),
    )? {
        let counts = cashiers.entry(user).or_default();
        counts.discounted_sales = discounted;
        counts.discount_amount = amount;
        counts.price_overrides = overrides;
    }
    for (user, voids, cancelled, no_sales, approved) in query(
        conn,
        "SELECT user_id, SUM(action = :void), SUM(action = :cancel), SUM(action = :no_sale),
           SUM(approved_by IS NOT NULL)
         FROM audit_v_events
         WHERE user_id IS NOT NULL AND business_date BETWEEN :from AND :to
         GROUP BY user_id",
        &params,
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )? {
        let counts = cashiers.entry(user).or_default();
        counts.voids = voids;
        counts.cancelled_carts = cancelled;
        counts.no_sales = no_sales;
        counts.overrides_approved = approved;
    }

    let mut table = new_table(
        "Discounts and overrides",
        range_params(range),
        vec![
            column("cashier", "Cashier", ColumnKind::Text),
            column("discounted_sales", "Manual discounts", ColumnKind::Int),
            column("discount_amount", "Discount amount", ColumnKind::Money),
            column("price_overrides", "Price overrides", ColumnKind::Int),
            column("voids", "Voids", ColumnKind::Int),
            column("cancelled_carts", "Cancelled carts", ColumnKind::Int),
            column("no_sales", "No-sale openings", ColumnKind::Int),
            column("overrides_approved", "Overrides approved", ColumnKind::Int),
        ],
    );
    let mut rows: Vec<(String, CashierCounts)> = cashiers
        .into_iter()
        .map(|(user, counts)| (shop.user_name(&user), counts))
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, counts) in rows {
        table.rows.push(vec![
            text(&name),
            Cell::Int(counts.discounted_sales),
            Cell::Money(counts.discount_amount),
            Cell::Int(counts.price_overrides),
            Cell::Int(counts.voids),
            Cell::Int(counts.cancelled_carts),
            Cell::Int(counts.no_sales),
            Cell::Int(counts.overrides_approved),
        ]);
    }
    table.sections.push(approvals(conn, &params, shop)?);
    Ok(table)
}

/// Who approved which overrides for whom: cashier, action, approver and how many times.
fn approvals(
    conn: &Connection,
    params: &[crate::sql::Param],
    shop: &ShopContext,
) -> Result<ReportTable, ReportsError> {
    let found = query(
        conn,
        "SELECT user_id, action, approved_by, COUNT(*) FROM audit_v_events
         WHERE approved_by IS NOT NULL AND user_id IS NOT NULL AND business_date BETWEEN :from AND :to
         GROUP BY user_id, action, approved_by",
        params,
        |row| {
            let user: String = row.get(0)?;
            let approver: String = row.get(2)?;
            Ok((shop.user_name(&user), row.get::<_, String>(1)?, shop.user_name(&approver), row.get(3)?))
        },
    )?;
    let mut rows: Vec<(String, String, String, i64)> = found;
    rows.sort();
    let mut table = new_table(
        "Overrides approved",
        Default::default(),
        vec![
            column("cashier", "Cashier", ColumnKind::Text),
            column("action", "Action", ColumnKind::Text),
            column("approved_by", "Approved by", ColumnKind::Text),
            column("count", "Count", ColumnKind::Int),
        ],
    );
    for (cashier, action, approver, count) in rows {
        table.rows.push(vec![
            text(&cashier),
            text(&action),
            text(&approver),
            Cell::Int(count),
        ]);
    }
    Ok(table)
}
