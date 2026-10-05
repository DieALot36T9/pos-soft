//! Saving and reading tax rates. Rates are never deleted, only deactivated (rule TAX-10).

use std::collections::HashMap;

use pos_core::{Bps, Id, Timestamp};
use rusqlite::{Connection, OptionalExtension, Transaction, named_params};

use crate::{NewRate, RateComponent, RateUpdate, RateWithComponents, Result, TaxError};

/// The largest rate: 10000 bps is 100%.
const MAX_RATE_BPS: u32 = 10_000;

/// Adds a new, active tax rate and returns its id (rules TAX-02, TAX-03).
pub fn create_rate(tx: &Transaction, new_rate: NewRate, now: Timestamp) -> Result<Id> {
    let (name, components) = check_rate(&new_rate.name, new_rate.rate_bps, &new_rate.components)?;
    check_name_is_free(tx, &name, None)?;
    let id = Id::new();
    tx.execute(
        "INSERT INTO tax_rates (id, name, rate_bps, active, sort, created_at, updated_at)
         VALUES (:id, :name, :rate_bps, 1, :sort, :now, :now)",
        named_params! {
            ":id": id.as_str(),
            ":name": name,
            ":rate_bps": new_rate.rate_bps.0,
            ":sort": new_rate.sort,
            ":now": now.0,
        },
    )?;
    insert_components(tx, &id, &components)?;
    Ok(id)
}

/// Changes a rate's name, percentage, parts and sort. Past sales keep their stored
/// rate, so this affects only future sales (rule TAX-07).
pub fn update_rate(tx: &Transaction, id: &Id, update: RateUpdate, now: Timestamp) -> Result<()> {
    let (name, components) = check_rate(&update.name, update.rate_bps, &update.components)?;
    check_name_is_free(tx, &name, Some(id))?;
    let changed = tx.execute(
        "UPDATE tax_rates SET name = :name, rate_bps = :rate_bps, sort = :sort, updated_at = :now
         WHERE id = :id",
        named_params! {
            ":id": id.as_str(),
            ":name": name,
            ":rate_bps": update.rate_bps.0,
            ":sort": update.sort,
            ":now": now.0,
        },
    )?;
    if changed == 0 {
        return Err(TaxError::NotFound { id: id.clone() });
    }
    // Components are display details of the rate, not a ledger, so they are replaced.
    tx.execute(
        "DELETE FROM tax_rate_components WHERE tax_rate_id = :id",
        named_params! { ":id": id.as_str() },
    )?;
    insert_components(tx, id, &components)?;
    Ok(())
}

/// Stops a rate being chosen for new products. It stays for past sales (rule TAX-10).
pub fn deactivate(tx: &Transaction, id: &Id, now: Timestamp) -> Result<()> {
    let changed = tx.execute(
        "UPDATE tax_rates SET active = 0, updated_at = :now WHERE id = :id",
        named_params! { ":id": id.as_str(), ":now": now.0 },
    )?;
    if changed == 0 {
        return Err(TaxError::NotFound { id: id.clone() });
    }
    Ok(())
}

/// Reads one rate with its parts, active or not.
pub fn get(conn: &Connection, id: &Id) -> Result<RateWithComponents> {
    match load_rate(conn, id)? {
        Some(rate) => Ok(rate),
        None => Err(TaxError::NotFound { id: id.clone() }),
    }
}

/// Reads several rates at once. Ids with no rate are simply missing from the answer,
/// so the caller can tell which ones don't exist.
pub fn get_many(conn: &Connection, ids: &[Id]) -> Result<HashMap<Id, RateWithComponents>> {
    let mut found = HashMap::new();
    for id in ids {
        if let Some(rate) = load_rate(conn, id)? {
            found.insert(id.clone(), rate);
        }
    }
    Ok(found)
}

/// Lists the rates in screen order (sort, then name). Deactivated ones only when asked.
pub fn list(conn: &Connection, include_inactive: bool) -> Result<Vec<RateWithComponents>> {
    let mut statement = conn.prepare(
        "SELECT id FROM tax_rates WHERE active = 1 OR :include_inactive
         ORDER BY sort, name",
    )?;
    let ids = statement
        .query_map(
            named_params! { ":include_inactive": include_inactive },
            |row| row.get::<_, String>(0),
        )?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    let mut rates = Vec::with_capacity(ids.len());
    for id in ids {
        rates.push(get(conn, &Id::parse(&id)?)?);
    }
    Ok(rates)
}

/// Checks what the admin typed and returns the name and parts with spaces trimmed.
fn check_rate(
    name: &str,
    rate_bps: Bps,
    components: &[RateComponent],
) -> Result<(String, Vec<RateComponent>)> {
    let name = name.trim();
    if name.is_empty() {
        return Err(TaxError::NameMissing);
    }
    if rate_bps.0 > MAX_RATE_BPS {
        return Err(TaxError::RateOutOfRange);
    }
    let mut trimmed = Vec::with_capacity(components.len());
    // u64, so adding many parts can never overflow.
    let mut parts_total: u64 = 0;
    for component in components {
        let component_name = component.name.trim();
        if component_name.is_empty() {
            return Err(TaxError::ComponentNameMissing);
        }
        parts_total += u64::from(component.rate_bps.0);
        trimmed.push(RateComponent {
            name: component_name.to_string(),
            rate_bps: component.rate_bps,
        });
    }
    if !trimmed.is_empty() && parts_total != u64::from(rate_bps.0) {
        return Err(TaxError::ComponentsDoNotAddUp);
    }
    Ok((name.to_string(), trimmed))
}

/// Refuses a name another rate already uses. Capitals are ignored, because
/// "Standard" and "standard" would look like the same rate to the cashier.
fn check_name_is_free(tx: &Transaction, name: &str, own_id: Option<&Id>) -> Result<()> {
    let own_id = own_id.map(Id::as_str).unwrap_or("");
    let clash: Option<String> = tx
        .query_row(
            "SELECT id FROM tax_rates WHERE name = :name COLLATE NOCASE AND id != :own_id",
            named_params! { ":name": name, ":own_id": own_id },
            |row| row.get(0),
        )
        .optional()?;
    match clash {
        Some(_) => Err(TaxError::NameTaken),
        None => Ok(()),
    }
}

fn insert_components(tx: &Transaction, rate_id: &Id, components: &[RateComponent]) -> Result<()> {
    // Count positions as i64, the type SQLite stores.
    for (position, component) in (0_i64..).zip(components) {
        tx.execute(
            "INSERT INTO tax_rate_components (id, tax_rate_id, name, rate_bps, sort)
             VALUES (:id, :tax_rate_id, :name, :rate_bps, :sort)",
            named_params! {
                ":id": Id::new().as_str(),
                ":tax_rate_id": rate_id.as_str(),
                ":name": component.name,
                ":rate_bps": component.rate_bps.0,
                ":sort": position,
            },
        )?;
    }
    Ok(())
}

/// Reads one rate and its parts, or None if no rate has this id.
fn load_rate(conn: &Connection, id: &Id) -> Result<Option<RateWithComponents>> {
    let row = conn
        .query_row(
            "SELECT name, rate_bps, active, sort FROM tax_rates WHERE id = :id",
            named_params! { ":id": id.as_str() },
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((name, rate_bps, active, sort)) = row else {
        return Ok(None);
    };
    Ok(Some(RateWithComponents {
        id: id.clone(),
        name,
        rate_bps: Bps(rate_bps),
        active,
        sort,
        components: load_components(conn, id)?,
    }))
}

fn load_components(conn: &Connection, rate_id: &Id) -> Result<Vec<RateComponent>> {
    let mut statement = conn.prepare(
        "SELECT name, rate_bps FROM tax_rate_components WHERE tax_rate_id = :id ORDER BY sort",
    )?;
    let components = statement
        .query_map(named_params! { ":id": rate_id.as_str() }, |row| {
            Ok(RateComponent {
                name: row.get(0)?,
                rate_bps: Bps(row.get(1)?),
            })
        })?
        .collect::<rusqlite::Result<Vec<RateComponent>>>()?;
    Ok(components)
}
