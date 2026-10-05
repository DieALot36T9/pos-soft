//! Every worked example in PRC-06 (Req - Pricing and Promotions), one test each.

mod common;

use common::*;
use pos_core::{Id, Money, Timestamp};
use pricing::{PromoRule, PromoTarget, price_cart};

#[test]
fn prc_06_buy_2_get_1_soap_50_times_7() {
    let soap = product("50.00");
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "Buy 2 get 1",
        PromoRule::BuyXGetY { buy: 2, get: 1 },
        PromoTarget::Product(soap.id.clone()),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &soap, "7")], None).unwrap();

    // Free = ⌊7 ÷ 3⌋ = 2 → discount 100.00, line 250.00.
    assert_eq!(out[0].base, money("350.00"));
    assert_eq!(out[0].promo_discount(), money("100.00"));
    assert_eq!(out[0].amount, money("250.00"));
}

#[test]
fn prc_06_multi_buy_noodles_3_for_100() {
    let noodles = product("40.00");
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "3 for 100.00",
        PromoRule::MultiBuy {
            bundle_qty: 3,
            bundle_price: money("100.00"),
        },
        PromoTarget::Product(noodles.id.clone()),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &noodles, "7")], None).unwrap();

    // 2 bundles = 200.00 + 1 × 40.00 = 240.00 → discount 40.00.
    assert_eq!(out[0].promo_discount(), money("40.00"));
    assert_eq!(out[0].amount, money("240.00"));
}

#[test]
fn prc_06_percent_off_detergent_10_percent() {
    let detergent = product("250.00");
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "10% off",
        percent_off(1000),
        PromoTarget::Product(detergent.id.clone()),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &detergent, "2")], None).unwrap();

    assert_eq!(out[0].promo_discount(), money("50.00"));
    assert_eq!(out[0].amount, money("450.00"));
}

#[test]
fn prc_06_fixed_price_above_retail_is_ignored() {
    let oil = product("170.00");
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "Oil 1L for 180.00",
        PromoRule::FixedPrice {
            price_per_unit: money("180.00"),
        },
        PromoTarget::Product(oil.id.clone()),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &oil, "1")], None).unwrap();

    // The promotion never raises the price.
    assert_eq!(out[0].promo, None);
    assert_eq!(out[0].amount, money("170.00"));
}

#[test]
fn prc_06_best_of_two_buy_2_get_1_beats_10_percent() {
    let soaps = Id::new();
    let mut soap = product("50.00");
    soap.category_path = vec![soaps.clone()];
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "10% off soaps",
        percent_off(1000),
        PromoTarget::Category(soaps),
        1,
    ));
    ctx.promotions.push(promotion(
        "Buy 2 get 1",
        PromoRule::BuyXGetY { buy: 2, get: 1 },
        PromoTarget::Product(soap.id.clone()),
        2,
    ));

    let out = price_cart(&ctx, &[line(0, &soap, "3")], None).unwrap();

    // Buy 2 get 1 (50.00) beats 10% (15.00). Only one promotion applies (ADR-019).
    let applied = out[0].promo.as_ref().unwrap();
    assert_eq!(applied.name, "Buy 2 get 1");
    assert_eq!(applied.discount, money("50.00"));
    assert_eq!(out[0].amount, money("100.00"));
}

#[test]
fn prc_06_weighed_beef_with_10_percent() {
    let beef = product("750.00");
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "10% off beef",
        percent_off(1000),
        PromoTarget::Product(beef.id.clone()),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &beef, "1.234")], None).unwrap();

    assert_eq!(out[0].base, money("925.50"));
    assert_eq!(out[0].promo_discount(), money("92.55"));
    assert_eq!(out[0].amount, money("832.95"));
}

#[test]
fn prc_06_promotion_that_ended_yesterday_is_not_applied() {
    let soap = product("50.00");
    let mut ctx = retail_context();
    let mut ended = promotion(
        "Last week's deal",
        percent_off(1000),
        PromoTarget::Product(soap.id.clone()),
        1,
    );
    ended.starts_at = Some(Timestamp(MONDAY_NOON_UTC.0 - 7 * ONE_DAY_MS));
    ended.ends_at = Some(Timestamp(MONDAY_NOON_UTC.0 - ONE_DAY_MS));
    ctx.promotions.push(ended);

    let out = price_cart(&ctx, &[line(0, &soap, "1")], None).unwrap();

    assert_eq!(out[0].promo, None);
    assert_eq!(out[0].amount, money("50.00"));
    assert_eq!(out[0].tier_discount, Money::zero());
}
