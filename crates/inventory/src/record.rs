//! `record` and `reverse`: the only ways stock changes (INV-01). Each checks the
//! change first, then writes ledger rows that also update the cached stock.

use pos_core::{Id, Qty};
use rusqlite::Transaction;

use crate::convert::{add_qty, negate_qty};
use crate::fefo::split_fefo;
use crate::{
    BatchChoice, InventoryError, LedgerRow, MoveCtx, MoveKind, Movement, RecordedRow, batches,
    ledger, levels, reasons,
};

/// Records one stock change. FEFO may split it into several rows, one per batch.
/// With `allow_negative` off, a change that would leave less than zero is refused (INV-04).
/// If this returns an error, the caller rolls back the transaction.
pub fn record(
    transaction: &Transaction,
    movement: Movement,
    allow_negative: bool,
) -> Result<Vec<RecordedRow>, InventoryError> {
    check_quantity(movement.kind, movement.qty_delta)?;
    check_reason(transaction, &movement)?;
    check_level(transaction, &movement, allow_negative)?;
    let parts = choose_batches(transaction, &movement, allow_negative)?;

    let mut written = Vec::new();
    for (batch_id, qty_delta) in parts {
        let row = LedgerRow {
            id: Id::new(),
            product_id: movement.product_id.clone(),
            batch_id,
            qty_delta,
            kind: movement.kind,
            reason_code: movement.reason_code.clone(),
            note: movement.note.clone(),
            ref_: movement.ref_.clone(),
            unit_cost: movement.unit_cost,
            at: movement.ctx.at,
            business_date: movement.ctx.business_date,
            till_id: movement.ctx.till_id.clone(),
            user_id: movement.ctx.user_id.clone(),
        };
        written.push(ledger::write_row(transaction, &row)?);
    }
    Ok(written)
}

/// Writes the exact opposite of every row that points at a document, on the same
/// batches and at the same cost, as `kind` rows (e.g. a sale's rows → `sale_void`).
/// The new rows point at the same document. A document with no rows (only kits or
/// non-stock products, INV-09) gives an empty list. Doing it twice is refused.
pub fn reverse(
    transaction: &Transaction,
    ref_type: &str,
    ref_id: &Id,
    kind: MoveKind,
    ctx: &MoveCtx,
) -> Result<Vec<RecordedRow>, InventoryError> {
    let originals = ledger::rows_for_ref(transaction, ref_type, ref_id)?;
    if originals.iter().any(|row| row.kind == kind) {
        return Err(InventoryError::Conflict("already_reversed"));
    }
    let mut written = Vec::new();
    for original in originals {
        let opposite = LedgerRow {
            id: Id::new(),
            qty_delta: negate_qty(original.qty_delta)?,
            kind,
            note: None,
            at: ctx.at,
            business_date: ctx.business_date,
            till_id: ctx.till_id.clone(),
            user_id: ctx.user_id.clone(),
            ..original
        };
        check_quantity(kind, opposite.qty_delta)?;
        written.push(ledger::write_row(transaction, &opposite)?);
    }
    Ok(written)
}

/// The quantity isn't zero and goes the way its kind says.
fn check_quantity(kind: MoveKind, qty_delta: Qty) -> Result<(), InventoryError> {
    if qty_delta.0 == 0 {
        return Err(InventoryError::Invalid("qty_zero"));
    }
    if !kind.allows(qty_delta) {
        return Err(InventoryError::Invalid("qty_wrong_sign"));
    }
    Ok(())
}

/// An adjustment needs a reason (INV-02). Any reason given must be active and fit the
/// direction, and the reason "other" needs a note.
fn check_reason(transaction: &Transaction, movement: &Movement) -> Result<(), InventoryError> {
    let Some(code) = &movement.reason_code else {
        if movement.kind == MoveKind::Adjust {
            return Err(InventoryError::Invalid("reason_needed"));
        }
        return Ok(());
    };
    let Some(direction) = reasons::active_direction(transaction, code)? else {
        return Err(InventoryError::Invalid("reason_unknown"));
    };
    if !direction.fits(movement.qty_delta) {
        return Err(InventoryError::Invalid("reason_wrong_direction"));
    }
    let has_note = movement
        .note
        .as_ref()
        .is_some_and(|note| !note.trim().is_empty());
    if code == "other" && !has_note {
        return Err(InventoryError::Invalid("note_needed"));
    }
    Ok(())
}

/// With negative stock not allowed, the product's level must stay at zero or above.
fn check_level(
    transaction: &Transaction,
    movement: &Movement,
    allow_negative: bool,
) -> Result<(), InventoryError> {
    if allow_negative || movement.qty_delta.0 > 0 {
        return Ok(());
    }
    let level = levels::on_hand(transaction, &movement.product_id)?;
    if add_qty(level, movement.qty_delta)?.0 < 0 {
        return Err(InventoryError::NotEnoughStock {
            available: Qty(level.0.max(0)),
        });
    }
    Ok(())
}

/// Decides which batch (or batches) the change uses, and checks the batch rules (INV-05).
fn choose_batches(
    transaction: &Transaction,
    movement: &Movement,
    allow_negative: bool,
) -> Result<Vec<(Option<Id>, Qty)>, InventoryError> {
    let coming_in = movement.qty_delta.0 > 0;
    match &movement.batch {
        BatchChoice::None => {
            if movement.tracks_expiry && coming_in {
                return Err(InventoryError::Invalid("expiry_needed"));
            }
            Ok(vec![(None, movement.qty_delta)])
        }
        BatchChoice::Specific(batch_id) => {
            let batch = batches::load(transaction, batch_id)?;
            if batch.product_id != movement.product_id {
                return Err(InventoryError::Invalid("batch_of_other_product"));
            }
            if movement.tracks_expiry && coming_in && batch.expiry_date.is_none() {
                return Err(InventoryError::Invalid("expiry_needed"));
            }
            if !allow_negative && !coming_in && add_qty(batch.on_hand, movement.qty_delta)?.0 < 0 {
                return Err(InventoryError::NotEnoughStockInBatch {
                    available: Qty(batch.on_hand.0.max(0)),
                });
            }
            Ok(vec![(Some(batch.id), movement.qty_delta)])
        }
        BatchChoice::Fefo => {
            // FEFO picks batches to take from. Stock coming in must name its batch.
            if coming_in {
                return Err(InventoryError::Invalid("batch_needed"));
            }
            let stock = batches::fefo_stock(transaction, &movement.product_id)?;
            split_fefo(movement.qty_delta, &stock)
        }
    }
}
