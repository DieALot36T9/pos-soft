//! Reasons for stock adjustments (INV-02). The built-in ones come with the app and
//! can't be switched off. Shops add their own. Nothing is ever deleted, because old
//! ledger rows still name their reason.

use pos_core::Qty;
use rusqlite::{Connection, OptionalExtension, Transaction, named_params};
use serde::{Deserialize, Serialize};

use crate::InventoryError;

/// Which way a reason moves stock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonDirection {
    In,
    Out,
    Both,
}

impl ReasonDirection {
    /// The text stored in the database: "in", "out" or "both".
    pub fn as_str(self) -> &'static str {
        match self {
            ReasonDirection::In => "in",
            ReasonDirection::Out => "out",
            ReasonDirection::Both => "both",
        }
    }

    /// Reads the stored text back.
    pub fn parse(text: &str) -> Result<ReasonDirection, InventoryError> {
        match text {
            "in" => Ok(ReasonDirection::In),
            "out" => Ok(ReasonDirection::Out),
            "both" => Ok(ReasonDirection::Both),
            _ => Err(InventoryError::BadData(format!(
                "unknown reason direction '{text}'"
            ))),
        }
    }

    /// Whether a change of this size fits. "damage" (out) only fits stock going out.
    pub fn fits(self, qty_delta: Qty) -> bool {
        match self {
            ReasonDirection::In => qty_delta.0 > 0,
            ReasonDirection::Out => qty_delta.0 < 0,
            ReasonDirection::Both => true,
        }
    }
}

/// One adjustment reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reason {
    pub code: String,
    pub name: String,
    pub direction: ReasonDirection,
    pub is_system: bool,
    pub active: bool,
}

/// Every reason, built-in ones first, then by name.
pub fn list_reasons(connection: &Connection) -> Result<Vec<Reason>, InventoryError> {
    let mut statement = connection.prepare(
        "SELECT code, name, direction, is_system, active FROM inventory_reasons
         ORDER BY is_system DESC, name",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, bool>(3)?,
            row.get::<_, bool>(4)?,
        ))
    })?;
    let mut reasons = Vec::new();
    for row in rows {
        let (code, name, direction, is_system, active) = row?;
        reasons.push(Reason {
            code,
            name,
            direction: ReasonDirection::parse(&direction)?,
            is_system,
            active,
        });
    }
    Ok(reasons)
}

/// Adds a shop's own reason. The code is lower case letters, digits and "_", e.g. "free_sample".
pub fn add_reason(
    transaction: &Transaction,
    code: &str,
    name: &str,
    direction: ReasonDirection,
) -> Result<(), InventoryError> {
    if !is_good_code(code) {
        return Err(InventoryError::Invalid("reason_code_invalid"));
    }
    let name = clean_name(name)?;
    if find(transaction, code)?.is_some() {
        return Err(InventoryError::Conflict("reason_exists"));
    }
    transaction.execute(
        "INSERT INTO inventory_reasons (code, name, direction) VALUES (:code, :name, :direction)",
        named_params! { ":code": code, ":name": name, ":direction": direction.as_str() },
    )?;
    Ok(())
}

/// Changes the name shown for a reason. Its code never changes.
pub fn rename_reason(
    transaction: &Transaction,
    code: &str,
    name: &str,
) -> Result<(), InventoryError> {
    let name = clean_name(name)?;
    let changed = transaction.execute(
        "UPDATE inventory_reasons SET name = :name WHERE code = :code",
        named_params! { ":code": code, ":name": name },
    )?;
    if changed == 0 {
        return Err(not_found(code));
    }
    Ok(())
}

/// Switches a reason on or off. Built-in reasons can't be switched off.
pub fn set_reason_active(
    transaction: &Transaction,
    code: &str,
    active: bool,
) -> Result<(), InventoryError> {
    let Some(is_system) = find(transaction, code)? else {
        return Err(not_found(code));
    };
    if is_system && !active {
        return Err(InventoryError::Conflict("reason_is_built_in"));
    }
    transaction.execute(
        "UPDATE inventory_reasons SET active = :active WHERE code = :code",
        named_params! { ":code": code, ":active": active },
    )?;
    Ok(())
}

/// The direction of an active reason, or nothing if the code is unknown or switched off.
pub(crate) fn active_direction(
    connection: &Connection,
    code: &str,
) -> Result<Option<ReasonDirection>, InventoryError> {
    let direction: Option<String> = connection
        .query_row(
            "SELECT direction FROM inventory_reasons WHERE code = :code AND active = 1",
            named_params! { ":code": code },
            |row| row.get(0),
        )
        .optional()?;
    direction
        .map(|text| ReasonDirection::parse(&text))
        .transpose()
}

/// Whether the reason exists, and if so whether it's built in.
fn find(connection: &Connection, code: &str) -> Result<Option<bool>, InventoryError> {
    let is_system = connection
        .query_row(
            "SELECT is_system FROM inventory_reasons WHERE code = :code",
            named_params! { ":code": code },
            |row| row.get(0),
        )
        .optional()?;
    Ok(is_system)
}

fn is_good_code(code: &str) -> bool {
    let starts_with_letter = code
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_lowercase());
    let allowed =
        |letter: char| letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == '_';
    starts_with_letter && code.len() <= 40 && code.chars().all(allowed)
}

fn clean_name(name: &str) -> Result<&str, InventoryError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(InventoryError::Invalid("reason_name_missing"));
    }
    Ok(name)
}

fn not_found(code: &str) -> InventoryError {
    InventoryError::NotFound {
        entity: "reason",
        key: code.to_string(),
    }
}
