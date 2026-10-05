//! Property tests (proptest): for any cart, no line goes below zero and the
//! discounts never add up to more than the lines cost (Module - pricing, "Tests").

mod common;

use common::*;
use pos_core::{Bps, Id, Money, Qty};
use pricing::{
    ManualDiscount, PriceLineIn, PricingProduct, PromoRule, PromoTarget, TierRule, price_cart,
};
use proptest::prelude::*;

/// Three products with the given retail prices, all in one category. Random carts
/// often repeat a product, so the combined quantity gets tested too.
fn products(retail_prices: &[i64]) -> Vec<PricingProduct> {
    let category = Id::new();
    let mut list = Vec::new();
    for retail in retail_prices {
        let mut item = product("0.00");
        item.retail_price = Money(*retail);
        item.category_path = vec![category.clone()];
        list.push(item);
    }
    list
}

fn rule_strategy() -> impl Strategy<Value = PromoRule> {
    prop_oneof![
        (1u32..=10_000).prop_map(|bps| PromoRule::PercentOff { percent: Bps(bps) }),
        (1i64..50_000).prop_map(|amount| PromoRule::AmountOff {
            amount_per_unit: Money(amount)
        }),
        (0i64..50_000).prop_map(|price| PromoRule::FixedPrice {
            price_per_unit: Money(price)
        }),
        (1u32..5, 1u32..5).prop_map(|(buy, get)| PromoRule::BuyXGetY { buy, get }),
        (2u32..6, 0i64..100_000).prop_map(|(bundle_qty, price)| PromoRule::MultiBuy {
            bundle_qty,
            bundle_price: Money(price)
        }),
    ]
}

fn discount_strategy() -> impl Strategy<Value = Option<ManualDiscount>> {
    prop_oneof![
        Just(None),
        (0u32..=12_000).prop_map(|bps| Some(ManualDiscount::Percent(Bps(bps)))),
        (0i64..200_000).prop_map(|amount| Some(ManualDiscount::Amount(Money(amount)))),
    ]
}

/// (product index, quantity in thousandths, embedded price?, override?, manual discount)
type LineSeed = (usize, i64, Option<i64>, Option<i64>, Option<ManualDiscount>);

fn line_strategy() -> impl Strategy<Value = LineSeed> {
    (
        0usize..3,
        0i64..20_000,
        proptest::option::weighted(0.2, 0i64..100_000),
        proptest::option::weighted(0.2, 0i64..100_000),
        discount_strategy(),
    )
}

fn build_lines(seeds: &[LineSeed], products: &[PricingProduct]) -> Vec<PriceLineIn> {
    let mut lines = Vec::new();
    for (key, (index, quantity, embedded, price_override, manual)) in seeds.iter().enumerate() {
        lines.push(PriceLineIn {
            key,
            product: products[*index].clone(),
            qty: Qty(*quantity),
            embedded_price: embedded.map(Money),
            price_override: price_override.map(Money),
            manual: *manual,
        });
    }
    lines
}

proptest! {
    #[test]
    fn amounts_are_never_negative_and_discounts_never_exceed_the_base(
        retail_prices in proptest::collection::vec(0i64..100_000, 3),
        seeds in proptest::collection::vec(line_strategy(), 0..8),
        rules in proptest::collection::vec(rule_strategy(), 0..4),
        tier_bps in proptest::option::of(0u32..=10_000),
        invoice in discount_strategy(),
    ) {
        let items = products(&retail_prices);
        let mut ctx = retail_context();
        ctx.tier = tier_bps.map(|bps| TierRule { id: Id::new(), discount: Bps(bps) });
        for (index, rule) in rules.into_iter().enumerate() {
            let target = if index % 2 == 0 {
                PromoTarget::Product(items[index % 3].id.clone())
            } else {
                PromoTarget::Category(items[0].category_path[0].clone())
            };
            ctx.promotions.push(promotion("Random", rule, target, index as i64));
        }
        let lines = build_lines(&seeds, &items);

        let out = price_cart(&ctx, &lines, invoice).unwrap();

        prop_assert_eq!(out.len(), lines.len());
        let mut base_total = 0i64;
        let mut discount_total = 0i64;
        for line in &out {
            let discounts = line.promo_discount().0
                + line.tier_discount.0
                + line.manual_discount.0
                + line.invoice_share.0;
            prop_assert!(line.amount.0 >= 0, "negative amount: {:?}", line);
            prop_assert!(line.promo_discount().0 >= 0);
            prop_assert!(line.tier_discount.0 >= 0);
            prop_assert!(line.manual_discount.0 >= 0);
            prop_assert!(line.invoice_share.0 >= 0);
            prop_assert_eq!(line.amount.0, line.base.0 - discounts);
            base_total += line.base.0;
            discount_total += discounts;
        }
        prop_assert!(discount_total <= base_total);
    }
}
