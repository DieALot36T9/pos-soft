//! The shapes that go in and out of the pricing engine.

use std::collections::HashMap;

use chrono::{DateTime, Datelike};
use pos_core::{AppError, Bps, Id, Money, Qty, Timestamp};
use serde::{Deserialize, Serialize};

use crate::{PromoRule, Promotion};

/// What pricing needs to know about a product. `app` fills it in from catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricingProduct {
    pub id: Id,
    /// The normal sale price for one unit (one piece, one kg). Example: 50.00.
    pub retail_price: Money,
    /// Category ids from the top category down to the product's own category.
    /// A promotion on any of them covers the product.
    pub category_path: Vec<Id>,
    pub brand_id: Option<Id>,
}

/// A moment in time with the shop's local offset, so promotions can check the local weekday.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalTime {
    /// The moment, in UTC.
    pub at: Timestamp,
    /// Minutes between local time and UTC at that moment. Dhaka is +360.
    pub offset_minutes: i32,
}

impl LocalTime {
    /// The local weekday as a `days_mask` bit: Monday is 1, Tuesday 2, … Sunday 64.
    pub(crate) fn weekday_bit(&self) -> Result<u8, AppError> {
        let local_ms = i64::from(self.offset_minutes)
            .checked_mul(60_000)
            .and_then(|shift| self.at.0.checked_add(shift))
            .ok_or_else(|| AppError::internal("local time overflowed"))?;
        let moment = DateTime::from_timestamp_millis(local_ms)
            .ok_or_else(|| AppError::internal("local time is out of range"))?;
        Ok(1 << moment.weekday().num_days_from_monday())
    }
}

/// A discount the cashier types in: a percentage or an amount (CHK-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ManualDiscount {
    /// Example: `Percent(Bps(500))` is 5% off.
    Percent(Bps),
    /// Example: `Amount(Money(1000))` is 10.00 off.
    Amount(Money),
}

/// The cart's tier, as the engine needs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierRule {
    pub id: Id,
    /// Taken off lines that have no promotion (PRC-03). Example: `Bps(200)` is 2%.
    pub discount: Bps,
}

/// Everything the engine needs besides the cart lines. `load_context` builds it from the
/// database. Tests build it by hand, because `price_cart` never reads the database or clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricingContext {
    pub now: LocalTime,
    /// The cart's tier. None if no customer is attached or the tier is archived.
    pub tier: Option<TierRule>,
    /// Prices from the cart's price list, by product id. Products not here use retail.
    pub list_prices: HashMap<Id, Money>,
    /// Promotions that might apply. The engine still checks dates, days, tier and the switch.
    pub promotions: Vec<Promotion>,
}

/// One cart line going into the engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceLineIn {
    /// Any number that lets the caller match the result to its line.
    pub key: usize,
    pub product: PricingProduct,
    pub qty: Qty,
    /// The price printed on a scale sticker. The line costs exactly this (CHK-08).
    pub embedded_price: Option<Money>,
    /// A unit price the admin typed in (CHK-13).
    pub price_override: Option<Money>,
    /// A discount the cashier typed in for this line.
    pub manual: Option<ManualDiscount>,
}

/// The promotion applied to a line and its share of the saving.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedPromo {
    pub promotion_id: Id,
    /// Shown on the receipt. Example: "Buy 2 get 1".
    pub name: String,
    pub rule: PromoRule,
    /// This line's part of the saving.
    pub discount: Money,
}

/// One priced cart line. `amount = base − promo − tier − manual − invoice share`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceLineOut {
    pub key: usize,
    /// The price for one unit: the override, the price list price or retail.
    pub unit_price: Money,
    /// unit price × quantity, or the scale sticker price.
    pub base: Money,
    pub promo: Option<AppliedPromo>,
    pub tier_discount: Money,
    pub manual_discount: Money,
    /// This line's part of the invoice discount.
    pub invoice_share: Money,
    /// What the customer pays for the line, before tax is worked out.
    pub amount: Money,
}

impl PriceLineOut {
    /// The promotion saving on this line, or zero if there is none.
    pub fn promo_discount(&self) -> Money {
        match &self.promo {
            Some(applied) => applied.discount,
            None => Money::zero(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekday_bit_follows_the_local_date() {
        // 2026-06-01 is a Monday. 23:00 UTC on Sunday 31 May is 05:00 Monday in Dhaka.
        let sunday_late_utc = LocalTime {
            at: Timestamp(1_780_268_400_000),
            offset_minutes: 0,
        };
        assert_eq!(sunday_late_utc.weekday_bit().unwrap(), 0b100_0000);
        let monday_in_dhaka = LocalTime {
            offset_minutes: 360,
            ..sunday_late_utc
        };
        assert_eq!(monday_in_dhaka.weekday_bit().unwrap(), 0b000_0001);
    }

    #[test]
    fn manual_discount_is_sent_as_kind_and_value() {
        let json = serde_json::to_string(&ManualDiscount::Percent(Bps(500))).unwrap();
        assert_eq!(json, r#"{"kind":"percent","value":500}"#);
    }
}
