//! Batches: deliveries of a product with their own expiry date and cost (INV-05).

use chrono::{Days, NaiveDate};
use pos_core::{Id, Money, Qty, Timestamp};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};

use crate::InventoryError;
use crate::convert::{date_text, id_at, optional_date_at};

/// One batch and the stock it holds now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchRow {
    pub id: Id,
    pub product_id: Id,
    pub batch_no: Option<String>,
    pub expiry_date: Option<NaiveDate>,
    pub unit_cost: Money,
    pub received_at: Timestamp,
    pub on_hand: Qty,
}

const BATCH_COLUMNS: &str =
    "id, product_id, batch_no, expiry_date, unit_cost_minor, received_at, on_hand_milli";

/// First expiry first; batches with no expiry date last; then oldest delivery first.
const FEFO_ORDER: &str = "ORDER BY expiry_date IS NULL, expiry_date, received_at, rowid";

/// Makes a new, empty batch. Stock goes into it with `record` (kind `receive`).
pub fn create_batch(
    transaction: &Transaction,
    product: &Id,
    batch_no: Option<&str>,
    expiry: Option<NaiveDate>,
    cost: Money,
    now: Timestamp,
) -> Result<Id, InventoryError> {
    if cost.0 < 0 {
        return Err(InventoryError::Invalid("cost_below_zero"));
    }
    let id = Id::new();
    let batch_no = batch_no.map(str::trim).filter(|text| !text.is_empty());
    transaction.execute(
        "INSERT INTO inventory_batches (id, product_id, batch_no, expiry_date, unit_cost_minor, received_at)
         VALUES (:id, :product_id, :batch_no, :expiry_date, :cost, :received_at)",
        named_params! {
            ":id": id.as_str(),
            ":product_id": product.as_str(),
            ":batch_no": batch_no,
            ":expiry_date": expiry.map(date_text),
            ":cost": cost.0,
            ":received_at": now.0,
        },
    )?;
    Ok(id)
}

/// A product's batches that hold stock (or are below zero), in FEFO order.
pub fn batches_of(connection: &Connection, product: &Id) -> Result<Vec<BatchRow>, InventoryError> {
    let sql = format!(
        "SELECT {BATCH_COLUMNS} FROM inventory_batches
         WHERE product_id = :product_id AND on_hand_milli <> 0 {FEFO_ORDER}"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        named_params! { ":product_id": product.as_str() },
        batch_from,
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Batches with stock that expire within `days` days of `today`, soonest first.
/// Already expired batches are included, at the top. Example: today 2 March, 30 days →
/// everything expiring on or before 1 April.
pub fn near_expiry(
    connection: &Connection,
    today: NaiveDate,
    days: u32,
) -> Result<Vec<BatchRow>, InventoryError> {
    let last_day = today
        .checked_add_days(Days::new(u64::from(days)))
        .ok_or_else(|| InventoryError::BadData(format!("{today} + {days} days is out of range")))?;
    let sql = format!(
        "SELECT {BATCH_COLUMNS} FROM inventory_batches
         WHERE on_hand_milli > 0 AND expiry_date IS NOT NULL AND expiry_date <= :last_day
         {FEFO_ORDER}"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        named_params! { ":last_day": date_text(last_day) },
        batch_from,
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// One batch by id.
pub(crate) fn load(connection: &Connection, batch_id: &Id) -> Result<BatchRow, InventoryError> {
    let sql = format!("SELECT {BATCH_COLUMNS} FROM inventory_batches WHERE id = :id");
    connection
        .query_row(&sql, named_params! { ":id": batch_id.as_str() }, batch_from)
        .optional()?
        .ok_or_else(|| InventoryError::NotFound {
            entity: "batch",
            key: batch_id.to_string(),
        })
}

/// The batches a stock-out can take from, in FEFO order, with what each holds.
pub(crate) fn fefo_stock(
    connection: &Connection,
    product: &Id,
) -> Result<Vec<(Id, Qty)>, InventoryError> {
    let sql = format!(
        "SELECT id, on_hand_milli FROM inventory_batches
         WHERE product_id = :product_id AND on_hand_milli > 0 {FEFO_ORDER}"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(named_params! { ":product_id": product.as_str() }, |row| {
        Ok((id_at(row, 0)?, Qty(row.get(1)?)))
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Whether a product has stock in a batch whose expiry date has passed.
/// A batch is still good on its expiry date itself.
pub(crate) fn has_expired_batch(
    connection: &Connection,
    product: &Id,
    today: NaiveDate,
) -> Result<bool, InventoryError> {
    let found: Option<i64> = connection
        .query_row(
            "SELECT 1 FROM inventory_batches
             WHERE product_id = :product_id AND on_hand_milli > 0
               AND expiry_date IS NOT NULL AND expiry_date < :today
             LIMIT 1",
            named_params! { ":product_id": product.as_str(), ":today": date_text(today) },
            |row| row.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}

fn batch_from(row: &Row) -> rusqlite::Result<BatchRow> {
    Ok(BatchRow {
        id: id_at(row, 0)?,
        product_id: id_at(row, 1)?,
        batch_no: row.get(2)?,
        expiry_date: optional_date_at(row, 3)?,
        unit_cost: Money(row.get(4)?),
        received_at: Timestamp(row.get(5)?),
        on_hand: Qty(row.get(6)?),
    })
}
