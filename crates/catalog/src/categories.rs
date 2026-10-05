//! Categories (a tree) and brands (a flat list), rule CAT-03.

use pos_core::{Id, Timestamp};
use rusqlite::{Connection, Transaction, named_params};

use crate::checks::{check_category, check_exists, uncategorised};
use crate::error::{CatalogError, conflict, invalid};
use crate::lookup::category_path;
use crate::rows::{clean_name, optional_id_text, read_id, read_optional_id, read_optional_time};
use crate::search::refresh_search_rows_where;
use crate::types::{Brand, Category, CategoryFields};

/// Adds a category, at the top or under an active parent.
pub fn create_category(tx: &Transaction, fields: &CategoryFields) -> Result<Id, CatalogError> {
    let name = clean_name(&fields.name)?;
    if let Some(parent_id) = &fields.parent_id {
        check_category(tx, parent_id)?;
    }
    let id = Id::new();
    tx.execute(
        "INSERT INTO catalog_categories (id, parent_id, name, sort) VALUES (:id, :parent_id, :name, :sort)",
        named_params! {
            ":id": id.as_str(),
            ":parent_id": optional_id_text(fields.parent_id.as_ref()),
            ":name": name,
            ":sort": fields.sort,
        },
    )?;
    Ok(id)
}

/// Renames or moves a category. It can't move under itself or one of its own children.
/// Its products' search rows are updated, so the new name is found.
pub fn update_category(
    tx: &Transaction,
    category_id: &Id,
    fields: &CategoryFields,
) -> Result<(), CatalogError> {
    check_exists(tx, "catalog_categories", "category", category_id)?;
    let name = clean_name(&fields.name)?;
    if let Some(parent_id) = &fields.parent_id {
        check_category(tx, parent_id)?;
        if category_path(tx, parent_id.clone())?.contains(category_id) {
            return Err(invalid("parent_id", "category_parent_loop"));
        }
    }
    tx.execute(
        "UPDATE catalog_categories SET name = :name, parent_id = :parent_id, sort = :sort WHERE id = :id",
        named_params! {
            ":id": category_id.as_str(),
            ":parent_id": optional_id_text(fields.parent_id.as_ref()),
            ":name": name,
            ":sort": fields.sort,
        },
    )?;
    refresh_search_rows_where(tx, "category_id = :id", category_id)
}

/// Hides a category. Refused for "Uncategorised", and while it still has active
/// products or active child categories.
pub fn archive_category(
    tx: &Transaction,
    category_id: &Id,
    now: Timestamp,
) -> Result<(), CatalogError> {
    check_exists(tx, "catalog_categories", "category", category_id)?;
    let name: String = tx.query_row(
        "SELECT name FROM catalog_categories WHERE id = :id",
        named_params! { ":id": category_id.as_str() },
        |row| row.get(0),
    )?;
    if *category_id == uncategorised() {
        return Err(conflict("category_is_built_in", name));
    }
    let count = |sql: &str| -> Result<i64, CatalogError> {
        Ok(
            tx.query_row(sql, named_params! { ":id": category_id.as_str() }, |row| {
                row.get(0)
            })?,
        )
    };
    if count(
        "SELECT count(*) FROM catalog_categories WHERE parent_id = :id AND archived_at IS NULL",
    )? > 0
    {
        return Err(conflict("category_has_children", name));
    }
    if count(
        "SELECT count(*) FROM catalog_products WHERE category_id = :id AND archived_at IS NULL",
    )? > 0
    {
        return Err(conflict("category_has_products", name));
    }
    tx.execute(
        "UPDATE catalog_categories SET archived_at = :now WHERE id = :id",
        named_params! { ":id": category_id.as_str(), ":now": now.0 },
    )?;
    Ok(())
}

/// Every category, sorted by `sort` and then name. The UI builds the tree from `parent_id`.
pub fn list_categories(
    connection: &Connection,
    include_archived: bool,
) -> Result<Vec<Category>, CatalogError> {
    let categories = connection
        .prepare_cached(
            "SELECT id, parent_id, name, sort, archived_at FROM catalog_categories
             WHERE :include_archived = 1 OR archived_at IS NULL ORDER BY sort, name",
        )?
        .query_map(
            named_params! { ":include_archived": include_archived },
            |row| {
                Ok(Category {
                    id: read_id(row, 0)?,
                    parent_id: read_optional_id(row, 1)?,
                    name: row.get(2)?,
                    sort: row.get(3)?,
                    archived_at: read_optional_time(row, 4)?,
                })
            },
        )?
        .collect::<Result<_, _>>()?;
    Ok(categories)
}

/// Adds a brand. Brand names are unique.
pub fn create_brand(tx: &Transaction, name: &str) -> Result<Id, CatalogError> {
    let name = clean_brand_name(tx, name, None)?;
    let id = Id::new();
    tx.execute(
        "INSERT INTO catalog_brands (id, name) VALUES (:id, :name)",
        named_params! { ":id": id.as_str(), ":name": name },
    )?;
    Ok(id)
}

/// Renames a brand and updates its products' search rows.
pub fn rename_brand(tx: &Transaction, brand_id: &Id, name: &str) -> Result<(), CatalogError> {
    check_exists(tx, "catalog_brands", "brand", brand_id)?;
    let name = clean_brand_name(tx, name, Some(brand_id))?;
    tx.execute(
        "UPDATE catalog_brands SET name = :name WHERE id = :id",
        named_params! { ":id": brand_id.as_str(), ":name": name },
    )?;
    refresh_search_rows_where(tx, "brand_id = :id", brand_id)
}

/// Hides a brand from new products. Products that have it keep it.
pub fn archive_brand(tx: &Transaction, brand_id: &Id, now: Timestamp) -> Result<(), CatalogError> {
    check_exists(tx, "catalog_brands", "brand", brand_id)?;
    tx.execute(
        "UPDATE catalog_brands SET archived_at = :now WHERE id = :id",
        named_params! { ":id": brand_id.as_str(), ":now": now.0 },
    )?;
    Ok(())
}

/// Every brand, sorted by name. Archived brands only when `include_archived` is true.
pub fn list_brands(
    connection: &Connection,
    include_archived: bool,
) -> Result<Vec<Brand>, CatalogError> {
    let brands = connection
        .prepare_cached(
            "SELECT id, name, archived_at FROM catalog_brands
             WHERE :include_archived = 1 OR archived_at IS NULL ORDER BY name",
        )?
        .query_map(
            named_params! { ":include_archived": include_archived },
            |row| {
                Ok(Brand {
                    id: read_id(row, 0)?,
                    name: row.get(1)?,
                    archived_at: read_optional_time(row, 2)?,
                })
            },
        )?
        .collect::<Result<_, _>>()?;
    Ok(brands)
}

fn clean_brand_name(
    tx: &Transaction,
    name: &str,
    except: Option<&Id>,
) -> Result<String, CatalogError> {
    let name = clean_name(name)?;
    let same_name: i64 = tx.query_row(
        "SELECT count(*) FROM catalog_brands WHERE name = :name AND id <> :except",
        named_params! { ":name": name, ":except": except.map_or("", Id::as_str) },
        |row| row.get(0),
    )?;
    if same_name > 0 {
        return Err(conflict("brand_name_in_use", name));
    }
    Ok(name)
}
