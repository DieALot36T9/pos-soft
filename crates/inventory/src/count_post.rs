//! Reviewing a count's differences and posting them as `count` ledger rows (INV-06).

use std::collections::HashMap;

use pos_core::{Id, Money, Qty, mul_qty};
use rusqlite::{Connection, Transaction, named_params};
use serde::{Deserialize, Serialize};

use crate::convert::{add_qty, id_at, negate_qty, optional_id_at};
use crate::counts::require_open;
use crate::{
    CountScope, CountStatus, InventoryError, LedgerRow, MoveCtx, MoveKind, RecordedRow, batches,
    get_count, ledger,
};

/// One line of the variance report: what was expected, what was counted, the
/// difference and what the difference is worth at cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VarianceRow {
    pub product_id: Id,
    pub batch_id: Option<Id>,
    pub expected: Qty,
    pub counted: Qty,
    /// Counted minus expected. Negative means stock is missing.
    pub difference: Qty,
    /// The difference × the product's cost. Example: 2 missing at 20.00 → −40.00.
    pub cost_value: Money,
    /// False for a product in a full count that nobody counted. Posting sets it to 0.
    pub was_counted: bool,
}

/// The variance report for a count. `costs` holds each product's current cost price
/// (from catalog, via `app`); a product missing from it is valued at 0.
/// For an open full count it also lists stocked products nobody has counted yet.
pub fn count_variances(
    connection: &Connection,
    count_id: &Id,
    costs: &HashMap<Id, Money>,
) -> Result<Vec<VarianceRow>, InventoryError> {
    let count = get_count(connection, count_id)?;
    let mut rows = counted_lines(connection, count_id, costs)?;
    if count.scope == CountScope::Full && count.status == CountStatus::Open {
        for (product_id, on_hand) in uncounted_products(connection, count_id)? {
            let difference = negate_qty(on_hand)?;
            rows.push(VarianceRow {
                cost_value: value_of(costs, &product_id, difference)?,
                product_id,
                batch_id: None,
                expected: on_hand,
                counted: Qty(0),
                difference,
                was_counted: false,
            });
        }
    }
    Ok(rows)
}

/// Posts an open count: writes one `count` row for every difference, pointing at the
/// count. In a full count, products nobody counted are set to 0 (the UI asks first).
/// Rows use the cost from `costs`, or 0 if the product isn't there.
pub fn post_count(
    transaction: &Transaction,
    count_id: &Id,
    costs: &HashMap<Id, Money>,
    ctx: &MoveCtx,
) -> Result<Vec<RecordedRow>, InventoryError> {
    let count = require_open(transaction, count_id)?;
    let mut changes: Vec<(Id, Option<Id>, Qty)> = Vec::new();
    for line in counted_lines(transaction, count_id, costs)? {
        changes.push((line.product_id, line.batch_id, line.difference));
    }
    if count.scope == CountScope::Full {
        for (product_id, on_hand) in uncounted_products(transaction, count_id)? {
            changes.extend(zero_product(transaction, &product_id, on_hand)?);
        }
    }

    let mut written = Vec::new();
    for (product_id, batch_id, qty_delta) in changes {
        if qty_delta.0 == 0 {
            continue;
        }
        let row = LedgerRow {
            id: Id::new(),
            unit_cost: costs.get(&product_id).copied().unwrap_or(Money::zero()),
            product_id,
            batch_id,
            qty_delta,
            kind: MoveKind::Count,
            reason_code: None,
            note: None,
            ref_: Some(("count".to_string(), count_id.clone())),
            at: ctx.at,
            business_date: ctx.business_date,
            till_id: ctx.till_id.clone(),
            user_id: ctx.user_id.clone(),
        };
        written.push(ledger::write_row(transaction, &row)?);
    }
    transaction.execute(
        "UPDATE inventory_counts SET status = 'posted', posted_at = :at, posted_by = :user
         WHERE id = :id",
        named_params! {
            ":at": ctx.at.0,
            ":user": ctx.user_id.as_ref().map(Id::as_str),
            ":id": count_id.as_str(),
        },
    )?;
    Ok(written)
}

/// The counted lines, with their differences, by product then batch.
fn counted_lines(
    connection: &Connection,
    count_id: &Id,
    costs: &HashMap<Id, Money>,
) -> Result<Vec<VarianceRow>, InventoryError> {
    let mut statement = connection.prepare(
        "SELECT product_id, batch_id, expected_milli, counted_milli FROM inventory_count_lines
         WHERE count_id = :count_id ORDER BY product_id, batch_id",
    )?;
    let lines = statement.query_map(named_params! { ":count_id": count_id.as_str() }, |row| {
        Ok((
            id_at(row, 0)?,
            optional_id_at(row, 1)?,
            Qty(row.get(2)?),
            Qty(row.get(3)?),
        ))
    })?;
    let mut rows = Vec::new();
    for line in lines {
        let (product_id, batch_id, expected, counted) = line?;
        let difference = add_qty(counted, negate_qty(expected)?)?;
        rows.push(VarianceRow {
            cost_value: value_of(costs, &product_id, difference)?,
            product_id,
            batch_id,
            expected,
            counted,
            difference,
            was_counted: true,
        });
    }
    Ok(rows)
}

/// Products with stock (or below zero) that have no line in this count.
fn uncounted_products(
    connection: &Connection,
    count_id: &Id,
) -> Result<Vec<(Id, Qty)>, InventoryError> {
    let mut statement = connection.prepare(
        "SELECT product_id, on_hand_milli FROM inventory_levels
         WHERE on_hand_milli <> 0
           AND product_id NOT IN (SELECT product_id FROM inventory_count_lines WHERE count_id = :count_id)
         ORDER BY product_id",
    )?;
    let rows = statement.query_map(named_params! { ":count_id": count_id.as_str() }, |row| {
        Ok((id_at(row, 0)?, Qty(row.get(1)?)))
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// The changes that bring a product to 0: empty each batch, then whatever is left
/// outside batches.
fn zero_product(
    connection: &Connection,
    product_id: &Id,
    on_hand: Qty,
) -> Result<Vec<(Id, Option<Id>, Qty)>, InventoryError> {
    let mut changes = Vec::new();
    let mut outside_batches = on_hand;
    for batch in batches::batches_of(connection, product_id)? {
        changes.push((
            product_id.clone(),
            Some(batch.id),
            negate_qty(batch.on_hand)?,
        ));
        outside_batches = add_qty(outside_batches, negate_qty(batch.on_hand)?)?;
    }
    changes.push((product_id.clone(), None, negate_qty(outside_batches)?));
    Ok(changes)
}

fn value_of(
    costs: &HashMap<Id, Money>,
    product_id: &Id,
    qty: Qty,
) -> Result<Money, InventoryError> {
    let cost = costs.get(product_id).copied().unwrap_or(Money::zero());
    mul_qty(cost, qty).map_err(|error| InventoryError::BadData(error.to_string()))
}
