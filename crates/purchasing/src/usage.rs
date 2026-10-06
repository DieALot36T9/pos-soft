//! Whether a product appears on any purchasing document (CAT-08).

use pos_core::Id;
use rusqlite::Connection;

use crate::PurchasingError;

/// True if the product is on any purchase order, goods received note or supplier
/// return. Such a product can only be archived, never deleted (CAT-08).
pub fn is_product_used(connection: &Connection, product: &Id) -> Result<bool, PurchasingError> {
    let used: bool = connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM purchasing_order_lines WHERE product_id = ?1)
             OR EXISTS (SELECT 1 FROM purchasing_grn_lines WHERE product_id = ?1)
             OR EXISTS (SELECT 1 FROM purchasing_return_lines WHERE product_id = ?1)",
        [product.as_str()],
        |row| row.get(0),
    )?;
    Ok(used)
}
