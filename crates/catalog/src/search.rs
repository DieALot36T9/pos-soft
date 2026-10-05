//! Product search with SQLite's built-in FTS5 full-text search (rules CAT-05, CHK-03).
//!
//! Every product, barcode, category or brand change calls `refresh_search_row` in the
//! same transaction, so search never goes stale.

use pos_core::Id;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};

use crate::error::CatalogError;
use crate::images::thumbnail_file;
use crate::rows::{read_id, read_money};
use crate::types::SearchHit;

/// Columns for a search hit, read by `read_hit`. `p` is the product, `u` its unit.
const HIT_COLUMNS: &str = "p.id, p.name, p.sku, p.sale_price_minor, u.symbol, p.image_path";

/// Products that can be sold: not archived and not a variant parent (rules CAT-08, CHK-15).
pub(crate) const SELLABLE: &str = "p.archived_at IS NULL AND p.kind <> 'variant_parent'";

/// Rule CAT-05: finds products by name, SKU, barcode, category or brand.
/// Each word matches as a prefix ("mil" finds "Milk"). An exact barcode or SKU comes first.
pub fn search(
    connection: &Connection,
    query: &str,
    limit: u32,
) -> Result<Vec<SearchHit>, CatalogError> {
    let text = query.trim();
    let wanted = usize::try_from(limit).unwrap_or(usize::MAX);
    if text.is_empty() || wanted == 0 {
        return Ok(Vec::new());
    }

    let mut hits = exact_hits(connection, text)?;
    if let Some(match_query) = fts_query(text) {
        let sql = format!(
            "SELECT {HIT_COLUMNS}
             FROM catalog_search s
             JOIN catalog_products p ON p.id = s.product_id
             JOIN catalog_units u ON u.id = p.unit_id
             WHERE catalog_search MATCH :query AND {SELLABLE}
             ORDER BY bm25(catalog_search, 0.0, 10.0, 4.0, 4.0, 1.0, 1.0), p.name
             LIMIT :limit"
        );
        let mut statement = connection.prepare_cached(&sql)?;
        // Ask for a few extra rows, because exact hits found above may come back again.
        let extra = i64::from(limit) + i64::try_from(hits.len()).unwrap_or(0);
        let rows = statement.query_map(
            named_params! { ":query": match_query, ":limit": extra },
            |row| read_hit(row, false),
        )?;
        for row in rows {
            let hit = row?;
            if !hits
                .iter()
                .any(|existing| existing.product_id == hit.product_id)
            {
                hits.push(hit);
            }
        }
    }
    hits.truncate(wanted);
    Ok(hits)
}

/// Products whose barcode or SKU is exactly `code`: barcode first, then SKU.
fn exact_hits(connection: &Connection, code: &str) -> Result<Vec<SearchHit>, CatalogError> {
    let by_barcode = format!(
        "SELECT {HIT_COLUMNS} FROM catalog_barcodes b
         JOIN catalog_products p ON p.id = b.product_id
         JOIN catalog_units u ON u.id = p.unit_id
         WHERE b.code = :code AND {SELLABLE}"
    );
    let by_sku = format!(
        "SELECT {HIT_COLUMNS} FROM catalog_products p
         JOIN catalog_units u ON u.id = p.unit_id
         WHERE p.sku = :code AND {SELLABLE}"
    );
    let mut hits: Vec<SearchHit> = Vec::new();
    for sql in [by_barcode, by_sku] {
        let hit = connection
            .prepare_cached(&sql)?
            .query_row(named_params! { ":code": code }, |row| read_hit(row, true))
            .optional()?;
        if let Some(hit) = hit
            && !hits
                .iter()
                .any(|existing| existing.product_id == hit.product_id)
        {
            hits.push(hit);
        }
    }
    Ok(hits)
}

fn read_hit(row: &Row, exact_match: bool) -> rusqlite::Result<SearchHit> {
    let image: Option<String> = row.get(5)?;
    Ok(SearchHit {
        product_id: read_id(row, 0)?,
        name: row.get(1)?,
        sku: row.get(2)?,
        sale_price: read_money(row, 3)?,
        unit_symbol: row.get(4)?,
        thumbnail: image.as_deref().map(thumbnail_file),
        exact_match,
    })
}

/// Turns typed text into an FTS5 query: every word must match as a prefix.
/// Symbols are dropped, so text like `"soap* OR (` can't break the query.
/// Example: "man jui" becomes `"man"* "jui"*`. `None` when no words are left.
fn fts_query(text: &str) -> Option<String> {
    let words: Vec<String> = text
        .split(|letter: char| !letter.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| format!("\"{word}\"*"))
        .collect();
    if words.is_empty() {
        None
    } else {
        Some(words.join(" "))
    }
}

/// Rewrites one product's search row from its current data. An archived product or a
/// variant parent gets no row, so it can't be found (rules CAT-08, CHK-15).
pub(crate) fn refresh_search_row(tx: &Transaction, product_id: &Id) -> Result<(), CatalogError> {
    remove_search_row(tx, product_id)?;
    let added = tx
        .prepare_cached(&format!(
            "INSERT INTO catalog_search (product_id, name, sku, barcodes, category, brand)
         SELECT p.id, p.name, p.sku,
                COALESCE((SELECT group_concat(code, ' ') FROM catalog_barcodes
                          WHERE product_id = p.id), ''),
                c.name, COALESCE(b.name, '')
         FROM catalog_products p
         JOIN catalog_categories c ON c.id = p.category_id
         LEFT JOIN catalog_brands b ON b.id = p.brand_id
         WHERE p.id = :product_id AND {SELLABLE}"
        ))?
        .execute(named_params! { ":product_id": product_id.as_str() })?;
    if added == 1 {
        tx.prepare_cached(
            "INSERT INTO catalog_search_rows (product_id, search_rowid) VALUES (:product_id, :rowid)",
        )?
        .execute(named_params! { ":product_id": product_id.as_str(), ":rowid": tx.last_insert_rowid() })?;
    }
    Ok(())
}

/// Deletes one product's search row, if it has one.
pub(crate) fn remove_search_row(tx: &Transaction, product_id: &Id) -> Result<(), CatalogError> {
    let old_rowid: Option<i64> = tx
        .prepare_cached(
            "SELECT search_rowid FROM catalog_search_rows WHERE product_id = :product_id",
        )?
        .query_row(
            named_params! { ":product_id": product_id.as_str() },
            |row| row.get(0),
        )
        .optional()?;
    if let Some(rowid) = old_rowid {
        tx.prepare_cached("DELETE FROM catalog_search WHERE rowid = :rowid")?
            .execute(named_params! { ":rowid": rowid })?;
        tx.prepare_cached("DELETE FROM catalog_search_rows WHERE product_id = :product_id")?
            .execute(named_params! { ":product_id": product_id.as_str() })?;
    }
    Ok(())
}

/// Rewrites the search rows of every product that matches `where_sql`
/// (a condition on `catalog_products` with one `:id` parameter).
pub(crate) fn refresh_search_rows_where(
    tx: &Transaction,
    where_sql: &str,
    id: &Id,
) -> Result<(), CatalogError> {
    let sql = format!("SELECT id FROM catalog_products WHERE {where_sql}");
    let product_ids: Vec<Id> = tx
        .prepare(&sql)?
        .query_map(named_params! { ":id": id.as_str() }, |row| read_id(row, 0))?
        .collect::<Result<_, _>>()?;
    for product_id in &product_ids {
        refresh_search_row(tx, product_id)?;
    }
    Ok(())
}

/// Rebuilds the whole search table from the products. Use it after a restore or
/// if search results ever look wrong.
pub fn rebuild_search(tx: &Transaction) -> Result<(), CatalogError> {
    tx.execute_batch("DELETE FROM catalog_search_rows; DELETE FROM catalog_search;")?;
    let product_ids: Vec<Id> = tx
        .prepare("SELECT id FROM catalog_products")?
        .query_map([], |row| read_id(row, 0))?
        .collect::<Result<_, _>>()?;
    for product_id in &product_ids {
        refresh_search_row(tx, product_id)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_word_becomes_a_quoted_prefix() {
        assert_eq!(fts_query("man jui").unwrap(), "\"man\"* \"jui\"*");
        assert_eq!(fts_query("Café-Crème").unwrap(), "\"Café\"* \"Crème\"*");
    }

    #[test]
    fn search_syntax_is_dropped() {
        assert_eq!(fts_query("\"soap* OR (").unwrap(), "\"soap\"* \"OR\"*");
        assert_eq!(fts_query(" -*- "), None);
    }
}
