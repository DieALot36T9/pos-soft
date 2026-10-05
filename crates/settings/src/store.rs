//! Reading and saving one setting row.

use pos_core::{Id, Timestamp};
use rusqlite::{Connection, OptionalExtension, Transaction, named_params};

use crate::{Result, Setting, SettingsError};

/// Reads one setting. Gives the default when the shop hasn't saved it yet.
pub fn get<S: Setting>(conn: &Connection) -> Result<S> {
    match read_json(conn, S::KEY)? {
        Some(json) => decode(S::KEY, &json),
        None => Ok(S::default_value()),
    }
}

/// Checks and saves one setting. Returns the value it replaced (`None` if there was none),
/// so `app` can write the old and new values to the audit log (rule SET-13).
pub fn set<S: Setting>(
    tx: &Transaction,
    value: &S,
    by: Option<&Id>,
    now: Timestamp,
) -> Result<Option<S>> {
    value.validate().map_err(SettingsError::Invalid)?;
    let old = match read_json(tx, S::KEY)? {
        Some(json) => Some(decode(S::KEY, &json)?),
        None => None,
    };
    let json = serde_json::to_string(value).map_err(|error| SettingsError::Unreadable {
        key: S::KEY.to_string(),
        detail: error.to_string(),
    })?;
    tx.execute(
        "INSERT INTO settings_values (key, value_json, updated_at, updated_by)
         VALUES (:key, :value_json, :updated_at, :updated_by)
         ON CONFLICT (key) DO UPDATE SET
           value_json = excluded.value_json,
           updated_at = excluded.updated_at,
           updated_by = excluded.updated_by",
        named_params! {
            ":key": S::KEY,
            ":value_json": json,
            ":updated_at": now.0,
            ":updated_by": by.map(Id::as_str),
        },
    )?;
    Ok(old)
}

/// The stored JSON for one key, if the shop has saved it.
fn read_json(conn: &Connection, key: &str) -> Result<Option<String>> {
    let json = conn
        .query_row(
            "SELECT value_json FROM settings_values WHERE key = :key",
            named_params! { ":key": key },
            |row| row.get(0),
        )
        .optional()?;
    Ok(json)
}

/// Turns stored JSON back into a setting. Fields missing from older saves get their
/// defaults through `#[serde(default)]` on each struct.
pub(crate) fn decode<S: Setting>(key: &str, json: &str) -> Result<S> {
    serde_json::from_str(json).map_err(|error| SettingsError::Unreadable {
        key: key.to_string(),
        detail: error.to_string(),
    })
}
