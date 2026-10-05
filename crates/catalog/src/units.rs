//! Units products are counted or weighed in (rule CAT-02).

use pos_core::{Id, Timestamp};
use rusqlite::{Connection, Row, Transaction, named_params};

use crate::checks::check_exists;
use crate::error::{CatalogError, conflict, invalid};
use crate::rows::{clean_name, clean_optional, read_id, read_optional_time};
use crate::types::{Unit, UnitFields};

/// Adds a unit, such as "dozen" or "box", with 0 to 3 decimals.
pub fn create_unit(tx: &Transaction, fields: &UnitFields) -> Result<Id, CatalogError> {
    let (name, symbol) = check_fields(tx, fields, None)?;
    let id = Id::new();
    tx.execute(
        "INSERT INTO catalog_units (id, name, symbol, unit_code, decimals, is_system)
         VALUES (:id, :name, :symbol, :unit_code, :decimals, 0)",
        named_params! {
            ":id": id.as_str(),
            ":name": name,
            ":symbol": symbol,
            ":unit_code": clean_optional(fields.unit_code.as_deref()),
            ":decimals": fields.decimals,
        },
    )?;
    Ok(id)
}

/// Changes a unit. Its decimals can't change while any product uses it, because
/// quantities already saved would change meaning.
pub fn update_unit(
    tx: &Transaction,
    unit_id: &Id,
    fields: &UnitFields,
) -> Result<(), CatalogError> {
    check_exists(tx, "catalog_units", "unit", unit_id)?;
    let (name, symbol) = check_fields(tx, fields, Some(unit_id))?;
    let (old_decimals, is_system): (u8, bool) = tx.query_row(
        "SELECT decimals, is_system FROM catalog_units WHERE id = :id",
        named_params! { ":id": unit_id.as_str() },
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if old_decimals != fields.decimals {
        if is_system {
            return Err(conflict("unit_is_built_in", name));
        }
        if products_using(tx, unit_id, false)? > 0 {
            return Err(conflict("unit_in_use", name));
        }
    }
    tx.execute(
        "UPDATE catalog_units SET name = :name, symbol = :symbol, unit_code = :unit_code,
            decimals = :decimals WHERE id = :id",
        named_params! {
            ":id": unit_id.as_str(),
            ":name": name,
            ":symbol": symbol,
            ":unit_code": clean_optional(fields.unit_code.as_deref()),
            ":decimals": fields.decimals,
        },
    )?;
    Ok(())
}

/// Hides a unit from new products. Built-in units and units used by an active
/// product can't be archived.
pub fn archive_unit(tx: &Transaction, unit_id: &Id, now: Timestamp) -> Result<(), CatalogError> {
    check_exists(tx, "catalog_units", "unit", unit_id)?;
    let (name, is_system): (String, bool) = tx.query_row(
        "SELECT name, is_system FROM catalog_units WHERE id = :id",
        named_params! { ":id": unit_id.as_str() },
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if is_system {
        return Err(conflict("unit_is_built_in", name));
    }
    if products_using(tx, unit_id, true)? > 0 {
        return Err(conflict("unit_in_use", name));
    }
    tx.execute(
        "UPDATE catalog_units SET archived_at = :now WHERE id = :id",
        named_params! { ":id": unit_id.as_str(), ":now": now.0 },
    )?;
    Ok(())
}

/// Every unit, sorted by name. Archived units only when `include_archived` is true.
pub fn list_units(
    connection: &Connection,
    include_archived: bool,
) -> Result<Vec<Unit>, CatalogError> {
    let units = connection
        .prepare_cached(
            "SELECT id, name, symbol, unit_code, decimals, is_system, archived_at FROM catalog_units
             WHERE :include_archived = 1 OR archived_at IS NULL ORDER BY name",
        )?
        .query_map(named_params! { ":include_archived": include_archived }, read_unit)?
        .collect::<Result<_, _>>()?;
    Ok(units)
}

fn read_unit(row: &Row) -> rusqlite::Result<Unit> {
    Ok(Unit {
        id: read_id(row, 0)?,
        name: row.get(1)?,
        symbol: row.get(2)?,
        unit_code: row.get(3)?,
        decimals: row.get(4)?,
        is_system: row.get(5)?,
        archived_at: read_optional_time(row, 6)?,
    })
}

/// Checks the typed fields and returns the cleaned name and symbol.
fn check_fields(
    tx: &Transaction,
    fields: &UnitFields,
    except: Option<&Id>,
) -> Result<(String, String), CatalogError> {
    let name = clean_name(&fields.name)?;
    let symbol = clean_optional(Some(&fields.symbol)).ok_or(invalid("symbol", "symbol_missing"))?;
    if fields.decimals > 3 {
        return Err(invalid("decimals", "unit_decimals_invalid"));
    }
    let same_name: i64 = tx.query_row(
        "SELECT count(*) FROM catalog_units WHERE name = :name AND id <> :except",
        named_params! { ":name": name, ":except": except.map_or("", Id::as_str) },
        |row| row.get(0),
    )?;
    if same_name > 0 {
        return Err(conflict("unit_name_in_use", name));
    }
    Ok((name, symbol))
}

/// How many products use the unit; only active ones when `active_only` is true.
fn products_using(tx: &Transaction, unit_id: &Id, active_only: bool) -> Result<i64, CatalogError> {
    let count = tx.query_row(
        "SELECT count(*) FROM catalog_products
         WHERE unit_id = :id AND (:active_only = 0 OR archived_at IS NULL)",
        named_params! { ":id": unit_id.as_str(), ":active_only": active_only },
        |row| row.get(0),
    )?;
    Ok(count)
}
