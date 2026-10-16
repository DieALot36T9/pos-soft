//! Price lists: named sets of prices for some products, e.g. Wholesale (PRC-02).

use std::collections::HashMap;

use pos_core::{Id, Money, Timestamp};
use rusqlite::{Connection, OptionalExtension, Transaction, named_params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::PricingProduct;
use crate::error::{PricingError, invalid, not_found, stored_id};

/// A saved price list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PriceList {
    pub id: Id,
    pub name: String,
    /// Switched-off lists are ignored when pricing a cart.
    pub active: bool,
    pub created_at: Timestamp,
}

/// What the admin can change on a price list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PriceListUpdate {
    pub name: String,
    pub active: bool,
}

/// One product's price in a price list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PriceListItem {
    pub product_id: Id,
    /// The price for one unit. Example: 95.00.
    pub price: Money,
}

/// Makes a new, switched-on price list. The name must be new.
pub fn create_price_list(tx: &Transaction, name: &str, now: Timestamp) -> Result<Id, PricingError> {
    let name = checked_name(tx, name, None)?;
    let id = Id::new();
    tx.execute(
        "INSERT INTO pricing_price_lists (id, name, active, created_at)
         VALUES (:id, :name, 1, :created_at)",
        named_params! { ":id": id.as_str(), ":name": name, ":created_at": now.0 },
    )?;
    Ok(id)
}

/// Renames a price list or switches it on or off.
pub fn update_price_list(
    tx: &Transaction,
    id: &Id,
    update: &PriceListUpdate,
) -> Result<(), PricingError> {
    require_price_list(tx, id)?;
    let name = checked_name(tx, &update.name, Some(id))?;
    tx.execute(
        "UPDATE pricing_price_lists SET name = :name, active = :active WHERE id = :id",
        named_params! { ":id": id.as_str(), ":name": name, ":active": update.active },
    )?;
    Ok(())
}

/// Sets the price of some products in a price list. A product already there gets the new price.
pub fn set_price_list_items(
    tx: &Transaction,
    id: &Id,
    items: &[PriceListItem],
) -> Result<(), PricingError> {
    require_price_list(tx, id)?;
    if items.iter().any(|item| item.price.0 < 0) {
        return Err(invalid("price", "price_negative"));
    }

    let id_str = id.as_str();

    // To borrow dynamically produced string slices safely, we can map `items` to keep strings in scope
    let items_with_strs: Vec<(&str, &str, i64)> = items
        .iter()
        .map(|item| (id_str, item.product_id.as_str(), item.price.0))
        .collect();

    // Chunk size of 100 items (300 parameters) is well within SQLite's limit.
    for chunk in items_with_strs.chunks(100) {
        let mut query = String::from(
            "INSERT INTO pricing_price_list_items (price_list_id, product_id, price_minor) VALUES ",
        );
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(chunk.len() * 3);

        for (i, (list_id, product_id, price)) in chunk.iter().enumerate() {
            if i > 0 {
                query.push_str(", ");
            }
            query.push_str("(?, ?, ?)");
            params.push(list_id as &dyn rusqlite::ToSql);
            params.push(product_id as &dyn rusqlite::ToSql);
            params.push(price as &dyn rusqlite::ToSql);
        }

        query.push_str(
            " ON CONFLICT (price_list_id, product_id) DO UPDATE SET price_minor = excluded.price_minor",
        );

        tx.execute(&query, rusqlite::params_from_iter(params))?;
    }
    Ok(())
}

/// Takes products out of a price list, so they sell at retail again on that list.
pub fn remove_price_list_items(
    tx: &Transaction,
    id: &Id,
    product_ids: &[Id],
) -> Result<(), PricingError> {
    require_price_list(tx, id)?;
    let mut delete = tx.prepare_cached(
        "DELETE FROM pricing_price_list_items WHERE price_list_id = :list AND product_id = :product",
    )?;
    for product_id in product_ids {
        delete.execute(named_params! { ":list": id.as_str(), ":product": product_id.as_str() })?;
    }
    Ok(())
}

/// Every price list, by name.
pub fn price_lists(conn: &Connection) -> Result<Vec<PriceList>, PricingError> {
    let mut statement =
        conn.prepare("SELECT id, name, active, created_at FROM pricing_price_lists ORDER BY name")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, bool>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;
    let mut lists = Vec::new();
    for row in rows {
        let (id, name, active, created_at) = row?;
        lists.push(PriceList {
            id: stored_id(&id)?,
            name,
            active,
            created_at: Timestamp(created_at),
        });
    }
    Ok(lists)
}

/// Every product price in one price list, ordered by product id.
pub fn price_list_items(conn: &Connection, id: &Id) -> Result<Vec<PriceListItem>, PricingError> {
    require_price_list(conn, id)?;
    let mut statement = conn.prepare(
        "SELECT product_id, price_minor FROM pricing_price_list_items
         WHERE price_list_id = :list ORDER BY product_id",
    )?;
    let rows = statement.query_map(named_params! { ":list": id.as_str() }, |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    let mut items = Vec::new();
    for row in rows {
        let (product_id, price) = row?;
        items.push(PriceListItem {
            product_id: stored_id(&product_id)?,
            price: Money(price),
        });
    }
    Ok(items)
}

/// The list's prices for these products. Empty if the list is switched off.
/// An unknown list is an error, because the cart should never point at one.
pub(crate) fn prices_for(
    conn: &Connection,
    id: &Id,
    products: &[PricingProduct],
) -> Result<HashMap<Id, Money>, PricingError> {
    let active: Option<bool> = conn
        .query_row(
            "SELECT active FROM pricing_price_lists WHERE id = :id",
            named_params! { ":id": id.as_str() },
            |row| row.get(0),
        )
        .optional()?;
    let mut prices = HashMap::new();
    match active {
        None => return Err(not_found("price_list", id)),
        Some(false) => return Ok(prices),
        Some(true) => {}
    }
    let mut statement = conn.prepare_cached(
        "SELECT price_minor FROM pricing_price_list_items
         WHERE price_list_id = :list AND product_id = :product",
    )?;
    for product in products {
        let price: Option<i64> = statement
            .query_row(
                named_params! { ":list": id.as_str(), ":product": product.id.as_str() },
                |row| row.get(0),
            )
            .optional()?;
        if let Some(price) = price {
            prices.insert(product.id.clone(), Money(price));
        }
    }
    Ok(prices)
}

/// Fails with "not found" if the price list doesn't exist.
pub(crate) fn require_price_list(conn: &Connection, id: &Id) -> Result<(), PricingError> {
    let found: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM pricing_price_lists WHERE id = :id",
            named_params! { ":id": id.as_str() },
            |row| row.get(0),
        )
        .optional()?;
    match found {
        Some(_) => Ok(()),
        None => Err(not_found("price_list", id)),
    }
}

/// The trimmed name, if it isn't empty and no other price list uses it.
fn checked_name(
    conn: &Connection,
    name: &str,
    this_list: Option<&Id>,
) -> Result<String, PricingError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(invalid("name", "price_list_name_missing"));
    }
    let owner: Option<String> = conn
        .query_row(
            "SELECT id FROM pricing_price_lists WHERE name = :name",
            named_params! { ":name": name },
            |row| row.get(0),
        )
        .optional()?;
    let this_id = this_list.map(Id::as_str);
    match owner {
        Some(owner) if Some(owner.as_str()) != this_id => Err(PricingError::NameTaken {
            code: "price_list_name_taken",
            name: name.to_string(),
        }),
        _ => Ok(name.to_string()),
    }
}
