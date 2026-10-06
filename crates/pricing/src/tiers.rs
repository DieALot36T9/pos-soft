//! Tiers: customer membership levels with an optional price list and discount (PRC-03).

use pos_core::{Bps, Id, Timestamp};
use rusqlite::{Connection, OptionalExtension, Transaction, named_params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{PricingError, invalid, not_found, stored_id};
use crate::price_lists::require_price_list;

/// A saved tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Tier {
    pub id: Id,
    pub name: String,
    pub price_list_id: Option<Id>,
    /// Taken off lines without a promotion. Example: `Bps(200)` is 2%.
    pub discount: Bps,
    /// Lower numbers are listed first.
    pub sort: i64,
    /// Set when the tier is archived. Archived tiers give no prices or discount.
    pub archived_at: Option<Timestamp>,
}

/// What the admin enters to make or change a tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TierInput {
    pub name: String,
    pub price_list_id: Option<Id>,
    pub discount: Bps,
    pub sort: i64,
}

/// Makes a new tier. Example: "Gold" = price list Member + 2% off.
pub fn create_tier(tx: &Transaction, input: &TierInput) -> Result<Id, PricingError> {
    let name = checked_input(tx, input, None)?;
    let id = Id::new();
    tx.execute(
        "INSERT INTO pricing_tiers (id, name, price_list_id, discount_bps, sort)
         VALUES (:id, :name, :list, :discount, :sort)",
        named_params! {
            ":id": id.as_str(), ":name": name,
            ":list": input.price_list_id.as_ref().map(Id::as_str),
            ":discount": input.discount.0, ":sort": input.sort,
        },
    )?;
    Ok(id)
}

/// Changes a tier's name, price list, discount or place in the list.
pub fn update_tier(tx: &Transaction, id: &Id, input: &TierInput) -> Result<(), PricingError> {
    tier(tx, id)?;
    let name = checked_input(tx, input, Some(id))?;
    tx.execute(
        "UPDATE pricing_tiers
         SET name = :name, price_list_id = :list, discount_bps = :discount, sort = :sort
         WHERE id = :id",
        named_params! {
            ":id": id.as_str(), ":name": name,
            ":list": input.price_list_id.as_ref().map(Id::as_str),
            ":discount": input.discount.0, ":sort": input.sort,
        },
    )?;
    Ok(())
}

/// Archives a tier. Customers may still point at it, but it no longer changes prices.
pub fn archive_tier(tx: &Transaction, id: &Id, now: Timestamp) -> Result<(), PricingError> {
    tier(tx, id)?;
    tx.execute(
        "UPDATE pricing_tiers SET archived_at = :now WHERE id = :id AND archived_at IS NULL",
        named_params! { ":id": id.as_str(), ":now": now.0 },
    )?;
    Ok(())
}

/// Every tier, archived ones too, in list order.
pub fn tiers(conn: &Connection) -> Result<Vec<Tier>, PricingError> {
    let mut statement = conn.prepare(
        "SELECT id, name, price_list_id, discount_bps, sort, archived_at
         FROM pricing_tiers ORDER BY sort, name",
    )?;
    let rows = statement.query_map([], read_row)?;
    let mut list = Vec::new();
    for row in rows {
        list.push(to_tier(row?)?);
    }
    Ok(list)
}

/// One tier by id.
pub fn tier(conn: &Connection, id: &Id) -> Result<Tier, PricingError> {
    let row = conn
        .query_row(
            "SELECT id, name, price_list_id, discount_bps, sort, archived_at
             FROM pricing_tiers WHERE id = :id",
            named_params! { ":id": id.as_str() },
            read_row,
        )
        .optional()?;
    match row {
        Some(row) => to_tier(row),
        None => Err(not_found("tier", id)),
    }
}

type TierRow = (String, String, Option<String>, u32, i64, Option<i64>);

fn read_row(row: &rusqlite::Row) -> rusqlite::Result<TierRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn to_tier(row: TierRow) -> Result<Tier, PricingError> {
    let (id, name, price_list_id, discount, sort, archived_at) = row;
    let price_list_id = match price_list_id {
        Some(text) => Some(stored_id(&text)?),
        None => None,
    };
    Ok(Tier {
        id: stored_id(&id)?,
        name,
        price_list_id,
        discount: Bps(discount),
        sort,
        archived_at: archived_at.map(Timestamp),
    })
}

/// Checks the input and returns the trimmed name.
fn checked_input(
    conn: &Connection,
    input: &TierInput,
    this_tier: Option<&Id>,
) -> Result<String, PricingError> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(invalid("name", "tier_name_missing"));
    }
    if input.discount.0 > 10_000 {
        return Err(invalid("discount", "tier_discount_invalid"));
    }
    if let Some(list) = &input.price_list_id {
        require_price_list(conn, list)?;
    }
    let owner: Option<String> = conn
        .query_row(
            "SELECT id FROM pricing_tiers WHERE name = :name",
            named_params! { ":name": name },
            |row| row.get(0),
        )
        .optional()?;
    let this_id = this_tier.map(Id::as_str);
    match owner {
        Some(owner) if Some(owner.as_str()) != this_id => Err(PricingError::NameTaken {
            code: "tier_name_taken",
            name: name.to_string(),
        }),
        _ => Ok(name.to_string()),
    }
}
