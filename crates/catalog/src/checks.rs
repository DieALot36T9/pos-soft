//! Checks that a unit, category or brand exists and is still active.

use pos_core::Id;
use rusqlite::{Connection, OptionalExtension, named_params};

use crate::error::{CatalogError, not_found};
use crate::types::UNCATEGORISED;

/// The built-in "Uncategorised" category.
pub(crate) fn uncategorised() -> Id {
    // The constant is a valid id (checked by a test below), so the fallback never runs.
    Id::parse(UNCATEGORISED).unwrap_or_else(|_| Id::new())
}

/// Checks the unit exists and isn't archived. Returns how many decimals it allows.
pub(crate) fn check_unit_decimals(
    connection: &Connection,
    unit_id: &Id,
) -> Result<u8, CatalogError> {
    let decimals: Option<u8> = connection
        .prepare_cached(
            "SELECT decimals FROM catalog_units WHERE id = :id AND archived_at IS NULL",
        )?
        .query_row(named_params! { ":id": unit_id.as_str() }, |row| row.get(0))
        .optional()?;
    decimals.ok_or_else(|| not_found("unit", unit_id.as_str()))
}

/// Checks the category exists and isn't archived.
pub(crate) fn check_category(
    connection: &Connection,
    category_id: &Id,
) -> Result<(), CatalogError> {
    check_active(connection, "catalog_categories", "category", category_id)
}

/// Checks the brand exists and isn't archived.
pub(crate) fn check_brand(connection: &Connection, brand_id: &Id) -> Result<(), CatalogError> {
    check_active(connection, "catalog_brands", "brand", brand_id)
}

fn check_active(
    connection: &Connection,
    table: &str,
    entity: &'static str,
    id: &Id,
) -> Result<(), CatalogError> {
    let sql = format!("SELECT count(*) FROM {table} WHERE id = :id AND archived_at IS NULL");
    let found: i64 = connection
        .prepare_cached(&sql)?
        .query_row(named_params! { ":id": id.as_str() }, |row| row.get(0))?;
    if found == 0 {
        return Err(not_found(entity, id.as_str()));
    }
    Ok(())
}

/// Checks a row with this id exists in `table`, archived or not.
pub(crate) fn check_exists(
    connection: &Connection,
    table: &str,
    entity: &'static str,
    id: &Id,
) -> Result<(), CatalogError> {
    let sql = format!("SELECT count(*) FROM {table} WHERE id = :id");
    let found: i64 =
        connection.query_row(&sql, named_params! { ":id": id.as_str() }, |row| row.get(0))?;
    if found == 0 {
        return Err(not_found(entity, id.as_str()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{UNIT_G, UNIT_KG, UNIT_LITRE, UNIT_ML, UNIT_PACK, UNIT_PIECE};

    #[test]
    fn built_in_ids_are_valid_ids() {
        for text in [
            UNCATEGORISED,
            UNIT_PIECE,
            UNIT_KG,
            UNIT_G,
            UNIT_LITRE,
            UNIT_ML,
            UNIT_PACK,
        ] {
            assert_eq!(Id::parse(text).unwrap().as_str(), text);
        }
    }
}
