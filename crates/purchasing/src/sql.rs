//! Small helpers shared by the SQL code in this module: reading ids and dates
//! from rows, document numbers, and checks that every document needs.

use chrono::NaiveDate;
use pos_core::{Id, Money, Qty, Timestamp, mul_qty};
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};

use crate::PurchasingError;

/// Reads an id column. A bad id in the database is a bug, so it becomes a database error.
pub(crate) fn read_id(row: &Row, column: &str) -> rusqlite::Result<Id> {
    let text: String = row.get(column)?;
    parse_id(column, &text)
}

/// Reads an id column that may be empty.
pub(crate) fn read_optional_id(row: &Row, column: &str) -> rusqlite::Result<Option<Id>> {
    let text: Option<String> = row.get(column)?;
    match text {
        Some(text) => parse_id(column, &text).map(Some),
        None => Ok(None),
    }
}

/// Reads a "YYYY-MM-DD" column.
pub(crate) fn read_date(row: &Row, column: &str) -> rusqlite::Result<NaiveDate> {
    let text: String = row.get(column)?;
    parse_date(column, &text)
}

/// Reads a "YYYY-MM-DD" column that may be empty.
pub(crate) fn read_optional_date(row: &Row, column: &str) -> rusqlite::Result<Option<NaiveDate>> {
    let text: Option<String> = row.get(column)?;
    match text {
        Some(text) => parse_date(column, &text).map(Some),
        None => Ok(None),
    }
}

/// Reads an optional timestamp column.
pub(crate) fn read_optional_timestamp(
    row: &Row,
    column: &str,
) -> rusqlite::Result<Option<Timestamp>> {
    let millis: Option<i64> = row.get(column)?;
    Ok(millis.map(Timestamp))
}

/// A date as stored in the database: "YYYY-MM-DD".
pub(crate) fn date_text(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn parse_id(column: &str, text: &str) -> rusqlite::Result<Id> {
    Id::parse(text).map_err(|error| bad_column(column, error))
}

fn parse_date(column: &str, text: &str) -> rusqlite::Result<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|error| bad_column(column, error))
}

fn bad_column(
    column: &str,
    error: impl std::error::Error + Send + Sync + 'static,
) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, Type::Text, format!("{column}: {error}").into())
}

/// The next gapless number for a document: "PO-000001", "GRN-000001", "SR-000001".
/// The counter goes up inside the caller's transaction, so a rollback gives the number back.
pub(crate) fn next_number(
    tx: &Transaction,
    doc: &str,
    prefix: &str,
) -> Result<String, PurchasingError> {
    tx.execute(
        "INSERT INTO purchasing_sequences (doc, next) VALUES (:doc, 1) ON CONFLICT (doc) DO NOTHING",
        named_params! { ":doc": doc },
    )?;
    let number: i64 = tx.query_row(
        "UPDATE purchasing_sequences SET next = next + 1 WHERE doc = :doc RETURNING next - 1",
        named_params! { ":doc": doc },
        |row| row.get(0),
    )?;
    Ok(format!("{prefix}-{number:06}"))
}

/// Facts about a supplier that documents need to check.
pub(crate) struct SupplierFacts {
    pub payment_terms_days: u32,
    pub archived: bool,
}

/// Looks up a supplier, or says it wasn't found.
pub(crate) fn supplier_facts(
    connection: &Connection,
    supplier: &Id,
) -> Result<SupplierFacts, PurchasingError> {
    connection
        .query_row(
            "SELECT payment_terms_days, archived_at FROM purchasing_suppliers WHERE id = :id",
            named_params! { ":id": supplier.as_str() },
            |row| {
                let archived_at: Option<i64> = row.get("archived_at")?;
                Ok(SupplierFacts {
                    payment_terms_days: row.get("payment_terms_days")?,
                    archived: archived_at.is_some(),
                })
            },
        )
        .optional()?
        .ok_or_else(|| PurchasingError::not_found("supplier", supplier))
}

/// Refuses a new document for a supplier that doesn't exist or is archived.
pub(crate) fn require_active_supplier(
    connection: &Connection,
    supplier: &Id,
) -> Result<SupplierFacts, PurchasingError> {
    let facts = supplier_facts(connection, supplier)?;
    if facts.archived {
        return Err(PurchasingError::Conflict {
            code: "supplier_archived",
        });
    }
    Ok(facts)
}

/// Checks one document line: quantity above zero and cost not below zero.
pub(crate) fn check_line(qty: Qty, unit_cost: Money) -> Result<(), PurchasingError> {
    if qty.0 <= 0 {
        return Err(PurchasingError::Invalid {
            field: "qty",
            code: "qty_not_positive",
        });
    }
    if unit_cost.0 < 0 {
        return Err(PurchasingError::Invalid {
            field: "unit_cost",
            code: "cost_negative",
        });
    }
    Ok(())
}

/// Refuses a document with no lines.
pub(crate) fn require_lines(count: usize) -> Result<(), PurchasingError> {
    if count == 0 {
        return Err(PurchasingError::Invalid {
            field: "lines",
            code: "lines_missing",
        });
    }
    Ok(())
}

/// The value of a list of (quantity, unit cost) lines: the sum of round(cost × qty).
pub(crate) fn lines_value(lines: &[(Qty, Money)]) -> Result<Money, PurchasingError> {
    let mut total = Money::zero();
    for (qty, unit_cost) in lines {
        total = total.checked_add(mul_qty(*unit_cost, *qty)?)?;
    }
    Ok(total)
}

/// Text with spaces trimmed, or `None` when nothing is left.
pub(crate) fn clean_text(text: Option<&str>) -> Option<String> {
    let trimmed = text?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
