//! Barcodes: each one belongs to exactly one product (rule CAT-04).
//! Check digits are checked by `app` with the barcode module before calling these.

use pos_core::{Id, Timestamp};
use rusqlite::{OptionalExtension, Transaction, named_params};

use crate::error::{CatalogError, invalid, not_found};
use crate::read::get;
use crate::rows::read_id;
use crate::search::refresh_search_row;
use crate::types::ProductKind;

/// Adds a barcode to a product and updates its search row.
/// Refused if another product already has it: "Already used by Milk 1L (SKU P000123)".
pub fn add_barcode(
    tx: &Transaction,
    product_id: &Id,
    code: &str,
    now: Timestamp,
) -> Result<(), CatalogError> {
    insert_barcode(tx, product_id, code, now)?;
    refresh_search_row(tx, product_id)
}

/// Saves a barcode row without touching search. The first barcode becomes the primary one.
pub(crate) fn insert_barcode(
    tx: &Transaction,
    product_id: &Id,
    code: &str,
    now: Timestamp,
) -> Result<(), CatalogError> {
    let code = code.trim();
    if code.is_empty() || code.chars().any(char::is_whitespace) {
        return Err(invalid("barcode", "barcode_invalid"));
    }
    let product = get(tx, product_id)?;
    if product.kind == ProductKind::VariantParent {
        return Err(invalid("barcode", "parent_not_sellable"));
    }
    let owner: Option<(String, String)> = tx
        .prepare_cached(
            "SELECT p.name, p.sku FROM catalog_barcodes b
             JOIN catalog_products p ON p.id = b.product_id WHERE b.code = :code",
        )?
        .query_row(named_params! { ":code": code }, |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()?;
    if let Some((product_name, sku)) = owner {
        return Err(CatalogError::BarcodeInUse {
            code: code.to_string(),
            product_name,
            sku,
        });
    }
    tx.prepare_cached(
        "INSERT INTO catalog_barcodes (code, product_id, is_primary, created_at)
         VALUES (:code, :product_id, :is_primary, :now)",
    )?
    .execute(named_params! {
        ":code": code,
        ":product_id": product_id.as_str(),
        ":is_primary": product.barcodes.is_empty(),
        ":now": now.0,
    })?;
    Ok(())
}

/// Removes a barcode, so it can be used again. If it was the primary barcode,
/// the product's next oldest barcode becomes primary.
pub fn remove_barcode(tx: &Transaction, code: &str) -> Result<(), CatalogError> {
    let code = code.trim();
    let product_id = tx
        .query_row(
            "SELECT product_id FROM catalog_barcodes WHERE code = :code",
            named_params! { ":code": code },
            |row| read_id(row, 0),
        )
        .optional()?
        .ok_or_else(|| not_found("barcode", code))?;

    tx.execute(
        "DELETE FROM catalog_barcodes WHERE code = :code",
        named_params! { ":code": code },
    )?;
    tx.execute(
        "UPDATE catalog_barcodes SET is_primary = 1
         WHERE code = (SELECT code FROM catalog_barcodes WHERE product_id = :product_id
                       ORDER BY created_at, rowid LIMIT 1)
           AND NOT EXISTS (SELECT 1 FROM catalog_barcodes
                           WHERE product_id = :product_id AND is_primary = 1)",
        named_params! { ":product_id": product_id.as_str() },
    )?;
    refresh_search_row(tx, &product_id)
}
