//! Helpers shared by this crate's test files.
//! Each test file uses only some of them, so unused ones are allowed.
#![allow(dead_code)]

use std::collections::HashMap;

use pos_core::{Bps, Id, Money, Qty, Timestamp, parse_money};
use pricing::{
    LocalTime, ManualDiscount, PriceLineIn, PricingContext, PricingProduct, PromoRule, PromoTarget,
    Promotion,
};
use rusqlite::Connection;

/// An in-memory database built by this module's own migrations (Module Rules R6).
pub fn test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory database must open");
    // The app turns foreign keys on for every connection, so tests do too.
    conn.execute_batch("PRAGMA foreign_keys = ON")
        .expect("foreign keys must switch on");
    for migration in pricing::migrations().migrations {
        conn.execute_batch(migration.sql)
            .expect("pricing migrations must run");
    }
    conn
}

/// Money from text with 2 decimals. Example: `money("50.00")`.
pub fn money(text: &str) -> Money {
    parse_money(text, 2).expect("test amount must be valid")
}

/// A quantity from text with 3 decimals. Example: `qty("1.234")`.
pub fn qty(text: &str) -> Qty {
    Qty(parse_money(text, 3).expect("test quantity must be valid").0)
}

/// Monday 2026-06-01 12:00 UTC, in a shop at UTC+6 (Dhaka), so 18:00 local time.
pub const MONDAY_NOON_UTC: Timestamp = Timestamp(1_780_315_200_000);
pub const DHAKA_OFFSET: i32 = 360;
pub const ONE_DAY_MS: i64 = 86_400_000;

/// "Now" for the engine: Monday 18:00 in Dhaka.
pub fn monday() -> LocalTime {
    LocalTime {
        at: MONDAY_NOON_UTC,
        offset_minutes: DHAKA_OFFSET,
    }
}

/// A product with a retail price and nothing else.
pub fn product(retail: &str) -> PricingProduct {
    PricingProduct {
        id: Id::new(),
        retail_price: money(retail),
        category_path: Vec::new(),
        brand_id: None,
    }
}

/// A cart line for `product` with nothing special about it.
pub fn line(key: usize, product: &PricingProduct, quantity: &str) -> PriceLineIn {
    PriceLineIn {
        key,
        product: product.clone(),
        qty: qty(quantity),
        embedded_price: None,
        price_override: None,
        manual: None,
    }
}

/// A context with retail prices only: no tier, no price list, no promotions.
pub fn retail_context() -> PricingContext {
    PricingContext {
        now: monday(),
        tier: None,
        list_prices: HashMap::new(),
        promotions: Vec::new(),
    }
}

/// An active promotion on any day, with no dates and no tier, created at `created_at`.
pub fn promotion(name: &str, rule: PromoRule, target: PromoTarget, created_at: i64) -> Promotion {
    Promotion {
        id: Id::new(),
        name: name.to_string(),
        rule,
        starts_at: None,
        ends_at: None,
        days_mask: 0b111_1111,
        tier_id: None,
        targets: vec![target],
        active: true,
        created_at: Timestamp(created_at),
    }
}

/// A percent-off rule. Example: `percent_off(1000)` is 10% off.
pub fn percent_off(bps: u32) -> PromoRule {
    PromoRule::PercentOff { percent: Bps(bps) }
}

/// A manual percent discount. Example: `manual_percent(500)` is 5%.
pub fn manual_percent(bps: u32) -> Option<ManualDiscount> {
    Some(ManualDiscount::Percent(Bps(bps)))
}

/// A manual amount discount. Example: `manual_amount("10.00")`.
pub fn manual_amount(amount: &str) -> Option<ManualDiscount> {
    Some(ManualDiscount::Amount(money(amount)))
}
