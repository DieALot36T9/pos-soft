//! Supplier and customer statements (RPT-07): the opening balance, every ledger row in
//! the period with the balance after it, and the closing balance.
//!
//! Same rules as `purchasing::statement` and `customers::statement`: rows are sorted by
//! business date, then time; the opening balance is the sum of every row before `from`.

use pos_core::{BusinessDate, Cell, ColumnKind, Id, Money, ReportTable};
use rusqlite::Connection;

use crate::sql::{Param, query, text as text_param};
use crate::table::{column, filter, new_table, optional_text, range_params, text};
use crate::types::kind_name;
use crate::{DateRange, ReportsError};

/// Where one kind of statement reads its rows.
struct Ledger {
    title: &'static str,
    entity: &'static str,
    name_sql: &'static str,
    rows_sql: &'static str,
}

const SUPPLIERS: Ledger = Ledger {
    title: "Supplier statement",
    entity: "supplier",
    name_sql: "SELECT name FROM purchasing_v_payables WHERE supplier_id = :owner",
    rows_sql: "SELECT business_date, kind, reference, amount_minor FROM purchasing_v_ledger
               WHERE supplier_id = :owner AND business_date <= :to ORDER BY business_date, at, id",
};

const CUSTOMERS: Ledger = Ledger {
    title: "Customer statement",
    entity: "customer",
    name_sql: "SELECT name FROM customers_v_customers WHERE id = :owner",
    rows_sql: "SELECT business_date, kind, reference, amount_minor FROM customers_v_credit_ledger
               WHERE customer_id = :owner AND business_date <= :to ORDER BY business_date, at, id",
};

/// A supplier's statement for a date range: + is what the shop owes more, − is paid.
pub fn supplier_statement(
    conn: &Connection,
    supplier: &Id,
    range: DateRange,
) -> Result<ReportTable, ReportsError> {
    statement(conn, &SUPPLIERS, supplier, range)
}

/// A customer's credit statement for a date range: + is what the customer owes more,
/// − is paid back (or store credit given).
pub fn customer_statement(
    conn: &Connection,
    customer: &Id,
    range: DateRange,
) -> Result<ReportTable, ReportsError> {
    statement(conn, &CUSTOMERS, customer, range)
}

fn statement(
    conn: &Connection,
    ledger: &Ledger,
    owner: &Id,
    range: DateRange,
) -> Result<ReportTable, ReportsError> {
    let range = range.check()?;
    let params: [Param; 2] = [
        (":owner", text_param(owner.as_str())),
        (":to", text_param(&range.to.to_string())),
    ];
    let name: String = query(conn, ledger.name_sql, &params, |row| row.get(0))?
        .into_iter()
        .next()
        .ok_or_else(|| ReportsError::NotFound {
            entity: ledger.entity,
            id: owner.clone(),
        })?;
    let rows: Vec<(String, String, Option<String>, i64)> =
        query(conn, ledger.rows_sql, &params, |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;

    let mut params = range_params(range);
    params
        .filters
        .push(filter(&kind_name(ledger.entity), &name));
    let mut table = new_table(
        ledger.title,
        params,
        vec![
            column("date", "Date", ColumnKind::Date),
            column("entry", "Entry", ColumnKind::Text),
            column("reference", "Reference", ColumnKind::Text),
            column("amount", "Amount", ColumnKind::Money),
            column("balance", "Balance", ColumnKind::Money),
        ],
    );
    let mut balance = Money::zero();
    let mut lines = Vec::new();
    for (date_text, kind, reference, amount) in rows {
        let date = BusinessDate::parse(&date_text)?;
        balance = balance.checked_add(Money(amount))?;
        if date >= range.from {
            lines.push(vec![
                Cell::Date(date),
                text(&entry_name(&kind)),
                optional_text(reference),
                Cell::Money(Money(amount)),
                Cell::Money(balance),
            ]);
        }
    }
    // The balance before the first row in the period is the opening balance.
    let mut opening = balance;
    for line in &lines {
        if let Some(Cell::Money(amount)) = line.get(3) {
            opening = opening.checked_sub(*amount)?;
        }
    }
    table.rows.push(vec![
        Cell::Date(range.from),
        text("Opening balance"),
        Cell::Empty,
        Cell::Empty,
        Cell::Money(opening),
    ]);
    table.rows.extend(lines);
    table.totals = Some(vec![
        Cell::Date(range.to),
        text("Closing balance"),
        Cell::Empty,
        Cell::Empty,
        Cell::Money(balance),
    ]);
    Ok(table)
}

/// A ledger kind in plain words. Example: "return_credit" → "Return".
fn entry_name(kind: &str) -> String {
    match kind {
        "return_credit" => "Return".to_string(),
        "adjust" => "Adjustment".to_string(),
        _ => kind_name(kind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_kinds_read_as_plain_words() {
        assert_eq!(entry_name("credit_sale"), "Credit sale");
        assert_eq!(entry_name("return_credit"), "Return");
        assert_eq!(entry_name("adjust"), "Adjustment");
        assert_eq!(entry_name("store_credit_used"), "Store credit used");
    }
}
