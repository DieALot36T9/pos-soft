//! Stock levels, reorder rules and the low-stock list (INV-01, INV-03, INV-04).

use std::collections::HashMap;

use chrono::NaiveDate;
use pos_core::{Id, Qty};
use rusqlite::{Connection, OptionalExtension, Transaction, named_params};
use serde::{Deserialize, Serialize};

use crate::convert::id_at;
use crate::{InventoryError, Page, batches};

/// What the till needs to know before selling a product (INV-04, INV-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Availability {
    pub on_hand: Qty,
    /// Some stock is in a batch past its expiry date. The till warns or blocks (setting).
    pub has_expired_batch: bool,
}

/// When to order more of a product. Example: level 10, qty 24 → "order 24 when below 10".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReorderRule {
    pub level: Qty,
    pub qty: Qty,
}

/// One product in the low-stock list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LowStockRow {
    pub product_id: Id,
    pub on_hand: Qty,
    pub reorder_level: Qty,
    pub reorder_qty: Qty,
}

/// How much of a product the shop has now. A product never moved has 0.
pub fn on_hand(connection: &Connection, product: &Id) -> Result<Qty, InventoryError> {
    let level: Option<i64> = connection
        .query_row(
            "SELECT on_hand_milli FROM inventory_levels WHERE product_id = :product_id",
            named_params! { ":product_id": product.as_str() },
            |row| row.get(0),
        )
        .optional()?;
    Ok(Qty(level.unwrap_or(0)))
}

/// Stock for several products at once. Every id asked for is in the answer.
pub fn on_hand_many(
    connection: &Connection,
    ids: &[Id],
) -> Result<HashMap<Id, Qty>, InventoryError> {
    let mut levels = HashMap::new();
    for id in ids {
        levels.insert(id.clone(), on_hand(connection, id)?);
    }
    Ok(levels)
}

/// Stock on hand and whether any of it is in an expired batch.
pub fn availability(
    connection: &Connection,
    product: &Id,
    today: NaiveDate,
) -> Result<Availability, InventoryError> {
    Ok(Availability {
        on_hand: on_hand(connection, product)?,
        has_expired_batch: batches::has_expired_batch(connection, product, today)?,
    })
}

/// Sets (or replaces) a product's reorder level and reorder quantity (INV-03).
pub fn set_reorder_rule(
    transaction: &Transaction,
    product: &Id,
    level: Qty,
    qty: Qty,
) -> Result<(), InventoryError> {
    if level.0 < 0 || qty.0 < 0 {
        return Err(InventoryError::Invalid("reorder_below_zero"));
    }
    transaction.execute(
        "INSERT INTO inventory_reorder_rules (product_id, reorder_level_milli, reorder_qty_milli)
         VALUES (:product_id, :level, :qty)
         ON CONFLICT(product_id) DO UPDATE
           SET reorder_level_milli = excluded.reorder_level_milli,
               reorder_qty_milli = excluded.reorder_qty_milli",
        named_params! { ":product_id": product.as_str(), ":level": level.0, ":qty": qty.0 },
    )?;
    Ok(())
}

/// A product's reorder rule, if it has one.
pub fn reorder_rule(
    connection: &Connection,
    product: &Id,
) -> Result<Option<ReorderRule>, InventoryError> {
    let rule = connection
        .query_row(
            "SELECT reorder_level_milli, reorder_qty_milli FROM inventory_reorder_rules
             WHERE product_id = :product_id",
            named_params! { ":product_id": product.as_str() },
            |row| {
                Ok(ReorderRule {
                    level: Qty(row.get(0)?),
                    qty: Qty(row.get(1)?),
                })
            },
        )
        .optional()?;
    Ok(rule)
}

/// Removes a product's reorder rule, so it never shows as low stock.
pub fn remove_reorder_rule(transaction: &Transaction, product: &Id) -> Result<(), InventoryError> {
    transaction.execute(
        "DELETE FROM inventory_reorder_rules WHERE product_id = :product_id",
        named_params! { ":product_id": product.as_str() },
    )?;
    Ok(())
}

/// Products below their reorder level, biggest shortfall first (INV-03).
/// At the level itself a product is not low yet: level 10 and stock 10 is fine, 9 is low.
pub fn low_stock(connection: &Connection, page: Page) -> Result<Vec<LowStockRow>, InventoryError> {
    let mut statement = connection.prepare(
        "SELECT product_id, on_hand_milli, reorder_level_milli, reorder_qty_milli
         FROM inventory_v_levels
         WHERE reorder_level_milli IS NOT NULL AND on_hand_milli < reorder_level_milli
         ORDER BY reorder_level_milli - on_hand_milli DESC, product_id
         LIMIT :limit OFFSET :offset",
    )?;
    let rows = statement.query_map(
        named_params! { ":limit": page.limit, ":offset": page.offset },
        |row| {
            Ok(LowStockRow {
                product_id: id_at(row, 0)?,
                on_hand: Qty(row.get(1)?),
                reorder_level: Qty(row.get(2)?),
                reorder_qty: Qty(row.get(3)?),
            })
        },
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// How many products are low on stock, for the back-office badge (INV-03).
pub fn low_stock_count(connection: &Connection) -> Result<u32, InventoryError> {
    let count: u32 = connection.query_row(
        "SELECT COUNT(*) FROM inventory_v_levels
         WHERE reorder_level_milli IS NOT NULL AND on_hand_milli < reorder_level_milli",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}
