//! Reading and writing this module's tables. No rules here, only SQL.

use pos_core::{BusinessDate, Id, Money, Timestamp};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};

use crate::ShiftsError;
use crate::model::{
    DenominationCount, MovementKind, Shift, ShiftMovement, ShiftStatus, stored_value_invalid,
};

/// Which count: the one when the shift opened or the one when it closed.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Stage {
    Open,
    Close,
}

impl Stage {
    fn as_str(self) -> &'static str {
        match self {
            Stage::Open => "open",
            Stage::Close => "close",
        }
    }
}

/// The figures saved when a shift closes.
pub(crate) struct ClosedFigures<'a> {
    pub closed_at: Timestamp,
    pub closed_by: &'a Id,
    pub forced: bool,
    pub expected_cash: Money,
    pub counted_cash: Option<Money>,
    pub difference: Option<Money>,
    pub note: Option<String>,
}

/// Adds a new open shift. The unique index refuses a second open shift on the till.
pub(crate) fn insert_shift(
    tx: &Transaction,
    id: &Id,
    till: &Id,
    user: &Id,
    float: Money,
    business_date: BusinessDate,
    now: Timestamp,
) -> Result<(), ShiftsError> {
    tx.execute(
        "INSERT INTO shifts_shifts
           (id, till_id, user_id, business_date, status, opened_at, opening_float_minor)
         VALUES (:id, :till_id, :user_id, :business_date, 'open', :opened_at, :float)",
        named_params! {
            ":id": id.as_str(),
            ":till_id": till.as_str(),
            ":user_id": user.as_str(),
            ":business_date": business_date.to_string(),
            ":opened_at": now.0,
            ":float": float.0,
        },
    )?;
    Ok(())
}

/// The id of the open shift on a till, if there is one.
pub(crate) fn open_shift_id(conn: &Connection, till: &Id) -> Result<Option<Id>, ShiftsError> {
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM shifts_shifts WHERE till_id = :till_id AND status = 'open'",
            named_params! { ":till_id": till.as_str() },
            |row| row.get(0),
        )
        .optional()?;
    match id {
        Some(text) => Ok(Some(stored_id(&text)?)),
        None => Ok(None),
    }
}

/// One shift, or nothing if no shift has this id.
pub(crate) fn load_shift(conn: &Connection, id: &Id) -> Result<Option<Shift>, ShiftsError> {
    let raw = conn
        .query_row(
            "SELECT id, till_id, user_id, business_date, status, opened_at, opening_float_minor,
                    closed_at, closed_by, forced, expected_cash_minor, counted_cash_minor,
                    difference_minor, close_note
             FROM shifts_shifts WHERE id = :id",
            named_params! { ":id": id.as_str() },
            RawShift::read,
        )
        .optional()?;
    match raw {
        Some(raw) => Ok(Some(raw.into_shift()?)),
        None => Ok(None),
    }
}

/// The newest shifts first, for everyone or for one user.
pub(crate) fn list_shifts(
    conn: &Connection,
    user: Option<&Id>,
    limit: u32,
) -> Result<Vec<Shift>, ShiftsError> {
    let mut statement = conn.prepare(
        "SELECT id, till_id, user_id, business_date, status, opened_at, opening_float_minor,
                closed_at, closed_by, forced, expected_cash_minor, counted_cash_minor,
                difference_minor, close_note
         FROM shifts_shifts
         WHERE :user_id IS NULL OR user_id = :user_id
         ORDER BY opened_at DESC, id DESC
         LIMIT :limit",
    )?;
    let raws = statement
        .query_map(
            named_params! { ":user_id": user.map(Id::as_str), ":limit": limit },
            RawShift::read,
        )?
        .collect::<Result<Vec<RawShift>, rusqlite::Error>>()?;
    raws.into_iter().map(RawShift::into_shift).collect()
}

/// Saves the close figures and marks the shift closed.
pub(crate) fn mark_closed(
    tx: &Transaction,
    id: &Id,
    figures: ClosedFigures,
) -> Result<(), ShiftsError> {
    tx.execute(
        "UPDATE shifts_shifts
         SET status = 'closed', closed_at = :closed_at, closed_by = :closed_by, forced = :forced,
             expected_cash_minor = :expected, counted_cash_minor = :counted,
             difference_minor = :difference, close_note = :note
         WHERE id = :id AND status = 'open'",
        named_params! {
            ":id": id.as_str(),
            ":closed_at": figures.closed_at.0,
            ":closed_by": figures.closed_by.as_str(),
            ":forced": figures.forced,
            ":expected": figures.expected_cash.0,
            ":counted": figures.counted_cash.map(|money| money.0),
            ":difference": figures.difference.map(|money| money.0),
            ":note": figures.note,
        },
    )?;
    Ok(())
}

/// Adds one movement row. Movements are never changed or removed.
pub(crate) fn insert_movement(
    tx: &Transaction,
    shift: &Id,
    movement: &ShiftMovement,
) -> Result<(), ShiftsError> {
    tx.execute(
        "INSERT INTO shifts_movements
           (id, shift_id, kind, method_id, method_kind, amount_minor, ref_type, ref_id, reason,
            at, user_id)
         VALUES (:id, :shift_id, :kind, :method_id, :method_kind, :amount, :ref_type, :ref_id,
                 :reason, :at, :user_id)",
        named_params! {
            ":id": Id::new().as_str(),
            ":shift_id": shift.as_str(),
            ":kind": movement.kind.as_str(),
            ":method_id": movement.method_id.as_ref().map(Id::as_str),
            ":method_kind": movement.method_kind,
            ":amount": movement.amount.0,
            ":ref_type": movement.ref_type,
            ":ref_id": movement.ref_id.as_ref().map(Id::as_str),
            ":reason": movement.reason,
            ":at": movement.at.0,
            ":user_id": movement.user_id.as_str(),
        },
    )?;
    Ok(())
}

const MOVEMENT_COLUMNS: &str =
    "kind, method_id, method_kind, amount_minor, ref_type, ref_id, reason, at, user_id";

/// Every movement of a shift, oldest first.
pub(crate) fn load_movements(
    conn: &Connection,
    shift: &Id,
) -> Result<Vec<ShiftMovement>, ShiftsError> {
    let sql = format!(
        "SELECT {MOVEMENT_COLUMNS} FROM shifts_movements WHERE shift_id = :shift_id ORDER BY rowid"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(
        named_params! { ":shift_id": shift.as_str() },
        RawMovement::read,
    )?;
    let mut movements = Vec::new();
    for raw in rows {
        movements.push(raw?.into_movement()?);
    }
    Ok(movements)
}

/// The movements of one document in a shift, oldest first.
pub(crate) fn load_movements_for_ref(
    conn: &Connection,
    shift: &Id,
    ref_type: &str,
    ref_id: &Id,
) -> Result<Vec<ShiftMovement>, ShiftsError> {
    let sql = format!(
        "SELECT {MOVEMENT_COLUMNS} FROM shifts_movements
         WHERE shift_id = :shift_id AND ref_type = :ref_type AND ref_id = :ref_id ORDER BY rowid"
    );
    let mut statement = conn.prepare(&sql)?;
    let params = named_params! {
        ":shift_id": shift.as_str(),
        ":ref_type": ref_type,
        ":ref_id": ref_id.as_str(),
    };
    let rows = statement.query_map(params, RawMovement::read)?;
    let mut movements = Vec::new();
    for raw in rows {
        movements.push(raw?.into_movement()?);
    }
    Ok(movements)
}

/// Saves a denomination count. Example: 12 × 1,000.00 at close.
pub(crate) fn insert_counts(
    tx: &Transaction,
    shift: &Id,
    stage: Stage,
    counts: &[(Money, u32)],
) -> Result<(), ShiftsError> {
    let mut statement = tx.prepare(
        "INSERT INTO shifts_counts (shift_id, stage, denomination_minor, count)
         VALUES (:shift_id, :stage, :denomination, :count)",
    )?;
    for &(denomination, count) in counts {
        statement.execute(named_params! {
            ":shift_id": shift.as_str(),
            ":stage": stage.as_str(),
            ":denomination": denomination.0,
            ":count": count,
        })?;
    }
    Ok(())
}

/// A denomination count, smallest denomination first.
pub(crate) fn load_counts(
    conn: &Connection,
    shift: &Id,
    stage: Stage,
) -> Result<Vec<DenominationCount>, ShiftsError> {
    let mut statement = conn.prepare(
        "SELECT denomination_minor, count FROM shifts_counts
         WHERE shift_id = :shift_id AND stage = :stage ORDER BY denomination_minor",
    )?;
    let params = named_params! { ":shift_id": shift.as_str(), ":stage": stage.as_str() };
    let rows = statement.query_map(params, |row| {
        Ok(DenominationCount {
            denomination: Money(row.get(0)?),
            count: row.get(1)?,
        })
    })?;
    let mut counts = Vec::new();
    for count in rows {
        counts.push(count?);
    }
    Ok(counts)
}

/// A shift row as SQLite gives it, before the text columns are checked.
struct RawShift {
    id: String,
    till_id: String,
    user_id: String,
    business_date: String,
    status: String,
    opened_at: i64,
    opening_float: i64,
    closed_at: Option<i64>,
    closed_by: Option<String>,
    forced: bool,
    expected_cash: Option<i64>,
    counted_cash: Option<i64>,
    difference: Option<i64>,
    close_note: Option<String>,
}

impl RawShift {
    fn read(row: &Row) -> rusqlite::Result<RawShift> {
        Ok(RawShift {
            id: row.get(0)?,
            till_id: row.get(1)?,
            user_id: row.get(2)?,
            business_date: row.get(3)?,
            status: row.get(4)?,
            opened_at: row.get(5)?,
            opening_float: row.get(6)?,
            closed_at: row.get(7)?,
            closed_by: row.get(8)?,
            forced: row.get(9)?,
            expected_cash: row.get(10)?,
            counted_cash: row.get(11)?,
            difference: row.get(12)?,
            close_note: row.get(13)?,
        })
    }

    fn into_shift(self) -> Result<Shift, ShiftsError> {
        Ok(Shift {
            id: stored_id(&self.id)?,
            till_id: stored_id(&self.till_id)?,
            user_id: stored_id(&self.user_id)?,
            business_date: BusinessDate::parse(&self.business_date)
                .map_err(|_| stored_value_invalid("business_date", &self.business_date))?,
            status: ShiftStatus::parse(&self.status)?,
            opened_at: Timestamp(self.opened_at),
            opening_float: Money(self.opening_float),
            closed_at: self.closed_at.map(Timestamp),
            closed_by: optional_id(self.closed_by)?,
            forced: self.forced,
            expected_cash: self.expected_cash.map(Money),
            counted_cash: self.counted_cash.map(Money),
            difference: self.difference.map(Money),
            close_note: self.close_note,
        })
    }
}

/// A movement row as SQLite gives it, before the text columns are checked.
struct RawMovement {
    kind: String,
    method_id: Option<String>,
    method_kind: String,
    amount: i64,
    ref_type: Option<String>,
    ref_id: Option<String>,
    reason: Option<String>,
    at: i64,
    user_id: String,
}

impl RawMovement {
    fn read(row: &Row) -> rusqlite::Result<RawMovement> {
        Ok(RawMovement {
            kind: row.get(0)?,
            method_id: row.get(1)?,
            method_kind: row.get(2)?,
            amount: row.get(3)?,
            ref_type: row.get(4)?,
            ref_id: row.get(5)?,
            reason: row.get(6)?,
            at: row.get(7)?,
            user_id: row.get(8)?,
        })
    }

    fn into_movement(self) -> Result<ShiftMovement, ShiftsError> {
        Ok(ShiftMovement {
            kind: MovementKind::parse(&self.kind)?,
            method_id: optional_id(self.method_id)?,
            method_kind: self.method_kind,
            amount: Money(self.amount),
            ref_type: self.ref_type,
            ref_id: optional_id(self.ref_id)?,
            reason: self.reason,
            at: Timestamp(self.at),
            user_id: stored_id(&self.user_id)?,
        })
    }
}

/// An id read back from the database. A bad one is a bug, not a typing mistake.
fn stored_id(text: &str) -> Result<Id, ShiftsError> {
    Id::parse(text).map_err(|_| stored_value_invalid("id", text))
}

fn optional_id(text: Option<String>) -> Result<Option<Id>, ShiftsError> {
    match text {
        Some(text) => Ok(Some(stored_id(&text)?)),
        None => Ok(None),
    }
}
