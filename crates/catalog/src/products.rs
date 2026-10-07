//! Saving, changing, archiving and deleting products (rules CAT-01, CAT-08, CAT-09).

use pos_core::{Id, Money, Timestamp};
use rusqlite::{Transaction, named_params};

use crate::barcodes::insert_barcode;
use crate::checks::{check_brand, check_category, check_unit_decimals, uncategorised};
use crate::error::{CatalogError, conflict, invalid};
use crate::prices::{check_reason, record_price_change};
use crate::read::get;
use crate::rows::{clean_name, clean_optional, optional_id_text};
use crate::search::{refresh_search_row, remove_search_row};
use crate::sequences::{next_sku, sku_exists};
use crate::types::{NewProduct, PriceChanges, PriceField, ProductKind, ProductUpdate};

/// Where a new variant belongs: its parent and its label, e.g. "200 ml / Mango".
pub(crate) struct VariantOf<'a> {
    pub parent_id: &'a Id,
    pub label: &'a str,
}

/// Saves a new product and its barcodes, and adds it to search. Returns its id.
/// Variants are made with `generate_variants`, not here.
pub fn create_product(
    tx: &Transaction,
    new: NewProduct,
    now: Timestamp,
) -> Result<Id, CatalogError> {
    if new.kind == ProductKind::Variant {
        return Err(invalid("kind", "variant_needs_parent"));
    }
    insert_product(tx, &new, None, now)
}

/// Checks and saves a product row, then its barcodes and search row.
pub(crate) fn insert_product(
    tx: &Transaction,
    new: &NewProduct,
    variant_of: Option<VariantOf>,
    now: Timestamp,
) -> Result<Id, CatalogError> {
    let name = clean_name(&new.name)?;
    let category_id = new.category_id.clone().unwrap_or_else(uncategorised);
    check_category(tx, &category_id)?;
    if let Some(brand_id) = &new.brand_id {
        check_brand(tx, brand_id)?;
    }
    check_prices(new.sale_price, new.cost_price)?;
    check_weighed(tx, &new.unit_id, new.is_weighed)?;
    let scale_item_code = clean_optional(new.scale_item_code.as_deref());
    check_scale_code_free(tx, scale_item_code.as_deref(), None)?;
    let sku = match clean_optional(new.sku.as_deref()) {
        Some(sku) => {
            if sku_exists(tx, &sku, None)? {
                return Err(conflict("sku_in_use", sku));
            }
            sku
        }
        None => next_sku(tx)?,
    };
    if new.kind == ProductKind::VariantParent && !new.barcodes.is_empty() {
        return Err(invalid("barcodes", "parent_not_sellable"));
    }

    let id = Id::new();
    tx.prepare_cached(
        "INSERT INTO catalog_products (id, kind, parent_id, variant_label, name, sku, category_id,
            brand_id, unit_id, sale_price_minor, cost_price_minor, tax_rate_id, is_weighed,
            scale_item_code, track_stock, track_expiry, preferred_supplier_id, description,
            created_at, updated_at)
         VALUES (:id, :kind, :parent_id, :variant_label, :name, :sku, :category_id,
            :brand_id, :unit_id, :sale_price, :cost_price, :tax_rate_id, :is_weighed,
            :scale_item_code, :track_stock, :track_expiry, :supplier_id, :description,
            :now, :now)",
    )?
    .execute(named_params! {
        ":id": id.as_str(),
        ":kind": new.kind.as_str(),
        ":parent_id": variant_of.as_ref().map(|variant| variant.parent_id.as_str()),
        ":variant_label": variant_of.as_ref().map(|variant| variant.label),
        ":name": name,
        ":sku": sku,
        ":category_id": category_id.as_str(),
        ":brand_id": optional_id_text(new.brand_id.as_ref()),
        ":unit_id": new.unit_id.as_str(),
        ":sale_price": new.sale_price.0,
        ":cost_price": new.cost_price.0,
        ":tax_rate_id": new.tax_rate_id.as_str(),
        ":is_weighed": new.is_weighed,
        ":scale_item_code": scale_item_code,
        ":track_stock": holds_stock(new.kind) && new.track_stock,
        ":track_expiry": new.track_expiry,
        ":supplier_id": optional_id_text(new.preferred_supplier_id.as_ref()),
        ":description": clean_optional(new.description.as_deref()),
        ":now": now.0,
    })?;

    for code in &new.barcodes {
        insert_barcode(tx, &id, code, now)?;
    }
    refresh_search_row(tx, &id)?;
    Ok(id)
}

/// Saves edits to a product. A changed sale or cost price is written to the price
/// history with the user and time (rule CAT-09). Returns which prices changed.
pub fn update_product(
    tx: &Transaction,
    product_id: &Id,
    update: ProductUpdate,
    by: &Id,
    now: Timestamp,
) -> Result<PriceChanges, CatalogError> {
    update_product_with_reason(tx, product_id, update, (by, "manual"), now)
}

/// Like `update_product`, with the reason written to the price history: "manual",
/// "grn" or "import". An import uses "import" (C-M12).
pub fn update_product_with_reason(
    tx: &Transaction,
    product_id: &Id,
    update: ProductUpdate,
    (by, reason): (&Id, &str),
    now: Timestamp,
) -> Result<PriceChanges, CatalogError> {
    check_reason(reason)?;
    let current = get(tx, product_id)?;
    let name = clean_name(&update.name)?;
    let sku = clean_optional(Some(&update.sku)).ok_or(invalid("sku", "sku_missing"))?;
    if sku_exists(tx, &sku, Some(product_id.as_str()))? {
        return Err(conflict("sku_in_use", sku));
    }
    check_category(tx, &update.category_id)?;
    if let Some(brand_id) = &update.brand_id {
        check_brand(tx, brand_id)?;
    }
    check_prices(update.sale_price, update.cost_price)?;
    check_weighed(tx, &update.unit_id, update.is_weighed)?;
    let scale_item_code = clean_optional(update.scale_item_code.as_deref());
    check_scale_code_free(tx, scale_item_code.as_deref(), Some(product_id))?;

    tx.prepare_cached(
        "UPDATE catalog_products SET name = :name, sku = :sku, category_id = :category_id,
            brand_id = :brand_id, unit_id = :unit_id, sale_price_minor = :sale_price,
            cost_price_minor = :cost_price, tax_rate_id = :tax_rate_id, is_weighed = :is_weighed,
            scale_item_code = :scale_item_code, track_stock = :track_stock,
            track_expiry = :track_expiry, preferred_supplier_id = :supplier_id,
            description = :description, updated_at = :now
         WHERE id = :id",
    )?
    .execute(named_params! {
        ":id": product_id.as_str(),
        ":name": name,
        ":sku": sku,
        ":category_id": update.category_id.as_str(),
        ":brand_id": optional_id_text(update.brand_id.as_ref()),
        ":unit_id": update.unit_id.as_str(),
        ":sale_price": update.sale_price.0,
        ":cost_price": update.cost_price.0,
        ":tax_rate_id": update.tax_rate_id.as_str(),
        ":is_weighed": update.is_weighed,
        ":scale_item_code": scale_item_code,
        ":track_stock": holds_stock(current.kind) && update.track_stock,
        ":track_expiry": update.track_expiry,
        ":supplier_id": optional_id_text(update.preferred_supplier_id.as_ref()),
        ":description": clean_optional(update.description.as_deref()),
        ":now": now.0,
    })?;

    let changes = PriceChanges {
        sale_price: record_price_change(
            tx,
            product_id,
            PriceField::SalePrice,
            (current.sale_price, update.sale_price),
            (by, reason),
            now,
        )?,
        cost_price: record_price_change(
            tx,
            product_id,
            PriceField::CostPrice,
            (current.cost_price, update.cost_price),
            (by, reason),
            now,
        )?,
    };
    refresh_search_row(tx, product_id)?;
    Ok(changes)
}

/// Rule CAT-08: hides a product from sale and search but keeps it for history and reports.
pub fn archive_product(
    tx: &Transaction,
    product_id: &Id,
    now: Timestamp,
) -> Result<(), CatalogError> {
    get(tx, product_id)?;
    tx.execute(
        "UPDATE catalog_products SET archived_at = :now, updated_at = :now WHERE id = :id",
        named_params! { ":id": product_id.as_str(), ":now": now.0 },
    )?;
    refresh_search_row(tx, product_id)
}

/// Rule CAT-08: brings an archived product back into sale and search. Its unit,
/// category and brand must still be active.
pub fn unarchive_product(
    tx: &Transaction,
    product_id: &Id,
    now: Timestamp,
) -> Result<(), CatalogError> {
    let product = get(tx, product_id)?;
    check_unit_decimals(tx, &product.unit_id)?;
    check_category(tx, &product.category_id)?;
    if let Some(brand_id) = &product.brand_id {
        check_brand(tx, brand_id)?;
    }
    tx.execute(
        "UPDATE catalog_products SET archived_at = NULL, updated_at = :now WHERE id = :id",
        named_params! { ":id": product_id.as_str(), ":now": now.0 },
    )?;
    refresh_search_row(tx, product_id)
}

/// Rule CAT-08: deletes a product for good, only if it was never used. `app` asks the
/// other modules (sales, stock, purchase orders) and passes the answer as `used_elsewhere`.
pub fn delete_unused_product(
    tx: &Transaction,
    product_id: &Id,
    used_elsewhere: bool,
) -> Result<(), CatalogError> {
    let product = get(tx, product_id)?;
    if used_elsewhere || is_kit_component(tx, product_id)? {
        return Err(conflict("product_in_use", product.name));
    }
    let variant_count: i64 = tx.query_row(
        "SELECT count(*) FROM catalog_products WHERE parent_id = :id",
        named_params! { ":id": product_id.as_str() },
        |row| row.get(0),
    )?;
    if variant_count > 0 {
        return Err(conflict("product_has_variants", product.name));
    }

    remove_search_row(tx, product_id)?;
    for sql in [
        "DELETE FROM catalog_barcodes WHERE product_id = :id",
        "DELETE FROM catalog_product_attributes WHERE product_id = :id",
        "DELETE FROM catalog_kit_components WHERE kit_id = :id",
        "DELETE FROM catalog_price_changes WHERE product_id = :id",
        "DELETE FROM catalog_products WHERE id = :id",
    ] {
        tx.execute(sql, named_params! { ":id": product_id.as_str() })?;
    }
    Ok(())
}

/// Kits and variant parents never hold stock of their own (rules CAT-06, CAT-07).
fn holds_stock(kind: ProductKind) -> bool {
    matches!(kind, ProductKind::Standard | ProductKind::Variant)
}

fn check_prices(sale_price: Money, cost_price: Money) -> Result<(), CatalogError> {
    if sale_price.0 < 0 {
        return Err(invalid("sale_price", "price_negative"));
    }
    if cost_price.0 < 0 {
        return Err(invalid("cost_price", "price_negative"));
    }
    Ok(())
}

/// Rule CAT-01: a weighed product needs a unit that allows decimals, such as kg.
fn check_weighed(tx: &Transaction, unit_id: &Id, is_weighed: bool) -> Result<(), CatalogError> {
    let decimals = check_unit_decimals(tx, unit_id)?;
    if is_weighed && decimals == 0 {
        return Err(invalid("is_weighed", "weighed_unit_needs_decimals"));
    }
    Ok(())
}

/// Rule CAT-01: a scale item code belongs to one product only.
fn check_scale_code_free(
    tx: &Transaction,
    scale_item_code: Option<&str>,
    except: Option<&Id>,
) -> Result<(), CatalogError> {
    let Some(code) = scale_item_code else {
        return Ok(());
    };
    let owners: i64 = tx.query_row(
        "SELECT count(*) FROM catalog_products WHERE scale_item_code = :code AND id <> :except",
        named_params! { ":code": code, ":except": except.map_or("", Id::as_str) },
        |row| row.get(0),
    )?;
    if owners > 0 {
        return Err(conflict("scale_item_code_in_use", code));
    }
    Ok(())
}

fn is_kit_component(tx: &Transaction, product_id: &Id) -> Result<bool, CatalogError> {
    let kits: i64 = tx.query_row(
        "SELECT count(*) FROM catalog_kit_components WHERE component_id = :id",
        named_params! { ":id": product_id.as_str() },
        |row| row.get(0),
    )?;
    Ok(kits > 0)
}
