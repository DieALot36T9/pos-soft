//! The till and its open shift, as every till workflow sees them: which shift is open,
//! who owns it, and the error answers the till screens share.

use pos_core::{AppError, BusinessDate, CurrencyFormat, Id, Money, Timestamp};
use rusqlite::Connection;
use serde::Serialize;
use settings::{CheckoutSettings, Denominations, TillSettings};
use shifts::Shift;
use ts_rs::TS;

use crate::state::AppState;

/// The till's open shift, as the screens show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ShiftView {
    pub shift_id: Id,
    pub user_id: Id,
    /// Example: "Rahim".
    pub cashier_name: String,
    pub business_date: BusinessDate,
    pub opened_at: Timestamp,
    pub opening_float: Money,
    /// True when the signed-in user owns this shift and may sell on it (SHF-02).
    pub mine: bool,
}

/// What the shift screens need: the open shift (if any) and how cash is counted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ShiftState {
    pub shift: Option<ShiftView>,
    /// The notes and coins, largest first, for counting by denomination.
    pub denominations: Vec<Money>,
    /// "Count first": the cashier counts before seeing the expected cash (SHF-06).
    pub blind_close: bool,
    /// Cash expected in the drawer now. None while "count first" is on.
    pub expected_cash: Option<Money>,
    pub currency: CurrencyFormat,
}

/// The till's open shift and the counting settings, for the signed-in user.
pub fn state_for(state: &AppState, user: &Id) -> Result<ShiftState, AppError> {
    state.db.read(|conn| {
        let checkout: CheckoutSettings = settings::get(conn)?;
        let denominations: Denominations = settings::get(conn)?;
        let mut largest_first = denominations.0;
        largest_first.sort_by(|a, b| b.cmp(a));
        let shift = till_shift(conn)?;
        let expected_cash = match &shift {
            Some(open) if !checkout.blind_close => Some(shifts::expected_cash(conn, &open.id)?),
            _ => None,
        };
        Ok(ShiftState {
            shift: shift
                .map(|open| shift_view(conn, &open, user))
                .transpose()?,
            denominations: largest_first,
            blind_close: checkout.blind_close,
            expected_cash,
            currency: settings::get(conn)?,
        })
    })
}

/// The open shift on this till, if any.
pub fn till_shift(conn: &Connection) -> Result<Option<Shift>, AppError> {
    let till: TillSettings = settings::get(conn)?;
    match till.till_id {
        Some(till_id) => Ok(shifts::current(conn, &till_id)?),
        None => Ok(None),
    }
}

/// A shift as the screens show it, with the cashier's name.
pub(crate) fn shift_view(
    conn: &Connection,
    shift: &Shift,
    user: &Id,
) -> Result<ShiftView, AppError> {
    let cashier_name = auth::list_users(conn)?
        .into_iter()
        .find(|each| each.id == shift.user_id)
        .map(|each| each.display_name)
        .unwrap_or_default();
    Ok(ShiftView {
        shift_id: shift.id.clone(),
        user_id: shift.user_id.clone(),
        cashier_name,
        business_date: shift.business_date,
        opened_at: shift.opened_at,
        opening_float: shift.opening_float,
        mine: shift.user_id == *user,
    })
}

/// The till's id and settings. Setup always gives the till an id.
pub(crate) struct TillInfo {
    pub id: Id,
    pub till: TillSettings,
}

pub(crate) fn till_settings(conn: &Connection) -> Result<TillInfo, AppError> {
    let till: TillSettings = settings::get(conn)?;
    match till.till_id.clone() {
        Some(id) => Ok(TillInfo { id, till }),
        None => Err(conflict("till_not_set_up")),
    }
}

pub(crate) fn conflict(code: &str) -> AppError {
    AppError::Conflict {
        code: code.to_string(),
        params: Default::default(),
    }
}

pub(crate) fn conflict_with(code: &str, key: &str, value: impl ToString) -> AppError {
    let mut params = std::collections::BTreeMap::new();
    params.insert(key.to_string(), value.to_string());
    AppError::Conflict {
        code: code.to_string(),
        params,
    }
}
