//! Price lists, tiers and promotions, and the pricing engine.
//!
//! Page: `docs/05 Modules/Module - pricing.md`. The engine (`price_cart`) is a pure
//! function: `load_context` reads the database, then `price_cart` does the maths.

// Money maths must never use floating-point numbers.
#![deny(clippy::float_arithmetic)]

mod cart;
mod engine;
mod error;
mod load;
mod price_lists;
mod promotion;
mod promotion_store;
mod tiers;

pub use cart::{
    AppliedPromo, LocalTime, ManualDiscount, PriceLineIn, PriceLineOut, PricingContext,
    PricingProduct, TierRule,
};
pub use engine::{manual_discount_bps, price_cart};
pub use error::PricingError;
pub use load::load_context;
pub use price_lists::{
    PriceList, PriceListItem, PriceListUpdate, create_price_list, price_list_items, price_lists,
    remove_price_list_items, set_price_list_items, update_price_list,
};
pub use promotion::{EVERY_DAY, PromoRule, PromoTarget, Promotion, PromotionInput};
pub use promotion_store::{
    create_promotion, promotion, promotions, set_promotion_active, update_promotion,
};
pub use tiers::{Tier, TierInput, archive_tier, create_tier, tier, tiers, update_tier};

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "pricing",
        migrations: MIGRATIONS,
    }
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/001_init.sql"),
}];
