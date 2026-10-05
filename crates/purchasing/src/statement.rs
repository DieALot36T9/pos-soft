//! PUR-06 Supplier statement: the ledger for a date range, with opening and closing balances.

use chrono::NaiveDate;
use pos_core::{Id, Money, Timestamp};
use rusqlite::{Connection, named_params};

use crate::ledger::LedgerKind;
use crate::sql::{date_text, read_date, read_id, read_optional_date, read_optional_id};
use crate::{PurchasingError, get_supplier};

/// One ledger row on a statement, with the running balance after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementLine {
    pub id: Id,
    pub at: Timestamp,
    pub business_date: NaiveDate,
    pub kind: LedgerKind,
    /// Signed: positive means the shop owes more.
    pub amount: Money,
    pub method_kind: Option<String>,
    /// Supplier invoice number, cheque number, SR number or adjustment reason.
    pub reference: Option<String>,
    /// "grn" or "supplier_return", with `ref_id` the document's id.
    pub ref_type: Option<String>,
    pub ref_id: Option<Id>,
    pub due_date: Option<NaiveDate>,
    pub balance_after: Money,
}

/// A supplier statement from `from` to `to`, both days included (by business date).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub supplier_id: Id,
    pub supplier_name: String,
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// What the shop owed before `from`.
    pub opening: Money,
    pub lines: Vec<StatementLine>,
    /// What the shop owed at the end of `to`.
    pub closing: Money,
}

/// Builds a supplier's statement for a date range.
pub fn statement(
    connection: &Connection,
    supplier: &Id,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Statement, PurchasingError> {
    if to < from {
        return Err(PurchasingError::Invalid {
            field: "to",
            code: "date_range_backwards",
        });
    }
    let found = get_supplier(connection, supplier)?;
    let opening: i64 = connection.query_row(
        "SELECT COALESCE(SUM(amount_minor), 0) FROM purchasing_ledger
         WHERE supplier_id = :supplier AND business_date < :from",
        named_params! { ":supplier": supplier.as_str(), ":from": date_text(from) },
        |row| row.get(0),
    )?;
    let opening = Money(opening);

    let mut statement = connection.prepare(
        "SELECT * FROM purchasing_ledger
         WHERE supplier_id = :supplier AND business_date BETWEEN :from AND :to
         ORDER BY business_date, at, rowid",
    )?;
    let rows = statement.query_map(
        named_params! {
            ":supplier": supplier.as_str(),
            ":from": date_text(from),
            ":to": date_text(to),
        },
        |row| {
            let kind: String = row.get("kind")?;
            Ok(StatementLine {
                id: read_id(row, "id")?,
                at: Timestamp(row.get("at")?),
                business_date: read_date(row, "business_date")?,
                kind: LedgerKind::parse(&kind)?,
                amount: Money(row.get("amount_minor")?),
                method_kind: row.get("method_kind")?,
                reference: row.get("reference")?,
                ref_type: row.get("ref_type")?,
                ref_id: read_optional_id(row, "ref_id")?,
                due_date: read_optional_date(row, "due_date")?,
                balance_after: Money::zero(),
            })
        },
    )?;
    let mut running = opening;
    let mut lines = Vec::new();
    for line in rows {
        let mut line = line?;
        running = running.checked_add(line.amount)?;
        line.balance_after = running;
        lines.push(line);
    }
    Ok(Statement {
        supplier_id: found.id,
        supplier_name: found.name,
        from,
        to,
        opening,
        lines,
        closing: running,
    })
}
