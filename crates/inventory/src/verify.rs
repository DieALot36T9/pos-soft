//! The self-check (INV-10, ADR-010): every cached level and batch stock must equal
//! the sum of its ledger rows. Differences are reported, never fixed here.

use pos_core::{Id, Qty};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::InventoryError;
use crate::convert::{id_at, optional_id_at};

/// A cache that doesn't match the ledger. `batch_id` is set when a batch's stock is wrong,
/// and empty when the product's level is wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mismatch {
    pub product_id: Id,
    pub batch_id: Option<Id>,
    pub cached: Qty,
    pub ledger_sum: Qty,
}

/// Compares every level and every batch with its ledger rows. An empty list means all is well.
pub fn verify(connection: &Connection) -> Result<Vec<Mismatch>, InventoryError> {
    // Levels: a product may have a level with no rows, or rows with no level.
    let levels = "
      SELECT product_id, NULL, SUM(cached), SUM(from_ledger) FROM (
        SELECT product_id, on_hand_milli AS cached, 0 AS from_ledger FROM inventory_levels
        UNION ALL
        SELECT product_id, 0, qty_delta_milli FROM inventory_ledger
      )
      GROUP BY product_id
      HAVING SUM(cached) <> SUM(from_ledger)";
    let batches = "
      SELECT b.product_id, b.id, b.on_hand_milli, COALESCE(SUM(l.qty_delta_milli), 0)
      FROM inventory_batches b LEFT JOIN inventory_ledger l ON l.batch_id = b.id
      GROUP BY b.id
      HAVING b.on_hand_milli <> COALESCE(SUM(l.qty_delta_milli), 0)";
    let sql = format!("{levels} UNION ALL {batches} ORDER BY 1, 2");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map([], |row| {
        Ok(Mismatch {
            product_id: id_at(row, 0)?,
            batch_id: optional_id_at(row, 1)?,
            cached: Qty(row.get(2)?),
            ledger_sum: Qty(row.get(3)?),
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}
