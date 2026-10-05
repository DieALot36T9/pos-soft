//! Recording money movements, undoing them for voids, and expected cash.

use pos_core::{Id, Money, Timestamp};
use rusqlite::{Connection, Transaction};

use crate::model::ShiftMovement;
use crate::shift::{require_open, require_shift};
use crate::{ShiftsError, rules, store};

/// Records one movement of money in an open shift. Example: a cash sale payment of +450.00.
/// The amount's sign must match its kind: + into the till, − out of it.
pub fn record(tx: &Transaction, shift: &Id, movement: ShiftMovement) -> Result<(), ShiftsError> {
    require_open(tx, shift)?;
    rules::check_movement(&movement)?;
    store::insert_movement(tx, shift, &movement)
}

/// Undoes every movement of a voided document by adding the same rows with the
/// opposite sign (RET-04). The shift must still be open; after that, use a return.
pub fn reverse_ref(
    tx: &Transaction,
    shift: &Id,
    ref_type: &str,
    ref_id: &Id,
    by: &Id,
    now: Timestamp,
) -> Result<(), ShiftsError> {
    require_open(tx, shift)?;
    let movements = store::load_movements_for_ref(tx, shift, ref_type, ref_id)?;
    for reversal in rules::reversals(&movements, ref_type, ref_id, by, now)? {
        store::insert_movement(tx, shift, &reversal)?;
    }
    Ok(())
}

/// The cash that should be in the drawer now: every cash movement added up (SHF-06).
pub fn expected_cash(conn: &Connection, shift: &Id) -> Result<Money, ShiftsError> {
    require_shift(conn, shift)?;
    rules::expected_cash(&store::load_movements(conn, shift)?)
}
