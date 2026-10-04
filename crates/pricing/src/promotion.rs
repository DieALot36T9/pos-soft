//! Promotions: what they are, when they run, which products they cover, and how much
//! one saves on a group of cart lines (PRC-04). The engine picks the best one (ADR-019).

use pos_core::{AppError, Bps, Id, Money, Qty, Timestamp, mul_qty, percent_of};
use serde::{Deserialize, Serialize};

use crate::{PricingProduct, TierRule};

/// Every day of the week. Bit 0 is Monday, bit 6 is Sunday.
pub const EVERY_DAY: u8 = 0b111_1111;

/// What a promotion gives (PRC-04). Quantities in `BuyXGetY` and `MultiBuy` are whole units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PromoRule {
    /// A percentage off. Example: 10% off all detergents.
    PercentOff { percent: Bps },
    /// An amount off each unit (each kg for weighed items). Example: 5.00 off each biscuit.
    AmountOff { amount_per_unit: Money },
    /// A set price for each unit. Example: oil 1L for 180.00.
    FixedPrice { price_per_unit: Money },
    /// Buy some, get some free. Example: buy 2 soaps, get 1 free.
    BuyXGetY { buy: u32, get: u32 },
    /// A bundle for a set price. Example: 3 noodles for 100.00.
    MultiBuy {
        bundle_qty: u32,
        bundle_price: Money,
    },
}

impl PromoRule {
    /// The name stored in the `kind` column. Example: "buy_x_get_y".
    pub fn kind(&self) -> &'static str {
        match self {
            PromoRule::PercentOff { .. } => "percent_off",
            PromoRule::AmountOff { .. } => "amount_off",
            PromoRule::FixedPrice { .. } => "fixed_price",
            PromoRule::BuyXGetY { .. } => "buy_x_get_y",
            PromoRule::MultiBuy { .. } => "multi_buy",
        }
    }
}

/// What a promotion covers: one product, a category (with its subcategories) or a brand.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum PromoTarget {
    Product(Id),
    Category(Id),
    Brand(Id),
}

impl PromoTarget {
    /// The name stored in the `target_type` column. Example: "category".
    pub fn type_name(&self) -> &'static str {
        match self {
            PromoTarget::Product(_) => "product",
            PromoTarget::Category(_) => "category",
            PromoTarget::Brand(_) => "brand",
        }
    }

    /// The product, category or brand id.
    pub fn id(&self) -> &Id {
        match self {
            PromoTarget::Product(id) | PromoTarget::Category(id) | PromoTarget::Brand(id) => id,
        }
    }
}

/// A saved promotion with its limits and targets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Promotion {
    pub id: Id,
    pub name: String,
    pub rule: PromoRule,
    /// The first moment it runs. None: it has always run.
    pub starts_at: Option<Timestamp>,
    /// The first moment it no longer runs. None: it never ends.
    pub ends_at: Option<Timestamp>,
    /// The days it runs, in local time. Bit 0 = Monday … bit 6 = Sunday. 127 = every day.
    pub days_mask: u8,
    /// If set, only carts with this tier get the promotion.
    pub tier_id: Option<Id>,
    /// Sorted, with no repeats.
    pub targets: Vec<PromoTarget>,
    /// The admin's on/off switch.
    pub active: bool,
    /// When it was made. When two promotions save the same, the older one wins (ADR-019).
    pub created_at: Timestamp,
}

impl Promotion {
    /// True if the promotion covers this product: the product itself, any category on its
    /// path (so a parent category covers its subcategories) or its brand.
    pub(crate) fn covers(&self, product: &PricingProduct) -> bool {
        self.targets.iter().any(|target| match target {
            PromoTarget::Product(id) => *id == product.id,
            PromoTarget::Category(id) => product.category_path.contains(id),
            PromoTarget::Brand(id) => product.brand_id.as_ref() == Some(id),
        })
    }

    /// True if the promotion is switched on, has started, hasn't ended, runs on this
    /// weekday and allows the cart's tier.
    pub(crate) fn runs_now(
        &self,
        now: Timestamp,
        weekday_bit: u8,
        tier: Option<&TierRule>,
    ) -> bool {
        let started = self.starts_at.is_none_or(|start| now >= start);
        let not_ended = self.ends_at.is_none_or(|end| now < end);
        let right_day = self.days_mask & weekday_bit != 0;
        let right_tier = match &self.tier_id {
            None => true,
            Some(needed) => tier.is_some_and(|cart_tier| cart_tier.id == *needed),
        };
        self.active && started && not_ended && right_day && right_tier
    }
}

/// What the admin enters to make or change a promotion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionInput {
    pub name: String,
    pub rule: PromoRule,
    pub starts_at: Option<Timestamp>,
    pub ends_at: Option<Timestamp>,
    /// Bit 0 = Monday … bit 6 = Sunday. Use `EVERY_DAY` for every day.
    pub days_mask: u8,
    pub tier_id: Option<Id>,
    pub targets: Vec<PromoTarget>,
}

/// How much a promotion saves on one product's lines together (PRC-05: combined quantity).
/// `unit_price` is the price the lines would have without it; `base` is their total.
/// The saving is never below zero and never more than `base`, so a promotion can't
/// raise a price (PRC-06 "fixed price above retail").
pub(crate) fn saving(
    rule: &PromoRule,
    unit_price: Money,
    qty: Qty,
    base: Money,
) -> Result<Money, AppError> {
    // Buy X get Y and multi-buy count whole units only. Example: 7.000 → 7.
    let whole_units = i128::from(qty.0 / 1000);
    let price = i128::from(unit_price.0);
    let saving: i128 = match rule {
        PromoRule::PercentOff { percent } => i128::from(percent_of(base, *percent)?.0),
        PromoRule::AmountOff { amount_per_unit } => i128::from(mul_qty(*amount_per_unit, qty)?.0),
        PromoRule::FixedPrice { price_per_unit } => {
            if *price_per_unit >= unit_price {
                0
            } else {
                i128::from(base.0) - i128::from(mul_qty(*price_per_unit, qty)?.0)
            }
        }
        PromoRule::BuyXGetY { buy, get } => {
            // Rule PRC-06: soap × 7, buy 2 get 1 → ⌊7 ÷ 3⌋ = 2 sets → 2 free.
            let set_size = i128::from(*buy) + i128::from(*get);
            let free_units = whole_units / set_size * i128::from(*get);
            free_units * price
        }
        PromoRule::MultiBuy {
            bundle_qty,
            bundle_price,
        } => {
            // Rule PRC-06: noodles 40.00 × 7, 3 for 100.00 → 2 bundles save 20.00 each.
            let normal_bundle_price = i128::from(*bundle_qty) * price;
            let saved_per_bundle = normal_bundle_price - i128::from(bundle_price.0);
            let bundles = whole_units / i128::from(*bundle_qty);
            if saved_per_bundle > 0 {
                bundles * saved_per_bundle
            } else {
                0
            }
        }
    };
    let capped = saving.clamp(0, i128::from(base.0).max(0));
    i64::try_from(capped)
        .map(Money)
        .map_err(|_| AppError::internal("promotion saving is too large"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buy_x_get_y_and_multi_buy_ignore_part_units() {
        // 2.900 kg is 2 whole units: not enough for "buy 2 get 1".
        let rule = PromoRule::BuyXGetY { buy: 2, get: 1 };
        assert_eq!(
            saving(&rule, Money(100), Qty(2900), Money(290)).unwrap(),
            Money(0)
        );
        let bundle = PromoRule::MultiBuy {
            bundle_qty: 3,
            bundle_price: Money(200),
        };
        assert_eq!(
            saving(&bundle, Money(100), Qty(3500), Money(350)).unwrap(),
            Money(100)
        );
    }

    #[test]
    fn a_saving_is_never_more_than_the_base() {
        let rule = PromoRule::AmountOff {
            amount_per_unit: Money(500),
        };
        assert_eq!(
            saving(&rule, Money(400), Qty(2000), Money(800)).unwrap(),
            Money(800)
        );
    }

    #[test]
    fn kinds_match_the_database_names() {
        assert_eq!(
            PromoRule::PercentOff { percent: Bps(1) }.kind(),
            "percent_off"
        );
        assert_eq!(
            PromoRule::MultiBuy {
                bundle_qty: 2,
                bundle_price: Money(1)
            }
            .kind(),
            "multi_buy"
        );
        assert_eq!(PromoTarget::Brand(Id::new()).type_name(), "brand");
    }
}
