//! Price history: every sale or cost price change is recorded (rule CAT-09).

use pos_core::{Id, Money, Timestamp};
use rusqlite::{Connection, Row, Transaction, named_params};

use crate::error::{CatalogError, invalid};
use crate::read::get;
use crate::rows::{read_money, read_optional_id};
use crate::types::{PriceChange, PriceField, PriceHistoryRow};

/// The reasons a price may change: typed by hand, a goods receipt, or an import.
const REASONS: &[&str] = &["manual", "grn", "import"];

/// Sets a product's cost price and records the change. Receiving goods calls this
/// with reason "grn"; nothing is recorded when the cost stays the same.
pub fn set_cost(
    tx: &Transaction,
    product_id: &Id,
    cost: Money,
    reason: &str,
    by: &Id,
    now: Timestamp,
) -> Result<(), CatalogError> {
    if cost.0 < 0 {
        return Err(invalid("cost_price", "price_negative"));
    }
    if !REASONS.contains(&reason) {
        return Err(invalid("reason", "price_reason_invalid"));
    }
    let current = get(tx, product_id)?;
    if current.cost_price == cost {
        return Ok(());
    }
    tx.execute(
        "UPDATE catalog_products SET cost_price_minor = :cost, updated_at = :now WHERE id = :id",
        named_params! { ":id": product_id.as_str(), ":cost": cost.0, ":now": now.0 },
    )?;
    record_price_change(
        tx,
        product_id,
        PriceField::CostPrice,
        (current.cost_price, cost),
        (by, reason),
        now,
    )?;
    Ok(())
}

/// Writes one history row if `(old, new)` differ, and returns the change.
/// The last-but-one argument is the user and the reason. Nothing is written when the price is the same.
pub(crate) fn record_price_change(
    tx: &Transaction,
    product_id: &Id,
    field: PriceField,
    (old, new): (Money, Money),
    (by, reason): (&Id, &str),
    now: Timestamp,
) -> Result<Option<PriceChange>, CatalogError> {
    if old == new {
        return Ok(None);
    }
    tx.prepare_cached(
        "INSERT INTO catalog_price_changes
            (id, product_id, field, old_minor, new_minor, changed_at, changed_by, reason)
         VALUES (:id, :product_id, :field, :old, :new, :now, :by, :reason)",
    )?
    .execute(named_params! {
        ":id": Id::new().as_str(),
        ":product_id": product_id.as_str(),
        ":field": field.as_str(),
        ":old": old.0,
        ":new": new.0,
        ":now": now.0,
        ":by": by.as_str(),
        ":reason": reason,
    })?;
    Ok(Some(PriceChange { old, new }))
}

/// A product's price history, oldest first, for the product page.
pub fn price_history(
    connection: &Connection,
    product_id: &Id,
) -> Result<Vec<PriceHistoryRow>, CatalogError> {
    let rows = connection
        .prepare_cached(
            "SELECT field, old_minor, new_minor, changed_at, changed_by, reason
             FROM catalog_price_changes WHERE product_id = :id ORDER BY changed_at, rowid",
        )?
        .query_map(
            named_params! { ":id": product_id.as_str() },
            read_history_row,
        )?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

fn read_history_row(row: &Row) -> rusqlite::Result<PriceHistoryRow> {
    let field: String = row.get(0)?;
    Ok(PriceHistoryRow {
        // The table's CHECK allows only these two values.
        field: if field == "cost_price" {
            PriceField::CostPrice
        } else {
            PriceField::SalePrice
        },
        old: read_money(row, 1)?,
        new: read_money(row, 2)?,
        changed_at: Timestamp(row.get(3)?),
        changed_by: read_optional_id(row, 4)?,
        reason: row.get(5)?,
    })
}
