//! Writing and reading ledger rows. Every row written here also updates the cached
//! level (and batch) in the same transaction, so the caches always equal the ledger.

use pos_core::{Id, Money, Qty, Timestamp};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};

use crate::convert::{add_qty, business_date_at, id_at, kind_at, optional_id_at};
use crate::{InventoryError, LedgerRow, RecordedRow, levels};

/// Every ledger column, in the order `row_from` reads them.
pub(crate) const LEDGER_COLUMNS: &str = "id, product_id, batch_id, qty_delta_milli, kind, \
     reason_code, note, ref_type, ref_id, unit_cost_minor, at, business_date, till_id, user_id";

/// Adds one row to the ledger and updates the cached level and batch stock.
pub(crate) fn write_row(
    transaction: &Transaction,
    row: &LedgerRow,
) -> Result<RecordedRow, InventoryError> {
    let (ref_type, ref_id) = match &row.ref_ {
        Some((ref_type, ref_id)) => (Some(ref_type.as_str()), Some(ref_id.as_str())),
        None => (None, None),
    };
    transaction.execute(
        "INSERT INTO inventory_ledger (id, product_id, batch_id, qty_delta_milli, kind,
           reason_code, note, ref_type, ref_id, unit_cost_minor, at, business_date, till_id, user_id)
         VALUES (:id, :product_id, :batch_id, :qty_delta, :kind, :reason_code, :note,
           :ref_type, :ref_id, :unit_cost, :at, :business_date, :till_id, :user_id)",
        named_params! {
            ":id": row.id.as_str(),
            ":product_id": row.product_id.as_str(),
            ":batch_id": row.batch_id.as_ref().map(Id::as_str),
            ":qty_delta": row.qty_delta.0,
            ":kind": row.kind.as_str(),
            ":reason_code": row.reason_code,
            ":note": row.note,
            ":ref_type": ref_type,
            ":ref_id": ref_id,
            ":unit_cost": row.unit_cost.0,
            ":at": row.at.0,
            ":business_date": row.business_date.to_string(),
            ":till_id": row.till_id.as_str(),
            ":user_id": row.user_id.as_ref().map(Id::as_str),
        },
    )?;
    add_to_level(transaction, &row.product_id, row.qty_delta, row.at)?;
    if let Some(batch_id) = &row.batch_id {
        add_to_batch(transaction, batch_id, row.qty_delta)?;
    }
    Ok(RecordedRow {
        id: row.id.clone(),
        batch_id: row.batch_id.clone(),
        qty_delta: row.qty_delta,
    })
}

/// Every row that points at one document, oldest first.
pub(crate) fn rows_for_ref(
    connection: &Connection,
    ref_type: &str,
    ref_id: &Id,
) -> Result<Vec<LedgerRow>, InventoryError> {
    let sql = format!(
        "SELECT {LEDGER_COLUMNS} FROM inventory_ledger
         WHERE ref_type = :ref_type AND ref_id = :ref_id ORDER BY at, rowid"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        named_params! { ":ref_type": ref_type, ":ref_id": ref_id.as_str() },
        row_from,
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Reads a ledger row selected with `LEDGER_COLUMNS`.
pub(crate) fn row_from(row: &Row) -> rusqlite::Result<LedgerRow> {
    let ref_type: Option<String> = row.get(7)?;
    let ref_id = optional_id_at(row, 8)?;
    let ref_ = match (ref_type, ref_id) {
        (Some(ref_type), Some(ref_id)) => Some((ref_type, ref_id)),
        _ => None,
    };
    Ok(LedgerRow {
        id: id_at(row, 0)?,
        product_id: id_at(row, 1)?,
        batch_id: optional_id_at(row, 2)?,
        qty_delta: Qty(row.get(3)?),
        kind: kind_at(row, 4)?,
        reason_code: row.get(5)?,
        note: row.get(6)?,
        ref_,
        unit_cost: Money(row.get(9)?),
        at: Timestamp(row.get(10)?),
        business_date: business_date_at(row, 11)?,
        till_id: id_at(row, 12)?,
        user_id: optional_id_at(row, 13)?,
    })
}

/// Changes a product's cached level. The new value is worked out in Rust, because
/// SQLite would quietly turn an overflowing sum into a decimal number.
fn add_to_level(
    transaction: &Transaction,
    product_id: &Id,
    qty_delta: Qty,
    at: Timestamp,
) -> Result<(), InventoryError> {
    let current = levels::on_hand(transaction, product_id)?;
    let new_level = add_qty(current, qty_delta)?;
    transaction.execute(
        "INSERT INTO inventory_levels (product_id, on_hand_milli, updated_at)
         VALUES (:product_id, :on_hand, :at)
         ON CONFLICT(product_id) DO UPDATE
           SET on_hand_milli = excluded.on_hand_milli, updated_at = excluded.updated_at",
        named_params! {
            ":product_id": product_id.as_str(),
            ":on_hand": new_level.0,
            ":at": at.0,
        },
    )?;
    Ok(())
}

/// Changes a batch's cached stock.
fn add_to_batch(
    transaction: &Transaction,
    batch_id: &Id,
    qty_delta: Qty,
) -> Result<(), InventoryError> {
    let current: Option<i64> = transaction
        .query_row(
            "SELECT on_hand_milli FROM inventory_batches WHERE id = :id",
            named_params! { ":id": batch_id.as_str() },
            |row| row.get(0),
        )
        .optional()?;
    let Some(current) = current else {
        return Err(InventoryError::NotFound {
            entity: "batch",
            key: batch_id.to_string(),
        });
    };
    let new_stock = add_qty(Qty(current), qty_delta)?;
    transaction.execute(
        "UPDATE inventory_batches SET on_hand_milli = :on_hand WHERE id = :id",
        named_params! { ":on_hand": new_stock.0, ":id": batch_id.as_str() },
    )?;
    Ok(())
}
