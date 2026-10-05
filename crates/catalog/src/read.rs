//! Reading products: one product in full, or a filtered page of the product list.

use pos_core::{Id, Timestamp};
use rusqlite::{Connection, OptionalExtension, Row, named_params};

use crate::error::{CatalogError, not_found};
use crate::rows::{read_id, read_money, read_optional_id, read_optional_time};
use crate::types::{Product, ProductFilter, ProductKind, ProductList, ProductPage, ProductRow};

/// Everything about one product, with its barcodes and variant attributes.
pub fn get(connection: &Connection, product_id: &Id) -> Result<Product, CatalogError> {
    let product = connection
        .prepare_cached(
            "SELECT id, kind, parent_id, variant_label, name, sku, category_id, brand_id, unit_id,
                    sale_price_minor, cost_price_minor, tax_rate_id, is_weighed, scale_item_code,
                    track_stock, track_expiry, preferred_supplier_id, image_path, description,
                    archived_at, created_at, updated_at
             FROM catalog_products WHERE id = :id",
        )?
        .query_row(named_params! { ":id": product_id.as_str() }, read_product)
        .optional()?;
    let (kind, mut product) = product.ok_or_else(|| not_found("product", product_id.as_str()))?;
    product.kind = ProductKind::parse(&kind)?;
    product.barcodes = barcodes_of(connection, product_id)?;
    product.attributes = attributes_of(connection, product_id)?;
    Ok(product)
}

/// Reads a product row. The kind comes back as text, because `get` checks it
/// with `ProductKind::parse`, which returns a catalog error.
fn read_product(row: &Row) -> rusqlite::Result<(String, Product)> {
    let product = Product {
        id: read_id(row, 0)?,
        kind: ProductKind::Standard,
        parent_id: read_optional_id(row, 2)?,
        variant_label: row.get(3)?,
        name: row.get(4)?,
        sku: row.get(5)?,
        category_id: read_id(row, 6)?,
        brand_id: read_optional_id(row, 7)?,
        unit_id: read_id(row, 8)?,
        sale_price: read_money(row, 9)?,
        cost_price: read_money(row, 10)?,
        tax_rate_id: read_id(row, 11)?,
        is_weighed: row.get(12)?,
        scale_item_code: row.get(13)?,
        track_stock: row.get(14)?,
        track_expiry: row.get(15)?,
        preferred_supplier_id: read_optional_id(row, 16)?,
        image_path: row.get(17)?,
        description: row.get(18)?,
        archived_at: read_optional_time(row, 19)?,
        created_at: Timestamp(row.get(20)?),
        updated_at: Timestamp(row.get(21)?),
        barcodes: Vec::new(),
        attributes: Vec::new(),
    };
    Ok((row.get(1)?, product))
}

/// A product's barcodes, primary first, then in the order they were added.
fn barcodes_of(connection: &Connection, product_id: &Id) -> Result<Vec<String>, CatalogError> {
    let codes = connection
        .prepare_cached(
            "SELECT code FROM catalog_barcodes WHERE product_id = :id
             ORDER BY is_primary DESC, created_at, rowid",
        )?
        .query_map(named_params! { ":id": product_id.as_str() }, |row| {
            row.get(0)
        })?
        .collect::<Result<_, _>>()?;
    Ok(codes)
}

fn attributes_of(
    connection: &Connection,
    product_id: &Id,
) -> Result<Vec<(String, String)>, CatalogError> {
    let attributes = connection
        .prepare_cached(
            "SELECT name, value FROM catalog_product_attributes WHERE product_id = :id ORDER BY name",
        )?
        .query_map(named_params! { ":id": product_id.as_str() }, |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<Result<_, _>>()?;
    Ok(attributes)
}

/// The filter conditions shared by the list and its count. A filter left empty matches everything.
const LIST_FROM_WHERE: &str = "
    FROM catalog_products p
    JOIN catalog_categories c ON c.id = p.category_id
    LEFT JOIN catalog_brands b ON b.id = p.brand_id
    JOIN catalog_units u ON u.id = p.unit_id
    WHERE (:include_archived = 1 OR p.archived_at IS NULL)
      AND (:kind IS NULL OR p.kind = :kind)
      AND (:brand_id IS NULL OR p.brand_id = :brand_id)
      AND (:pattern IS NULL OR p.name LIKE :pattern ESCAPE '\\' OR p.sku LIKE :pattern ESCAPE '\\')
      AND (:category_id IS NULL OR p.category_id IN (
            WITH RECURSIVE wanted(id) AS (
              SELECT :category_id
              UNION SELECT child.id FROM catalog_categories child JOIN wanted ON child.parent_id = wanted.id)
            SELECT id FROM wanted))";

/// One page of the product list, sorted by name, plus the number of matching products.
/// A category filter includes the categories below it.
pub fn list(
    connection: &Connection,
    filter: &ProductFilter,
    page: ProductPage,
) -> Result<ProductList, CatalogError> {
    let pattern = filter
        .text
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(like_pattern);
    let kind = filter.kind.map(ProductKind::as_str);
    let category_id = filter.category_id.as_ref().map(Id::as_str);
    let brand_id = filter.brand_id.as_ref().map(Id::as_str);
    let params = named_params! {
        ":include_archived": filter.include_archived,
        ":kind": kind,
        ":brand_id": brand_id,
        ":pattern": pattern,
        ":category_id": category_id,
    };

    let count_sql = format!("SELECT count(*) {LIST_FROM_WHERE}");
    let total: i64 = connection
        .prepare_cached(&count_sql)?
        .query_row(params, |row| row.get(0))?;

    let rows_sql = format!(
        "SELECT p.id, p.kind, p.name, p.sku, p.category_id, c.name, b.name, u.symbol,
                p.sale_price_minor, p.cost_price_minor, p.archived_at
         {LIST_FROM_WHERE}
         ORDER BY p.name, p.id LIMIT {} OFFSET {}",
        page.limit, page.offset
    );
    let mut statement = connection.prepare_cached(&rows_sql)?;
    let mut rows = Vec::new();
    for row in statement.query_map(params, read_list_row)? {
        let (kind, row) = row?;
        rows.push(ProductRow {
            kind: ProductKind::parse(&kind)?,
            ..row
        });
    }
    Ok(ProductList {
        rows,
        total: u64::try_from(total).unwrap_or(0),
    })
}

fn read_list_row(row: &Row) -> rusqlite::Result<(String, ProductRow)> {
    let kind: String = row.get(1)?;
    let product_row = ProductRow {
        id: read_id(row, 0)?,
        kind: ProductKind::Standard,
        name: row.get(2)?,
        sku: row.get(3)?,
        category_id: read_id(row, 4)?,
        category_name: row.get(5)?,
        brand_name: row.get(6)?,
        unit_symbol: row.get(7)?,
        sale_price: read_money(row, 8)?,
        cost_price: read_money(row, 9)?,
        archived_at: read_optional_time(row, 10)?,
    };
    Ok((kind, product_row))
}

/// "50%_off" becomes "%50\%\_off%", so the user's text is matched as typed.
fn like_pattern(text: &str) -> String {
    let mut pattern = String::from("%");
    for letter in text.chars() {
        if matches!(letter, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(letter);
    }
    pattern.push('%');
    pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_wildcards_typed_by_the_user_are_matched_as_text() {
        assert_eq!(like_pattern("mil"), "%mil%");
        assert_eq!(like_pattern("50%_off"), "%50\\%\\_off%");
    }
}
