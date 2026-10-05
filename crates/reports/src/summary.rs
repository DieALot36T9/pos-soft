//! The sales part shared by the daily summary (RPT-01) and the X/Z shift reports
//! (SHF-08): sales totals, tax by rate, payments by method and voided sales.
//! The two reports only differ in which sales they look at: the `Scope`.

use std::collections::BTreeMap;

use pos_core::{BusinessDate, Cell, ColumnKind, Id, Money, Qty, ReportTable, round_half_away};
use rusqlite::Connection;
use rusqlite::types::Value;

use crate::sql::{Param, optional_text, query, query_one, text as text_param};
use crate::table::{column, new_table, text};
use crate::tax::tax_by_rate_section;
use crate::tax_sums::rate_totals;
use crate::{ACTION_CART_CANCEL, ReportsError, ShopContext};

/// Which sales a summary covers.
pub(crate) enum Scope<'a> {
    /// One business date, on one till or all of them.
    Day {
        date: BusinessDate,
        till: Option<&'a Id>,
    },
    /// One shift. Cancelled carts are found by till and time, because carts have no shift
    /// in the audit log.
    Shift {
        shift: &'a Id,
        till: String,
        opened_at: i64,
        closed_at: Option<i64>,
    },
}

impl Scope<'_> {
    /// The query parameters. Each query binds only the ones it uses.
    fn params(&self) -> Vec<Param> {
        match self {
            Scope::Day { date, till } => vec![
                (":date", text_param(&date.to_string())),
                (":till", optional_text(till.map(Id::as_str))),
                (":cart_cancel", text_param(ACTION_CART_CANCEL)),
            ],
            Scope::Shift {
                shift,
                till,
                opened_at,
                closed_at,
            } => vec![
                (":shift", text_param(shift.as_str())),
                (":till", text_param(till)),
                (":opened", Value::Integer(*opened_at)),
                (":closed", closed_at.map_or(Value::Null, Value::Integer)),
                (":cart_cancel", text_param(ACTION_CART_CANCEL)),
            ],
        }
    }

    /// A condition on alias `s`: a sale, sale line or return line in scope.
    fn documents(&self) -> &'static str {
        match self {
            Scope::Day { .. } => "s.business_date = :date AND (:till IS NULL OR s.till_id = :till)",
            Scope::Shift { .. } => "s.shift_id = :shift",
        }
    }

    /// A condition on alias `p` (`sales_v_payments`, which has no till column).
    fn payments(&self) -> &'static str {
        match self {
            // The view works out its business date for every payment, so filtering on it
            // reads the whole table. Finding the day's documents first uses the indexes.
            Scope::Day { .. } => {
                "((p.doc_type = 'sale' AND p.doc_id IN
                    (SELECT s.id FROM sales_v_sales s
                     WHERE s.business_date = :date AND (:till IS NULL OR s.till_id = :till)))
                  OR (p.doc_type = 'return' AND p.doc_id IN
                    (SELECT s.return_id FROM sales_v_return_lines s
                     WHERE s.business_date = :date AND (:till IS NULL OR s.till_id = :till))))"
            }
            Scope::Shift { .. } => "p.shift_id = :shift",
        }
    }

    /// A condition on alias `e` (`audit_v_events`).
    fn events(&self) -> &'static str {
        match self {
            Scope::Day { .. } => "e.business_date = :date AND (:till IS NULL OR e.till_id = :till)",
            Scope::Shift { .. } => {
                "e.till_id = :till AND e.at >= :opened AND (:closed IS NULL OR e.at <= :closed)"
            }
        }
    }
}

/// Completed sales added up. Voided sales are not in here (they are listed apart).
struct SaleSums {
    count: i64,
    base: Money,
    promo: Money,
    tier: Money,
    manual: Money,
    gross: Money,
    tax: Money,
    rounding: Money,
    total: Money,
}

fn sale_sums(conn: &Connection, scope: &Scope, params: &[Param]) -> Result<SaleSums, ReportsError> {
    let sql = format!(
        "SELECT COUNT(*), COALESCE(SUM(s.base_minor), 0), COALESCE(SUM(s.promo_discount_minor), 0),
           COALESCE(SUM(s.tier_discount_minor), 0),
           COALESCE(SUM(s.manual_discount_minor + s.invoice_discount_minor), 0),
           COALESCE(SUM(s.gross_minor), 0), COALESCE(SUM(s.tax_minor), 0),
           COALESCE(SUM(s.rounding_minor), 0), COALESCE(SUM(s.total_minor), 0)
         FROM sales_v_sales s WHERE s.status = 'completed' AND {}",
        scope.documents()
    );
    query_one(conn, &sql, params, |row| {
        Ok(SaleSums {
            count: row.get(0)?,
            base: Money(row.get(1)?),
            promo: Money(row.get(2)?),
            tier: Money(row.get(3)?),
            manual: Money(row.get(4)?),
            gross: Money(row.get(5)?),
            tax: Money(row.get(6)?),
            rounding: Money(row.get(7)?),
            total: Money(row.get(8)?),
        })
    })
}

/// Returns added up: (number of returns, gross, tax, refunds paid).
fn return_sums(
    conn: &Connection,
    scope: &Scope,
    params: &[Param],
) -> Result<(i64, Money, Money, Money), ReportsError> {
    let lines_sql = format!(
        "SELECT COUNT(DISTINCT s.return_id), COALESCE(SUM(s.gross_minor), 0), COALESCE(SUM(s.tax_minor), 0)
         FROM sales_v_return_lines s WHERE {}",
        scope.documents()
    );
    let (count, gross, tax) = query_one(conn, &lines_sql, params, |row| {
        Ok((row.get(0)?, Money(row.get(1)?), Money(row.get(2)?)))
    })?;
    let refunds_sql = format!(
        "SELECT COALESCE(SUM(p.amount_minor), 0) FROM sales_v_payments p
         WHERE p.doc_type = 'return' AND {}",
        scope.payments()
    );
    let refunds = query_one(conn, &refunds_sql, params, |row| Ok(Money(row.get(0)?)))?;
    Ok((count, gross, tax, refunds))
}

/// The main table plus the "Tax by rate", "Payments by method" and "Voided sales"
/// sections, for the given scope.
pub(crate) fn sales_summary(
    conn: &Connection,
    title: &str,
    scope: &Scope,
    shop: &ShopContext,
) -> Result<ReportTable, ReportsError> {
    let params = scope.params();
    let sales = sale_sums(conn, scope, &params)?;
    let (return_count, return_gross, return_tax, refunds) = return_sums(conn, scope, &params)?;
    let items_sql = format!(
        "SELECT COALESCE(SUM(s.qty_milli), 0) FROM sales_v_lines s WHERE s.status = 'completed' AND {}",
        scope.documents()
    );
    let items = query_one(conn, &items_sql, &params, |row| Ok(Qty(row.get(0)?)))?;
    let voids = voided_sales(conn, scope, &params, shop)?;
    let carts_sql = format!(
        "SELECT COUNT(*) FROM audit_v_events e WHERE e.action = :cart_cancel AND {}",
        scope.events()
    );
    let cancelled_carts = query_one(conn, &carts_sql, &params, |row| row.get::<_, i64>(0))?;

    let mut table = new_table(
        title,
        Default::default(),
        vec![
            column("item", "Item", ColumnKind::Text),
            column("count", "Count", ColumnKind::Int),
            column("quantity", "Quantity", ColumnKind::Qty),
            column("amount", "Amount", ColumnKind::Money),
        ],
    );
    // A return's own rounding is what was paid back minus its line amounts.
    let return_rounding = refunds.checked_sub(return_gross)?;
    let average = if sales.count > 0 {
        Cell::Money(Money(round_half_away(
            i128::from(sales.total.0),
            i128::from(sales.count),
        )?))
    } else {
        Cell::Empty
    };
    let money_row = |label: &str, amount: Money| {
        vec![text(label), Cell::Empty, Cell::Empty, Cell::Money(amount)]
    };
    table.rows = vec![
        vec![
            text("Gross sales"),
            Cell::Int(sales.count),
            Cell::Empty,
            Cell::Money(sales.base),
        ],
        money_row("Promotion discounts", sales.promo),
        money_row("Customer discounts", sales.tier),
        money_row("Manual discounts", sales.manual),
        vec![
            text("Returns"),
            Cell::Int(return_count),
            Cell::Empty,
            Cell::Money(return_gross),
        ],
        money_row("Net sales", sales.gross.checked_sub(return_gross)?),
        money_row("Tax", sales.tax.checked_sub(return_tax)?),
        money_row("Rounding", sales.rounding.checked_sub(return_rounding)?),
        money_row("Total taken", sales.total.checked_sub(refunds)?),
        vec![
            text("Voided sales"),
            Cell::Int(voids.rows.len() as i64),
            Cell::Empty,
            Cell::Money(void_total(&voids)?),
        ],
        vec![
            text("Cancelled carts"),
            Cell::Int(cancelled_carts),
            Cell::Empty,
            Cell::Empty,
        ],
        vec![text("Average sale"), Cell::Empty, Cell::Empty, average],
        vec![
            text("Items sold"),
            Cell::Empty,
            Cell::Qty(items),
            Cell::Empty,
        ],
    ];
    let rates = rate_totals(conn, scope.documents(), &params)?;
    table.sections.push(tax_by_rate_section(&rates)?);
    table
        .sections
        .push(payments_by_method(conn, scope, &params, shop)?);
    table.sections.push(voids);
    Ok(table)
}

/// Sales minus refunds per payment method. Payments of voided sales are left out.
fn payments_by_method(
    conn: &Connection,
    scope: &Scope,
    params: &[Param],
    shop: &ShopContext,
) -> Result<ReportTable, ReportsError> {
    let sql = format!(
        "SELECT p.method_id, MAX(p.kind), p.doc_type, SUM(p.amount_minor) FROM sales_v_payments p
         WHERE p.doc_status = 'completed' AND {} GROUP BY p.method_id, p.doc_type",
        scope.payments()
    );
    // name → (sales, refunds)
    let mut methods: BTreeMap<String, (Money, Money)> = BTreeMap::new();
    for (method_id, kind, doc_type, amount) in query(conn, &sql, params, |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            Money(row.get(3)?),
        ))
    })? {
        let name = shop.payment_method_name(&method_id, &kind);
        let entry = methods
            .entry(name)
            .or_insert((Money::zero(), Money::zero()));
        if doc_type == "return" {
            entry.1 = entry.1.checked_add(amount)?;
        } else {
            entry.0 = entry.0.checked_add(amount)?;
        }
    }
    let mut table = new_table(
        "Payments by method",
        Default::default(),
        vec![
            column("method", "Method", ColumnKind::Text),
            column("sales", "Sales", ColumnKind::Money),
            column("refunds", "Refunds", ColumnKind::Money),
            column("net", "Net", ColumnKind::Money),
        ],
    );
    let mut sums = [Money::zero(); 3];
    for (name, (sales, refunds)) in methods {
        let net = sales.checked_sub(refunds)?;
        for (sum, amount) in sums.iter_mut().zip([sales, refunds, net]) {
            *sum = sum.checked_add(amount)?;
        }
        table.rows.push(vec![
            text(&name),
            Cell::Money(sales),
            Cell::Money(refunds),
            Cell::Money(net),
        ]);
    }
    let mut totals = vec![text("Total")];
    totals.extend(sums.map(Cell::Money));
    table.totals = Some(totals);
    Ok(table)
}

/// Voided sales, listed apart from the totals: invoice, cashier, reason and amount.
fn voided_sales(
    conn: &Connection,
    scope: &Scope,
    params: &[Param],
    shop: &ShopContext,
) -> Result<ReportTable, ReportsError> {
    let sql = format!(
        "SELECT s.invoice_no, s.user_id, s.void_reason, s.total_minor FROM sales_v_sales s
         WHERE s.status = 'voided' AND {} ORDER BY s.invoice_no",
        scope.documents()
    );
    let mut table = new_table(
        "Voided sales",
        Default::default(),
        vec![
            column("invoice", "Invoice", ColumnKind::Text),
            column("cashier", "Cashier", ColumnKind::Text),
            column("reason", "Reason", ColumnKind::Text),
            column("amount", "Amount", ColumnKind::Money),
        ],
    );
    table.rows = query(conn, &sql, params, |row| {
        let user: String = row.get(1)?;
        let reason: Option<String> = row.get(2)?;
        Ok(vec![
            Cell::Text(row.get(0)?),
            Cell::Text(shop.user_name(&user)),
            reason.map_or(Cell::Empty, Cell::Text),
            Cell::Money(Money(row.get(3)?)),
        ])
    })?;
    Ok(table)
}

fn void_total(voids: &ReportTable) -> Result<Money, ReportsError> {
    let mut total = Money::zero();
    for row in &voids.rows {
        if let Some(Cell::Money(amount)) = row.last() {
            total = total.checked_add(*amount)?;
        }
    }
    Ok(total)
}
