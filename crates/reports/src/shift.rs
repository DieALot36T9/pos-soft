//! X and Z shift reports (RPT-02, SHF-05, SHF-08).

use chrono::DateTime;
use pos_core::{AppError, BusinessDate, Cell, ColumnKind, Id, Money, ReportTable};
use rusqlite::Connection;

use crate::sql::{query, query_one, text as text_param};
use crate::summary::{Scope, sales_summary};
use crate::table::{column, day_params, new_table, optional_text, text};
use crate::{ReportsError, ShopContext};

/// One shift as `shifts_v_shifts` shows it.
struct ShiftRow {
    till_id: String,
    user_id: String,
    business_date: String,
    open: bool,
    forced: bool,
    opened_at: i64,
    closed_at: Option<i64>,
    opening_float: Money,
    expected: Option<i64>,
    counted: Option<i64>,
    difference: Option<i64>,
}

/// The shift report: "Z report" once the shift is closed, "X report – shift still open"
/// before. Same content either way (SHF-05): shift details, sales, tax by rate,
/// payments by method, the cash summary and voided sales. Past Z reports can be
/// made again at any time, because everything is read from the stored rows.
pub fn shift_report(
    conn: &Connection,
    shift: &Id,
    shop: &ShopContext,
) -> Result<ReportTable, ReportsError> {
    let row = load_shift(conn, shift)?;
    let title = if row.open {
        "X report – shift still open"
    } else {
        "Z report"
    };
    let scope = Scope::Shift {
        shift,
        till: row.till_id.clone(),
        opened_at: row.opened_at,
        closed_at: row.closed_at,
    };
    let mut report = sales_summary(conn, title, &scope, shop)?;
    if let Ok(date) = BusinessDate::parse(&row.business_date) {
        report.params = day_params(date);
    }
    report.sections.insert(0, details_section(&row, shop));
    let voids = report.sections.pop();
    report.sections.push(cash_section(conn, shift, &row)?);
    report.sections.push(count_section(conn, shift)?);
    report.sections.extend(voids);
    Ok(report)
}

fn load_shift(conn: &Connection, shift: &Id) -> Result<ShiftRow, ReportsError> {
    let found = query(
        conn,
        "SELECT till_id, user_id, business_date, status, forced, opened_at, closed_at,
                opening_float_minor, expected_cash_minor, counted_cash_minor, difference_minor
         FROM shifts_v_shifts WHERE id = :shift",
        &[(":shift", text_param(shift.as_str()))],
        |row| {
            Ok(ShiftRow {
                till_id: row.get(0)?,
                user_id: row.get(1)?,
                business_date: row.get(2)?,
                open: row.get::<_, String>(3)? == "open",
                forced: row.get(4)?,
                opened_at: row.get(5)?,
                closed_at: row.get(6)?,
                opening_float: Money(row.get(7)?),
                expected: row.get(8)?,
                counted: row.get(9)?,
                difference: row.get(10)?,
            })
        },
    )?;
    found
        .into_iter()
        .next()
        .ok_or_else(|| ReportsError::NotFound {
            entity: "shift",
            id: shift.clone(),
        })
}

/// Shop details are printed by `printing` from settings; this part has till, cashier,
/// business date, opening and closing times, and the status.
fn details_section(row: &ShiftRow, shop: &ShopContext) -> ReportTable {
    let mut table = new_table(
        "Shift",
        Default::default(),
        vec![
            column("item", "Item", ColumnKind::Text),
            column("value", "Value", ColumnKind::Text),
        ],
    );
    let status = match (row.open, row.forced) {
        (true, _) => "Open",
        (false, true) => "Force closed",
        (false, false) => "Closed",
    };
    let offset = shop.utc_offset_minutes;
    table.rows = vec![
        vec![text("Till"), text(&shop.till_name(&row.till_id))],
        vec![text("Cashier"), text(&shop.user_name(&row.user_id))],
        vec![text("Business date"), text(&row.business_date)],
        vec![
            text("Opened"),
            optional_text(local_time(row.opened_at, offset)),
        ],
        vec![
            text("Closed"),
            optional_text(row.closed_at.and_then(|at| local_time(at, offset))),
        ],
        vec![text("Status"), text(status)],
    ];
    table
}

/// A moment written in local time. Example: "2026-03-05 09:00".
fn local_time(at_ms: i64, offset_minutes: i32) -> Option<String> {
    let local_ms = at_ms.checked_add(i64::from(offset_minutes) * 60_000)?;
    let moment = DateTime::from_timestamp_millis(local_ms)?;
    Some(moment.format("%Y-%m-%d %H:%M").to_string())
}

/// The cash in the drawer (SHF-06, SHF-08): float, cash moved by kind, expected,
/// counted and the difference. Amounts carry their sign: − means money out.
fn cash_section(
    conn: &Connection,
    shift: &Id,
    row: &ShiftRow,
) -> Result<ReportTable, ReportsError> {
    let kind_total = |kind: &str| -> Result<Money, ReportsError> {
        query_one(
            conn,
            "SELECT COALESCE(SUM(amount_minor), 0) FROM shifts_v_movements
             WHERE shift_id = :shift AND method_kind = 'cash' AND kind = :kind",
            &[
                (":shift", text_param(shift.as_str())),
                (":kind", text_param(kind)),
            ],
            |found| Ok(Money(found.get(0)?)),
        )
    };
    let lines = [
        ("Cash sales", "sale_payment"),
        ("Cash refunds", "refund"),
        ("Void reversals", "void_reversal"),
        ("Cash in", "cash_in"),
        ("Cash out", "cash_out"),
        ("Cash drops", "cash_drop"),
        ("Customer payments", "customer_payment"),
        ("Supplier payments", "supplier_payment"),
    ];
    let mut table = new_table(
        "Cash",
        Default::default(),
        vec![
            column("item", "Item", ColumnKind::Text),
            column("amount", "Amount", ColumnKind::Money),
        ],
    );
    table
        .rows
        .push(vec![text("Float"), Cell::Money(row.opening_float)]);
    for (label, kind) in lines {
        table
            .rows
            .push(vec![text(label), Cell::Money(kind_total(kind)?)]);
    }
    // A closed shift shows what was saved at closing. An open one adds up the cash so far.
    let expected = match row.expected {
        Some(saved) => Money(saved),
        None => query_one(
            conn,
            "SELECT COALESCE(SUM(amount_minor), 0) FROM shifts_v_movements
             WHERE shift_id = :shift AND method_kind = 'cash'",
            &[(":shift", text_param(shift.as_str()))],
            |found| Ok(Money(found.get(0)?)),
        )?,
    };
    let money_or_empty =
        |amount: Option<i64>| amount.map_or(Cell::Empty, |minor| Cell::Money(Money(minor)));
    table
        .rows
        .push(vec![text("Expected cash"), Cell::Money(expected)]);
    table
        .rows
        .push(vec![text("Counted cash"), money_or_empty(row.counted)]);
    table
        .rows
        .push(vec![text("Difference"), money_or_empty(row.difference)]);
    Ok(table)
}

/// The cash count by denomination (SHF-08): how many of each note or coin at opening
/// and at closing, and what the closing count is worth. Largest first. Empty when the
/// shift was opened and closed without counting.
fn count_section(conn: &Connection, shift: &Id) -> Result<ReportTable, ReportsError> {
    let counts = query(
        conn,
        "SELECT denomination_minor,
           SUM(CASE WHEN stage = 'open' THEN count ELSE 0 END),
           SUM(CASE WHEN stage = 'close' THEN count ELSE 0 END)
         FROM shifts_v_counts WHERE shift_id = :shift
         GROUP BY denomination_minor ORDER BY denomination_minor DESC",
        &[(":shift", text_param(shift.as_str()))],
        |row| {
            Ok((
                Money(row.get(0)?),
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;
    let mut table = new_table(
        "Count",
        Default::default(),
        vec![
            column("denomination", "Note or coin", ColumnKind::Money),
            column("opening_count", "At opening", ColumnKind::Int),
            column("closing_count", "At closing", ColumnKind::Int),
            column("closing_amount", "Closing amount", ColumnKind::Money),
        ],
    );
    let mut counted = Money::zero();
    for (denomination, opening, closing) in counts {
        let amount = denomination.0.checked_mul(closing).ok_or_else(|| {
            AppError::internal(format!("count: {} × {closing} overflowed", denomination.0))
        })?;
        counted = counted.checked_add(Money(amount))?;
        table.rows.push(vec![
            Cell::Money(denomination),
            Cell::Int(opening),
            Cell::Int(closing),
            Cell::Money(Money(amount)),
        ]);
    }
    // The first column holds money, so the totals row has no "Total" label.
    table.totals = Some(vec![
        Cell::Empty,
        Cell::Empty,
        Cell::Empty,
        Cell::Money(counted),
    ]);
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_time_uses_the_shop_offset() {
        // 2026-03-05 03:00 UTC is 09:00 in Dhaka.
        assert_eq!(
            local_time(1_772_679_600_000, 360).unwrap(),
            "2026-03-05 09:00"
        );
        assert_eq!(
            local_time(1_772_679_600_000, 0).unwrap(),
            "2026-03-05 03:00"
        );
    }
}
