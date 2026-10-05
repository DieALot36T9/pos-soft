//! Lookups used while selling: by scanned code, by scale code, and price details.

use std::collections::HashMap;

use pos_core::Id;
use rusqlite::{Connection, OptionalExtension, Row, named_params};

use crate::error::{CatalogError, not_found};
use crate::images::thumbnail_file;
use crate::rows::{read_id, read_money, read_optional_id};
use crate::search::SELLABLE;
use crate::types::{ProductKind, ProductPriceInfo, SaleProduct};

/// Columns for a sale product, read by `read_sale_product`.
const SALE_COLUMNS: &str = "p.id, p.kind, p.name, p.sku, u.symbol, u.decimals, p.sale_price_minor,
    p.tax_rate_id, p.is_weighed, p.track_stock, p.track_expiry, p.category_id, p.brand_id,
    p.image_path";

/// Rule CHK-01: finds the product for a scanned or typed code, by barcode first, then SKU.
/// Archived products and variant parents are never found (rules CAT-08, CHK-15).
pub fn find_by_code(
    connection: &Connection,
    code: &str,
) -> Result<Option<SaleProduct>, CatalogError> {
    let code = code.trim();
    let by_barcode = format!(
        "SELECT {SALE_COLUMNS} FROM catalog_barcodes b
         JOIN catalog_products p ON p.id = b.product_id
         JOIN catalog_units u ON u.id = p.unit_id
         WHERE b.code = :code AND {SELLABLE}"
    );
    if let Some(found) = find_one(connection, &by_barcode, code)? {
        return Ok(Some(found));
    }
    let by_sku = format!(
        "SELECT {SALE_COLUMNS} FROM catalog_products p
         JOIN catalog_units u ON u.id = p.unit_id
         WHERE p.sku = :code AND {SELLABLE}"
    );
    find_one(connection, &by_sku, code)
}

/// Rule CHK-08: finds the product for the item code inside a scale-label barcode.
pub fn find_by_scale_code(
    connection: &Connection,
    item_code: &str,
) -> Result<Option<SaleProduct>, CatalogError> {
    let sql = format!(
        "SELECT {SALE_COLUMNS} FROM catalog_products p
         JOIN catalog_units u ON u.id = p.unit_id
         WHERE p.scale_item_code = :code AND {SELLABLE}"
    );
    find_one(connection, &sql, item_code.trim())
}

fn find_one(
    connection: &Connection,
    sql: &str,
    code: &str,
) -> Result<Option<SaleProduct>, CatalogError> {
    let found = connection
        .prepare_cached(sql)?
        .query_row(named_params! { ":code": code }, read_sale_product)
        .optional()?;
    match found {
        None => Ok(None),
        Some((kind, product)) => Ok(Some(SaleProduct {
            kind: ProductKind::parse(&kind)?,
            ..product
        })),
    }
}

fn read_sale_product(row: &Row) -> rusqlite::Result<(String, SaleProduct)> {
    let image: Option<String> = row.get(13)?;
    let product = SaleProduct {
        id: read_id(row, 0)?,
        kind: ProductKind::Standard,
        name: row.get(2)?,
        sku: row.get(3)?,
        unit_symbol: row.get(4)?,
        unit_decimals: row.get(5)?,
        sale_price: read_money(row, 6)?,
        tax_rate_id: read_id(row, 7)?,
        is_weighed: row.get(8)?,
        track_stock: row.get(9)?,
        track_expiry: row.get(10)?,
        category_id: read_id(row, 11)?,
        brand_id: read_optional_id(row, 12)?,
        thumbnail: image.as_deref().map(thumbnail_file),
    };
    Ok((row.get(1)?, product))
}

/// Price details for each product: price, tax rate, category path and brand.
/// Pricing uses the category path so a promotion on "Food" also covers "Dairy".
pub fn price_info(
    connection: &Connection,
    ids: &[Id],
) -> Result<HashMap<Id, ProductPriceInfo>, CatalogError> {
    let mut found = HashMap::new();
    let mut statement = connection.prepare_cached(
        "SELECT sale_price_minor, cost_price_minor, tax_rate_id, category_id, brand_id
         FROM catalog_products WHERE id = :id",
    )?;
    for product_id in ids {
        let row = statement
            .query_row(named_params! { ":id": product_id.as_str() }, |row| {
                Ok((
                    read_money(row, 0)?,
                    read_money(row, 1)?,
                    read_id(row, 2)?,
                    read_id(row, 3)?,
                    read_optional_id(row, 4)?,
                ))
            })
            .optional()?;
        let (sale_price, cost_price, tax_rate_id, category_id, brand_id) =
            row.ok_or_else(|| not_found("product", product_id.as_str()))?;
        let info = ProductPriceInfo {
            sale_price,
            cost_price,
            tax_rate_id,
            category_path: category_path(connection, category_id)?,
            brand_id,
        };
        found.insert(product_id.clone(), info);
    }
    Ok(found)
}

/// The category and its parents, up to the top: [Dairy, Food].
pub(crate) fn category_path(
    connection: &Connection,
    category_id: Id,
) -> Result<Vec<Id>, CatalogError> {
    let mut statement =
        connection.prepare_cached("SELECT parent_id FROM catalog_categories WHERE id = :id")?;
    let mut path = Vec::new();
    let mut next = Some(category_id);
    // `update_category` refuses loops; stopping at a repeat keeps a damaged tree safe.
    while let Some(current) = next.filter(|current| !path.contains(current)) {
        next = statement
            .query_row(named_params! { ":id": current.as_str() }, |row| {
                read_optional_id(row, 0)
            })
            .optional()?
            .flatten();
        path.push(current);
    }
    Ok(path)
}
