//! Loading what the engine needs from the database (the only I/O around the engine).

use std::collections::HashMap;

use pos_core::Id;
use rusqlite::Connection;

use crate::price_lists::prices_for;
use crate::promotion_store::not_ended;
use crate::tiers::tier;
use crate::{LocalTime, PricingContext, PricingError, PricingProduct, TierRule};

/// Reads the tier, price list prices and promotions for these products, ready for
/// `price_cart`. `app` passes the cart's tier and price list and the local time.
/// A price list chosen for the cart (CHK-17) wins over the tier's own price list.
/// Archived tiers and switched-off price lists change nothing. Unknown ids are errors.
pub fn load_context(
    conn: &Connection,
    products: &[PricingProduct],
    tier_id: Option<&Id>,
    price_list_id: Option<&Id>,
    now: LocalTime,
) -> Result<PricingContext, PricingError> {
    let mut tier_rule = None;
    let mut tier_price_list = None;
    if let Some(id) = tier_id {
        let saved = tier(conn, id)?;
        if saved.archived_at.is_none() {
            tier_rule = Some(TierRule {
                id: saved.id,
                discount: saved.discount,
            });
            tier_price_list = saved.price_list_id;
        }
    }

    let list_id = price_list_id.cloned().or(tier_price_list);
    let list_prices = match &list_id {
        Some(id) => prices_for(conn, id, products)?,
        None => HashMap::new(),
    };

    let mut promotions = not_ended(conn, now.at)?;
    promotions.retain(|promo| products.iter().any(|product| promo.covers(product)));

    Ok(PricingContext {
        now,
        tier: tier_rule,
        list_prices,
        promotions,
    })
}
