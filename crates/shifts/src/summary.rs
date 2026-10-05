//! Shift totals for the X and Z reports.

use pos_core::Id;
use rusqlite::Connection;

use crate::model::ShiftSummary;
use crate::shift::require_shift;
use crate::store::{self, Stage};
use crate::{ShiftsError, rules};

/// Totals by kind and method, payments by method, expected cash and the counts (SHF-08).
/// Works for open shifts (X report) and closed ones (Z report).
pub fn summary(conn: &Connection, shift: &Id) -> Result<ShiftSummary, ShiftsError> {
    let found = require_shift(conn, shift)?;
    let movements = store::load_movements(conn, shift)?;
    Ok(ShiftSummary {
        shift: found,
        totals: rules::totals(&movements)?,
        payments_by_method: rules::payments_by_method(&movements)?,
        expected_cash: rules::expected_cash(&movements)?,
        open_counts: store::load_counts(conn, shift, Stage::Open)?,
        close_counts: store::load_counts(conn, shift, Stage::Close)?,
    })
}
