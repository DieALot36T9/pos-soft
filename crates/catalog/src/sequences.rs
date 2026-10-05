//! Number sequences: automatic SKUs and internal barcode numbers.
//! Each number is taken inside the caller's transaction, so a cancelled save leaves no gap.

use rusqlite::{Connection, OptionalExtension, Transaction, named_params};

use crate::error::CatalogError;

/// Takes the next number from a sequence ("sku" or "internal_barcode").
fn take_next(tx: &Transaction, name: &str) -> Result<i64, CatalogError> {
    let taken = tx.query_row(
        "UPDATE catalog_sequences SET next = next + 1 WHERE name = :name RETURNING next - 1",
        named_params! { ":name": name },
        |row| row.get(0),
    )?;
    Ok(taken)
}

/// Rule CAT-01: a blank SKU becomes "P000001", "P000002", …
/// A number already typed by hand on another product is skipped.
pub(crate) fn next_sku(tx: &Transaction) -> Result<String, CatalogError> {
    loop {
        let sku = format_sku(take_next(tx, "sku")?);
        if !sku_exists(tx, &sku, None)? {
            return Ok(sku);
        }
    }
}

/// Rule CAT-11: the next number for an internal barcode. `app` turns it into an
/// EAN-13 with the shop's prefix and check digit (barcode module).
pub fn next_internal_number(tx: &Transaction) -> Result<u64, CatalogError> {
    let number = take_next(tx, "internal_barcode")?;
    // The sequence starts at 1, so a number below zero means the table was damaged.
    u64::try_from(number)
        .map_err(|_| CatalogError::Database(rusqlite::Error::IntegralValueOutOfRange(0, number)))
}

/// True when another product (not `except`) already has this SKU.
pub(crate) fn sku_exists(
    connection: &Connection,
    sku: &str,
    except: Option<&str>,
) -> Result<bool, CatalogError> {
    let found: Option<String> = connection
        .query_row(
            "SELECT id FROM catalog_products WHERE sku = :sku",
            named_params! { ":sku": sku },
            |row| row.get(0),
        )
        .optional()?;
    Ok(match found {
        None => false,
        Some(id) => Some(id.as_str()) != except,
    })
}

fn format_sku(number: i64) -> String {
    format!("P{number:06}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skus_have_six_digits_and_grow_past_them() {
        assert_eq!(format_sku(1), "P000001");
        assert_eq!(format_sku(123), "P000123");
        assert_eq!(format_sku(1_234_567), "P1234567");
    }
}
