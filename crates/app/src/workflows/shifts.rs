//! Opening and closing shifts, cash in and out, and the no-sale drawer
//! (Req - Shifts and Cash, Workflows "Open / close shift"). Each change is one
//! transaction with its audit event. After the commit the drawer opens for cash in and
//! out and the no-sale, and closing prints the Z report (HW-04, SHF-06).

use pos_core::{AppError, Clock, Id, Money, business_date};
use serde::{Deserialize, Serialize};
use shifts::{MovementKind, Shift, ShiftMovement};

pub use crate::workflows::till::{ShiftState, ShiftView, state_for};
use ts_rs::TS;

use crate::audit_actions::{
    DRAWER_NO_SALE, SHIFT_CASH_MOVE, SHIFT_CLOSE, SHIFT_FORCE_CLOSE, SHIFT_OPEN,
};
use crate::audit_log;
use crate::devices::Devices;
use crate::state::AppState;
use crate::workflows::receipts::{self, PrintResult};
use crate::workflows::till::{conflict, conflict_with, shift_view, till_settings, till_shift};

/// How many of one note or coin were counted. Example: 12 × 500.00.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CountLine {
    pub denomination: Money,
    pub count: u32,
}

/// Money put into or taken out of the drawer during a shift (SHF-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CashMoveKind {
    /// Extra change put in.
    CashIn,
    /// Paid out, for example to a delivery person.
    CashOut,
    /// Taken to the safe.
    CashDrop,
}

/// A cash in or out as typed on the shift menu.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct CashMoveInput {
    pub kind: CashMoveKind,
    /// Always more than zero; the kind decides the direction. Example: 50000 is 500.00.
    pub amount: Money,
    /// Example: "Delivery man, rice". Required.
    pub reason: String,
}

/// What closing a shift found (SHF-06). A shortage is negative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ShiftClosed {
    pub shift_id: Id,
    pub expected_cash: Money,
    pub counted_cash: Option<Money>,
    pub difference: Option<Money>,
    pub forced: bool,
    /// How the Z report printed, after the shift was closed. None only inside the
    /// close functions, before printing.
    pub report: Option<PrintResult>,
}

/// Adds up a count by denomination, so the screen never does money maths.
pub fn count_total(counts: &[CountLine]) -> Result<Money, AppError> {
    let mut total = Money::zero();
    for line in counts {
        let amount = line
            .denomination
            .0
            .checked_mul(i64::from(line.count))
            .ok_or_else(|| AppError::validation("count_too_large"))?;
        total = total.checked_add(Money(amount))?;
    }
    Ok(total)
}

/// Opens a shift on this till for the user (SHF-01, SHF-02). With a count, the float
/// is the count's total; without one, the float as typed.
pub fn open(
    state: &AppState,
    user: &Id,
    float: Money,
    counts: &[CountLine],
) -> Result<ShiftView, AppError> {
    let clock = state.clock.as_ref();
    let counts = counted(counts);
    let float = if counts.is_empty() {
        float
    } else {
        count_total(&as_lines(&counts))?
    };
    state.db.write(|tx| {
        let till = till_settings(tx)?;
        let now = clock.now();
        let date = business_date(
            now,
            clock.local_offset_minutes(),
            till.till.business_day_cutoff_hour,
        )?;
        let id = shifts::open(tx, &till.id, user, float, &counts, date, now)?;
        audit_log::record(
            tx,
            clock,
            SHIFT_OPEN,
            serde_json::json!({ "shift_id": id, "float_minor": float.0 }),
            Some(user.clone()),
        )?;
        let shift = till_shift(tx)?.ok_or_else(|| AppError::internal("the new shift is gone"))?;
        shift_view(tx, &shift, user)
    })
}

/// Records a cash in, paid-out or cash drop on the till's open shift (SHF-03), audited
/// with the admin who approved it, if one had to.
pub fn cash_move(
    state: &AppState,
    user: &Id,
    approved_by: Option<&Id>,
    input: &CashMoveInput,
) -> Result<(), AppError> {
    if input.amount.0 <= 0 {
        return Err(field_error("amount", "amount_not_positive"));
    }
    let reason = input.reason.trim();
    if reason.is_empty() {
        return Err(field_error("reason", "reason_missing"));
    }
    let (kind, signed) = match input.kind {
        CashMoveKind::CashIn => (MovementKind::CashIn, input.amount),
        CashMoveKind::CashOut => (MovementKind::CashOut, input.amount.checked_neg()?),
        CashMoveKind::CashDrop => (MovementKind::CashDrop, input.amount.checked_neg()?),
    };
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let shift = till_shift(tx)?.ok_or_else(|| conflict("shift_not_open"))?;
        shifts::record(
            tx,
            &shift.id,
            ShiftMovement {
                kind,
                method_id: None,
                method_kind: shifts::CASH.to_string(),
                amount: signed,
                ref_type: None,
                ref_id: None,
                reason: Some(reason.to_string()),
                at: clock.now(),
                user_id: user.clone(),
            },
        )?;
        let details = serde_json::json!({
            "shift_id": shift.id, "kind": kind.as_str(), "amount_minor": signed.0, "reason": reason,
        });
        audit_log::record_approved(
            tx,
            clock,
            SHIFT_CASH_MOVE,
            details,
            Some(user.clone()),
            approved_by.cloned(),
        )
    })
}

/// Opening the drawer without a sale (SHF-04): no money moves, so it is only audited.
pub fn no_sale(
    state: &AppState,
    user: &Id,
    approved_by: Option<&Id>,
    reason: &str,
) -> Result<(), AppError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(field_error("reason", "reason_missing"));
    }
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let shift = till_shift(tx)?.ok_or_else(|| conflict("shift_not_open"))?;
        audit_log::record_approved(
            tx,
            clock,
            DRAWER_NO_SALE,
            serde_json::json!({ "shift_id": shift.id, "reason": reason }),
            Some(user.clone()),
            approved_by.cloned(),
        )
    })
}

/// The cashier closes their own shift with the cash count (SHF-06). Refused while a
/// cart with items is open on it; an empty cart is simply thrown away.
pub fn close(
    state: &AppState,
    user: &Id,
    counts: &[CountLine],
    note: Option<&str>,
) -> Result<ShiftClosed, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let shift = till_shift(tx)?.ok_or_else(|| conflict("shift_not_open"))?;
        if shift.user_id != *user {
            return Err(conflict("shift_not_yours"));
        }
        let open_carts = sales::open_carts_on_shift(tx, &shift.id)?;
        if open_carts > 0 {
            return Err(conflict_with("shift_has_open_carts", "count", open_carts));
        }
        drop_empty_cart(tx, &shift.till_id, user)?;
        let result = shifts::close(
            tx,
            &shift.id,
            &counted(counts),
            user,
            false,
            blank_to_none(note),
            clock.now(),
        )?;
        let closed = closed_view(&shift, result, false);
        audit_close(tx, clock, SHIFT_CLOSE, user, None, &closed)?;
        Ok(closed)
    })
}

/// An admin closes the shift left open on this till (SHF-07). The count is optional.
/// The owner's cart is held, so any cashier can bring it back later; an empty one goes.
pub fn force_close(
    state: &AppState,
    user: &Id,
    approved_by: Option<&Id>,
    counts: &[CountLine],
    note: Option<&str>,
) -> Result<ShiftClosed, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let shift = till_shift(tx)?.ok_or_else(|| conflict("shift_not_open"))?;
        if let Some(cart) = sales::current_cart(tx, &shift.till_id, &shift.user_id)? {
            if cart.lines.is_empty() {
                sales::cancel(tx, &cart.id)?;
            } else {
                sales::hold(tx, &cart.id, None, clock.now())?;
            }
        }
        let result = shifts::close(
            tx,
            &shift.id,
            &counted(counts),
            user,
            true,
            blank_to_none(note),
            clock.now(),
        )?;
        let closed = closed_view(&shift, result, true);
        audit_close(tx, clock, SHIFT_FORCE_CLOSE, user, approved_by, &closed)?;
        Ok(closed)
    })
}

/// Cash in, paid-out or cash drop, then the drawer opens (SHF-03, HW-04).
pub fn cash_move_and_open(
    state: &AppState,
    devices: &Devices,
    user: &Id,
    approved_by: Option<&Id>,
    input: &CashMoveInput,
) -> Result<PrintResult, AppError> {
    cash_move(state, user, approved_by, input)?;
    Ok(receipts::open_drawer(state, devices))
}

/// The no-sale, then the drawer opens (SHF-04, HW-04).
pub fn no_sale_and_open(
    state: &AppState,
    devices: &Devices,
    user: &Id,
    approved_by: Option<&Id>,
    reason: &str,
) -> Result<PrintResult, AppError> {
    no_sale(state, user, approved_by, reason)?;
    Ok(receipts::open_drawer(state, devices))
}

/// Closes the user's shift, then prints the Z report (SHF-06).
pub fn close_and_print(
    state: &AppState,
    devices: &Devices,
    user: &Id,
    counts: &[CountLine],
    note: Option<&str>,
) -> Result<ShiftClosed, AppError> {
    let mut closed = close(state, user, counts, note)?;
    closed.report = Some(receipts::print_shift_report(
        state,
        devices,
        &closed.shift_id,
    ));
    Ok(closed)
}

/// Force-closes the till's shift, then prints the Z report (SHF-07).
pub fn force_close_and_print(
    state: &AppState,
    devices: &Devices,
    user: &Id,
    approved_by: Option<&Id>,
    counts: &[CountLine],
    note: Option<&str>,
) -> Result<ShiftClosed, AppError> {
    let mut closed = force_close(state, user, approved_by, counts, note)?;
    closed.report = Some(receipts::print_shift_report(
        state,
        devices,
        &closed.shift_id,
    ));
    Ok(closed)
}

/// Throws away the user's open cart on this till when it has no items, so a closed
/// shift never leaves an empty cart behind.
fn drop_empty_cart(conn: &rusqlite::Transaction, till_id: &Id, user: &Id) -> Result<(), AppError> {
    if let Some(cart) = sales::current_cart(conn, till_id, user)?
        && cart.lines.is_empty()
    {
        sales::cancel(conn, &cart.id)?;
    }
    Ok(())
}

fn audit_close(
    tx: &rusqlite::Transaction,
    clock: &dyn Clock,
    action: &'static str,
    user: &Id,
    approved_by: Option<&Id>,
    closed: &ShiftClosed,
) -> Result<(), AppError> {
    let details = serde_json::json!({
        "shift_id": closed.shift_id,
        "expected_minor": closed.expected_cash.0,
        "counted_minor": closed.counted_cash.map(|money| money.0),
        "difference_minor": closed.difference.map(|money| money.0),
    });
    audit_log::record_approved(
        tx,
        clock,
        action,
        details,
        Some(user.clone()),
        approved_by.cloned(),
    )
}

fn closed_view(shift: &Shift, result: shifts::CloseResult, forced: bool) -> ShiftClosed {
    ShiftClosed {
        shift_id: shift.id.clone(),
        expected_cash: result.expected_cash,
        counted_cash: result.counted_cash,
        difference: result.difference,
        forced,
        report: None,
    }
}

/// The count lines with at least one note or coin, as the shifts module takes them.
fn counted(counts: &[CountLine]) -> Vec<(Money, u32)> {
    counts
        .iter()
        .filter(|line| line.count > 0)
        .map(|line| (line.denomination, line.count))
        .collect()
}

fn as_lines(counts: &[(Money, u32)]) -> Vec<CountLine> {
    counts
        .iter()
        .map(|(denomination, count)| CountLine {
            denomination: *denomination,
            count: *count,
        })
        .collect()
}

fn blank_to_none(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn field_error(field: &str, code: &str) -> AppError {
    AppError::Validation {
        field: Some(field.to_string()),
        code: code.to_string(),
        params: Default::default(),
    }
}
