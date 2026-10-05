//! Small helpers that read our value types out of SQLite rows.
//! Ids, dates and enum names are stored as text, and are checked as they're read back.

use pos_core::{AppError, Bps, BusinessDate, Id, Money, Qty, Timestamp};
use rusqlite::Row;
use rusqlite::types::Type;

/// Reads an id column.
pub(crate) fn get_id(row: &Row, column: &str) -> rusqlite::Result<Id> {
    let text: String = row.get(column)?;
    Id::parse(&text).map_err(|error| bad_value(row, column, error))
}

/// Reads an id column that may be empty (NULL).
pub(crate) fn get_optional_id(row: &Row, column: &str) -> rusqlite::Result<Option<Id>> {
    let text: Option<String> = row.get(column)?;
    match text {
        Some(text) => Id::parse(&text)
            .map(Some)
            .map_err(|error| bad_value(row, column, error)),
        None => Ok(None),
    }
}

/// Reads a `*_minor` column.
pub(crate) fn get_money(row: &Row, column: &str) -> rusqlite::Result<Money> {
    Ok(Money(row.get(column)?))
}

/// Reads a `*_minor` column that may be empty (NULL).
pub(crate) fn get_optional_money(row: &Row, column: &str) -> rusqlite::Result<Option<Money>> {
    let value: Option<i64> = row.get(column)?;
    Ok(value.map(Money))
}

/// Reads a `*_milli` column.
pub(crate) fn get_qty(row: &Row, column: &str) -> rusqlite::Result<Qty> {
    Ok(Qty(row.get(column)?))
}

/// Reads a `*_bps` column.
pub(crate) fn get_bps(row: &Row, column: &str) -> rusqlite::Result<Bps> {
    Ok(Bps(row.get(column)?))
}

/// Reads a `*_at` column.
pub(crate) fn get_timestamp(row: &Row, column: &str) -> rusqlite::Result<Timestamp> {
    Ok(Timestamp(row.get(column)?))
}

/// Reads a `*_at` column that may be empty (NULL).
pub(crate) fn get_optional_timestamp(
    row: &Row,
    column: &str,
) -> rusqlite::Result<Option<Timestamp>> {
    let value: Option<i64> = row.get(column)?;
    Ok(value.map(Timestamp))
}

/// Reads a 0/1 column.
pub(crate) fn get_bool(row: &Row, column: &str) -> rusqlite::Result<bool> {
    let value: i64 = row.get(column)?;
    Ok(value != 0)
}

/// Reads a `business_date` column ("YYYY-MM-DD").
pub(crate) fn get_business_date(row: &Row, column: &str) -> rusqlite::Result<BusinessDate> {
    let text: String = row.get(column)?;
    BusinessDate::parse(&text).map_err(|error| bad_value(row, column, error))
}

/// Reads an enum column with the enum's own `parse` function.
pub(crate) fn get_enum<T>(
    row: &Row,
    column: &str,
    parse: fn(&str) -> Option<T>,
) -> rusqlite::Result<T> {
    let text: String = row.get(column)?;
    parse(&text).ok_or_else(|| {
        let error = AppError::internal(format!("unknown value '{text}' in {column}"));
        bad_value(row, column, error)
    })
}

/// A value in the database that doesn't fit its Rust type.
fn bad_value(row: &Row, column: &str, error: AppError) -> rusqlite::Error {
    let index = row.as_ref().column_index(column).unwrap_or(0);
    rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
}
