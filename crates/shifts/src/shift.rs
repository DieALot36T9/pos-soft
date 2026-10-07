//! Opening, finding and closing shifts.

use pos_core::{BusinessDate, Id, Money, Timestamp};
use rusqlite::{Connection, ErrorCode, Transaction};

use crate::model::{CASH, CloseResult, MovementKind, Shift, ShiftMovement, ShiftStatus};
use crate::store::{self, ClosedFigures, Stage};
use crate::{ShiftsError, rules};

/// Opens a shift on a till for one cashier, with the opening float (SHF-01, SHF-02).
/// `counts` is the optional count by denomination. If given, it must add up to `float`.
/// `business_date` comes from `app` (settings cutoff rule).
pub fn open(
    tx: &Transaction,
    till: &Id,
    user: &Id,
    float: Money,
    counts: &[(Money, u32)],
    business_date: BusinessDate,
    now: Timestamp,
) -> Result<Id, ShiftsError> {
    rules::check_float(float, counts)?;
    if store::open_shift_id(tx, till)?.is_some() {
        return Err(ShiftsError::AlreadyOpen { till: till.clone() });
    }
    let id = Id::new();
    let inserted = store::insert_shift(tx, &id, till, user, float, business_date, now);
    // The unique index is the final guard if another writer got there first.
    if let Err(ShiftsError::Database(error)) = &inserted
        && error.sqlite_error_code() == Some(ErrorCode::ConstraintViolation)
    {
        return Err(ShiftsError::AlreadyOpen { till: till.clone() });
    }
    inserted?;
    store::insert_counts(tx, &id, Stage::Open, counts)?;
    let float_row = ShiftMovement {
        kind: MovementKind::Float,
        method_id: None,
        method_kind: CASH.to_string(),
        amount: float,
        ref_type: None,
        ref_id: None,
        reason: None,
        at: now,
        user_id: user.clone(),
    };
    store::insert_movement(tx, &id, &float_row)?;
    Ok(id)
}

/// The open shift on a till, or nothing if the till has no open shift.
pub fn current(conn: &Connection, till: &Id) -> Result<Option<Shift>, ShiftsError> {
    match store::open_shift_id(conn, till)? {
        Some(id) => store::load_shift(conn, &id),
        None => Ok(None),
    }
}

/// One shift, open or closed (RPT-02: past Z reports can be reprinted).
pub fn get(conn: &Connection, shift: &Id) -> Result<Shift, ShiftsError> {
    require_shift(conn, shift)
}

/// The newest shifts first, at most `limit`. With `user`, only that cashier's shifts
/// (RPT-10: a cashier sees only their own X and Z reports).
pub fn list(conn: &Connection, user: Option<&Id>, limit: u32) -> Result<Vec<Shift>, ShiftsError> {
    store::list_shifts(conn, user, limit)
}

/// Closes a shift and saves expected cash, counted cash and the difference (SHF-06).
/// `counts` is the cash count by denomination; it may be empty only when `forced`
/// is true (an admin closing someone else's shift, SHF-07).
/// `app` checks permissions and that no carts are open before calling this.
pub fn close(
    tx: &Transaction,
    shift: &Id,
    counts: &[(Money, u32)],
    by: &Id,
    forced: bool,
    note: Option<String>,
    now: Timestamp,
) -> Result<CloseResult, ShiftsError> {
    require_open(tx, shift)?;
    if counts.is_empty() && !forced {
        return Err(ShiftsError::Invalid("count_missing"));
    }
    let expected_cash = rules::expected_cash(&store::load_movements(tx, shift)?)?;
    let counted_cash = if counts.is_empty() {
        None
    } else {
        Some(rules::count_total(counts)?)
    };
    let difference = match counted_cash {
        Some(counted) => Some(rules::difference(counted, expected_cash)?),
        None => None,
    };
    store::insert_counts(tx, shift, Stage::Close, counts)?;
    let figures = ClosedFigures {
        closed_at: now,
        closed_by: by,
        forced,
        expected_cash,
        counted_cash,
        difference,
        note,
    };
    store::mark_closed(tx, shift, figures)?;
    Ok(CloseResult {
        expected_cash,
        counted_cash,
        difference,
    })
}

/// Loads a shift, or says it doesn't exist.
pub(crate) fn require_shift(conn: &Connection, shift: &Id) -> Result<Shift, ShiftsError> {
    store::load_shift(conn, shift)?.ok_or_else(|| ShiftsError::NotFound(shift.clone()))
}

/// Loads a shift and checks it is still open, because closed shifts never change.
pub(crate) fn require_open(conn: &Connection, shift: &Id) -> Result<Shift, ShiftsError> {
    let found = require_shift(conn, shift)?;
    if found.status != ShiftStatus::Open {
        return Err(ShiftsError::Closed(shift.clone()));
    }
    Ok(found)
}
