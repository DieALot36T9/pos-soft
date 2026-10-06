//! The pricing screens in the back office (Req - Pricing and Promotions): price lists
//! and their prices (PRC-02), tiers (PRC-03) and promotions (PRC-04). Each save is one
//! transaction with its `pricing.change` audit event.

use pos_core::{AppError, Bps, Id, Money};
use pricing::{
    PriceList, PriceListItem, PriceListUpdate, Promotion, PromotionInput, Tier, TierInput,
};
use rusqlite::Transaction;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::audit_actions::PRICING_CHANGE;
use crate::audit_log;
use crate::state::AppState;

/// What the admin types to add a price list (`id` empty) or rename or switch one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct PriceListSave {
    pub id: Option<Id>,
    /// Example: "Wholesale".
    pub name: String,
    pub active: bool,
}

/// One product's price in a price list, with what the screen shows next to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PriceListItemRow {
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    /// The normal sale price, for comparison.
    pub retail_price: Money,
    /// The price in this list.
    pub price: Money,
}

/// What the admin types to add a tier (`id` empty) or change one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct TierSave {
    pub id: Option<Id>,
    /// Example: "Gold".
    pub name: String,
    pub price_list_id: Option<Id>,
    /// Taken off lines without a promotion. Example: 200 is 2%.
    pub discount: Bps,
    pub sort: i64,
}

/// What the admin types to add a promotion (`id` empty) or change one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct PromotionSave {
    pub id: Option<Id>,
    pub promotion: PromotionInput,
}

/// Every price list, by name.
pub fn price_lists(state: &AppState) -> Result<Vec<PriceList>, AppError> {
    state.db.read(|conn| Ok(pricing::price_lists(conn)?))
}

/// Adds or changes a price list (PRC-02).
pub fn save_price_list(state: &AppState, by: &Id, input: &PriceListSave) -> Result<Id, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let id = match &input.id {
            Some(id) => {
                let update = PriceListUpdate {
                    name: input.name.clone(),
                    active: input.active,
                };
                pricing::update_price_list(tx, id, &update)?;
                id.clone()
            }
            None => {
                let id = pricing::create_price_list(tx, &input.name, clock.now())?;
                if !input.active {
                    let update = PriceListUpdate {
                        name: input.name.clone(),
                        active: false,
                    };
                    pricing::update_price_list(tx, &id, &update)?;
                }
                id
            }
        };
        let details = serde_json::json!({
            "what": "price_list",
            "id": id,
            "name": input.name.trim(),
            "active": input.active,
            "new": input.id.is_none(),
        });
        record(tx, state, by, details)?;
        Ok(id)
    })
}

/// The prices in one list, with each product's name and normal price.
pub fn price_list_items(state: &AppState, id: &Id) -> Result<Vec<PriceListItemRow>, AppError> {
    state.db.read(|conn| {
        let items = pricing::price_list_items(conn, id)?;
        let ids: Vec<Id> = items.iter().map(|item| item.product_id.clone()).collect();
        let infos = catalog::price_info(conn, &ids)?;
        let mut rows = Vec::with_capacity(items.len());
        for item in items {
            let product = catalog::get(conn, &item.product_id)?;
            rows.push(PriceListItemRow {
                retail_price: infos
                    .get(&item.product_id)
                    .map_or(product.sale_price, |info| info.sale_price),
                product_id: item.product_id,
                name: product.name,
                sku: product.sku,
                price: item.price,
            });
        }
        rows.sort_by_key(|row| row.name.to_lowercase());
        Ok(rows)
    })
}

/// Adds or replaces prices in a list (PRC-02). Every product must exist.
pub fn set_price_list_items(
    state: &AppState,
    by: &Id,
    id: &Id,
    items: &[PriceListItem],
) -> Result<(), AppError> {
    state.db.write(|tx| {
        for item in items {
            catalog::get(tx, &item.product_id)?;
        }
        pricing::set_price_list_items(tx, id, items)?;
        let details = serde_json::json!({
            "what": "price_list_items",
            "id": id,
            "prices": items
                .iter()
                .map(|item| serde_json::json!({ "product_id": item.product_id, "price_minor": item.price.0 }))
                .collect::<Vec<_>>(),
        });
        record(tx, state, by, details)
    })
}

/// Takes products out of a list: they sell at the normal price again.
pub fn remove_price_list_items(
    state: &AppState,
    by: &Id,
    id: &Id,
    product_ids: &[Id],
) -> Result<(), AppError> {
    state.db.write(|tx| {
        pricing::remove_price_list_items(tx, id, product_ids)?;
        let details = serde_json::json!({
            "what": "price_list_items_removed",
            "id": id,
            "product_ids": product_ids,
        });
        record(tx, state, by, details)
    })
}

/// Every tier, archived ones too, by sort then name.
pub fn tiers(state: &AppState) -> Result<Vec<Tier>, AppError> {
    state.db.read(|conn| Ok(pricing::tiers(conn)?))
}

/// Adds or changes a tier (PRC-03).
pub fn save_tier(state: &AppState, by: &Id, input: &TierSave) -> Result<Id, AppError> {
    state.db.write(|tx| {
        let tier = TierInput {
            name: input.name.clone(),
            price_list_id: input.price_list_id.clone(),
            discount: input.discount,
            sort: input.sort,
        };
        let id = match &input.id {
            Some(id) => {
                pricing::update_tier(tx, id, &tier)?;
                id.clone()
            }
            None => pricing::create_tier(tx, &tier)?,
        };
        let details = serde_json::json!({
            "what": "tier",
            "id": id,
            "name": input.name.trim(),
            "price_list_id": input.price_list_id,
            "discount_bps": input.discount.0,
            "new": input.id.is_none(),
        });
        record(tx, state, by, details)?;
        Ok(id)
    })
}

/// Archives a tier: it gives no prices or discount from now on, even to customers who have it.
pub fn archive_tier(state: &AppState, by: &Id, id: &Id) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        pricing::archive_tier(tx, id, clock.now())?;
        record(
            tx,
            state,
            by,
            serde_json::json!({ "what": "tier_archived", "id": id }),
        )
    })
}

/// Every promotion, oldest first.
pub fn promotions(state: &AppState) -> Result<Vec<Promotion>, AppError> {
    state.db.read(|conn| Ok(pricing::promotions(conn)?))
}

/// Adds or changes a promotion (PRC-04). A tier, if chosen, must exist.
pub fn save_promotion(state: &AppState, by: &Id, input: &PromotionSave) -> Result<Id, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        if let Some(tier_id) = &input.promotion.tier_id {
            pricing::tier(tx, tier_id)?;
        }
        let id = match &input.id {
            Some(id) => {
                pricing::update_promotion(tx, id, &input.promotion)?;
                id.clone()
            }
            None => pricing::create_promotion(tx, &input.promotion, clock.now())?,
        };
        let details = serde_json::json!({
            "what": "promotion",
            "id": id,
            "promotion": input.promotion,
            "new": input.id.is_none(),
        });
        record(tx, state, by, details)?;
        Ok(id)
    })
}

/// Switches a promotion on or off (PRC-04 "active switch").
pub fn set_promotion_active(
    state: &AppState,
    by: &Id,
    id: &Id,
    active: bool,
) -> Result<(), AppError> {
    state.db.write(|tx| {
        pricing::set_promotion_active(tx, id, active)?;
        record(
            tx,
            state,
            by,
            serde_json::json!({ "what": "promotion_active", "id": id, "active": active }),
        )
    })
}

fn record(
    tx: &Transaction,
    state: &AppState,
    by: &Id,
    details: serde_json::Value,
) -> Result<(), AppError> {
    audit_log::record(
        tx,
        state.clock.as_ref(),
        PRICING_CHANGE,
        details,
        Some(by.clone()),
    )
}
