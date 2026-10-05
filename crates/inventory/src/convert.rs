//! Small helpers that turn database columns into our types and back,
//! and add quantities without overflowing.

use chrono::NaiveDate;
use pos_core::{BusinessDate, Id, Qty};
use rusqlite::Row;
use rusqlite::types::Type;

use crate::{InventoryError, MoveKind};

/// Reads an id column.
pub(crate) fn id_at(row: &Row, index: usize) -> rusqlite::Result<Id> {
    let text: String = row.get(index)?;
    parse_id(&text).map_err(|error| bad_column(index, error))
}

/// Reads an id column that may be NULL.
pub(crate) fn optional_id_at(row: &Row, index: usize) -> rusqlite::Result<Option<Id>> {
    let text: Option<String> = row.get(index)?;
    match text {
        Some(text) => parse_id(&text)
            .map(Some)
            .map_err(|error| bad_column(index, error)),
        None => Ok(None),
    }
}

/// Reads a calendar date column ("YYYY-MM-DD") that may be NULL.
pub(crate) fn optional_date_at(row: &Row, index: usize) -> rusqlite::Result<Option<NaiveDate>> {
    let text: Option<String> = row.get(index)?;
    match text {
        Some(text) => NaiveDate::parse_from_str(&text, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| bad_column(index, bad_data(format!("bad date '{text}'")))),
        None => Ok(None),
    }
}

/// Reads a business date column.
pub(crate) fn business_date_at(row: &Row, index: usize) -> rusqlite::Result<BusinessDate> {
    let text: String = row.get(index)?;
    BusinessDate::parse(&text)
        .map_err(|_| bad_column(index, bad_data(format!("bad business date '{text}'"))))
}

/// Reads a stock kind column.
pub(crate) fn kind_at(row: &Row, index: usize) -> rusqlite::Result<MoveKind> {
    let text: String = row.get(index)?;
    MoveKind::parse(&text).map_err(|error| bad_column(index, error))
}

/// A date as stored in the database: "YYYY-MM-DD".
pub(crate) fn date_text(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

/// Adds two quantities. Too big to store is an error, never a wrong number.
pub(crate) fn add_qty(first: Qty, second: Qty) -> Result<Qty, InventoryError> {
    first
        .0
        .checked_add(second.0)
        .map(Qty)
        .ok_or_else(|| bad_data(format!("quantity overflow: {} + {}", first.0, second.0)))
}

/// The opposite quantity: 3 becomes −3.
pub(crate) fn negate_qty(qty: Qty) -> Result<Qty, InventoryError> {
    qty.0
        .checked_neg()
        .map(Qty)
        .ok_or_else(|| bad_data(format!("quantity overflow: -{}", qty.0)))
}

fn parse_id(text: &str) -> Result<Id, InventoryError> {
    Id::parse(text).map_err(|_| bad_data(format!("bad id '{text}'")))
}

fn bad_data(detail: String) -> InventoryError {
    InventoryError::BadData(detail)
}

/// A column that can't be read into our type. The details go to the log.
pub(crate) fn bad_column(index: usize, error: InventoryError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
}
