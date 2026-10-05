//! The pricing engine's promotion rules (PRC-04, PRC-05), tested on the pure
//! `price_cart` function. No database is needed.

mod common;

use common::*;
use pos_core::{Bps, Id, Money, Timestamp};
use pricing::{LocalTime, PromoRule, PromoTarget, TierRule, price_cart};

#[test]
fn prc_04_amount_off_each_unit() {
    let biscuit = product("20.00");
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "5.00 off each",
        PromoRule::AmountOff {
            amount_per_unit: money("5.00"),
        },
        PromoTarget::Product(biscuit.id.clone()),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &biscuit, "3")], None).unwrap();

    assert_eq!(out[0].promo_discount(), money("15.00"));
}

#[test]
fn prc_04_amount_off_larger_than_the_price_stops_at_zero() {
    let biscuit = product("4.00");
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "5.00 off each",
        PromoRule::AmountOff {
            amount_per_unit: money("5.00"),
        },
        PromoTarget::Product(biscuit.id.clone()),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &biscuit, "2")], None).unwrap();

    assert_eq!(out[0].promo_discount(), money("8.00"));
    assert_eq!(out[0].amount, Money::zero());
}

#[test]
fn prc_04_fixed_price_below_retail_applies() {
    let oil = product("170.00");
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "Oil 1L for 160.00",
        PromoRule::FixedPrice {
            price_per_unit: money("160.00"),
        },
        PromoTarget::Product(oil.id.clone()),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &oil, "2")], None).unwrap();

    assert_eq!(out[0].promo_discount(), money("20.00"));
    assert_eq!(out[0].amount, money("320.00"));
}

#[test]
fn prc_04_fixed_price_above_the_price_list_price_is_ignored() {
    // Retail 170.00, member price 150.00, promotion 160.00: it would raise the price.
    let oil = product("170.00");
    let mut ctx = retail_context();
    ctx.list_prices.insert(oil.id.clone(), money("150.00"));
    ctx.promotions.push(promotion(
        "Oil 1L for 160.00",
        PromoRule::FixedPrice {
            price_per_unit: money("160.00"),
        },
        PromoTarget::Product(oil.id.clone()),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &oil, "1")], None).unwrap();

    assert_eq!(out[0].promo, None);
    assert_eq!(out[0].amount, money("150.00"));
}

#[test]
fn prc_04_multi_buy_that_costs_more_than_single_units_is_ignored() {
    let noodles = product("30.00");
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

    let out = price_cart(&ctx, &[line(0, &noodles, "3")], None).unwrap();

    assert_eq!(out[0].promo, None);
}

#[test]
fn prc_04_day_of_week_mask_uses_the_local_day() {
    let soap = product("50.00");
    let mut tuesday_only = promotion(
        "Tuesday deal",
        percent_off(1000),
        PromoTarget::Product(soap.id.clone()),
        1,
    );
    tuesday_only.days_mask = 0b000_0010; // bit 0 = Monday, bit 1 = Tuesday
    let mut ctx = retail_context();
    ctx.promotions.push(tuesday_only);

    // Monday 18:00 in Dhaka: not Tuesday.
    let out = price_cart(&ctx, &[line(0, &soap, "1")], None).unwrap();
    assert_eq!(out[0].promo, None);

    // Monday 20:00 UTC is already Tuesday 02:00 in Dhaka.
    ctx.now = LocalTime {
        at: Timestamp(MONDAY_NOON_UTC.0 + 8 * 3_600_000),
        offset_minutes: DHAKA_OFFSET,
    };
    let out = price_cart(&ctx, &[line(0, &soap, "1")], None).unwrap();
    assert_eq!(out[0].promo_discount(), money("5.00"));
}

#[test]
fn prc_04_promotion_that_has_not_started_is_not_applied() {
    let soap = product("50.00");
    let mut later = promotion(
        "Next week",
        percent_off(1000),
        PromoTarget::Product(soap.id.clone()),
        1,
    );
    later.starts_at = Some(Timestamp(MONDAY_NOON_UTC.0 + ONE_DAY_MS));
    let mut ctx = retail_context();
    ctx.promotions.push(later);

    let out = price_cart(&ctx, &[line(0, &soap, "1")], None).unwrap();

    assert_eq!(out[0].promo, None);
}

#[test]
fn prc_04_promotion_runs_from_its_start_up_to_but_not_including_its_end() {
    let soap = product("50.00");
    let mut ends_now = promotion(
        "Ends now",
        percent_off(1000),
        PromoTarget::Product(soap.id.clone()),
        1,
    );
    ends_now.starts_at = Some(MONDAY_NOON_UTC);
    ends_now.ends_at = Some(Timestamp(MONDAY_NOON_UTC.0 + 1));
    let mut ctx = retail_context();
    ctx.promotions.push(ends_now);

    let out = price_cart(&ctx, &[line(0, &soap, "1")], None).unwrap();
    assert!(out[0].promo.is_some());

    ctx.now.at = Timestamp(MONDAY_NOON_UTC.0 + 1);
    let out = price_cart(&ctx, &[line(0, &soap, "1")], None).unwrap();
    assert_eq!(out[0].promo, None);
}

#[test]
fn prc_04_switched_off_promotion_is_not_applied() {
    let soap = product("50.00");
    let mut off = promotion(
        "Off",
        percent_off(1000),
        PromoTarget::Product(soap.id.clone()),
        1,
    );
    off.active = false;
    let mut ctx = retail_context();
    ctx.promotions.push(off);

    let out = price_cart(&ctx, &[line(0, &soap, "1")], None).unwrap();

    assert_eq!(out[0].promo, None);
}

#[test]
fn prc_04_tier_only_promotion_needs_that_tier() {
    let gold = Id::new();
    let soap = product("50.00");
    let mut gold_only = promotion(
        "Gold 10%",
        percent_off(1000),
        PromoTarget::Product(soap.id.clone()),
        1,
    );
    gold_only.tier_id = Some(gold.clone());
    let mut ctx = retail_context();
    ctx.promotions.push(gold_only);

    // No tier on the cart.
    let out = price_cart(&ctx, &[line(0, &soap, "1")], None).unwrap();
    assert_eq!(out[0].promo, None);

    // Another tier.
    ctx.tier = Some(TierRule {
        id: Id::new(),
        discount: Bps(0),
    });
    let out = price_cart(&ctx, &[line(0, &soap, "1")], None).unwrap();
    assert_eq!(out[0].promo, None);

    // The Gold tier.
    ctx.tier = Some(TierRule {
        id: gold,
        discount: Bps(0),
    });
    let out = price_cart(&ctx, &[line(0, &soap, "1")], None).unwrap();
    assert_eq!(out[0].promo_discount(), money("5.00"));
}

#[test]
fn prc_04_parent_category_promotion_applies_to_a_child_category() {
    let household = Id::new();
    let detergents = Id::new();
    let mut powder = product("250.00");
    powder.category_path = vec![household.clone(), detergents];
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "10% off household",
        percent_off(1000),
        PromoTarget::Category(household),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &powder, "1")], None).unwrap();

    assert_eq!(out[0].promo_discount(), money("25.00"));
}

#[test]
fn prc_04_brand_promotion_applies_to_that_brand_only() {
    let brand = Id::new();
    let mut branded = product("100.00");
    branded.brand_id = Some(brand.clone());
    let other = product("100.00");
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "10% off the brand",
        percent_off(1000),
        PromoTarget::Brand(brand),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &branded, "1"), line(1, &other, "1")], None).unwrap();

    assert_eq!(out[0].promo_discount(), money("10.00"));
    assert_eq!(out[1].promo, None);
}

#[test]
fn prc_05_same_product_on_two_lines_uses_the_combined_quantity() {
    let soap = product("50.00");
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "Buy 2 get 1",
        PromoRule::BuyXGetY { buy: 2, get: 1 },
        PromoTarget::Product(soap.id.clone()),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &soap, "2"), line(1, &soap, "1")], None).unwrap();

    // 3 soaps in total, so one is free: 50.00, split by each line's base (100.00 and 50.00).
    assert_eq!(out[0].promo_discount(), money("33.33"));
    assert_eq!(out[1].promo_discount(), money("16.67"));
    assert!(out[1].promo.is_some());
}

#[test]
fn prc_05_tie_goes_to_the_promotion_created_first() {
    let soap = product("50.00");
    let mut ctx = retail_context();
    // Listed newest first on purpose: the engine must not rely on the order given.
    ctx.promotions.push(promotion(
        "Newer 10%",
        percent_off(1000),
        PromoTarget::Product(soap.id.clone()),
        20,
    ));
    ctx.promotions.push(promotion(
        "Older 10%",
        percent_off(1000),
        PromoTarget::Product(soap.id.clone()),
        10,
    ));

    let out = price_cart(&ctx, &[line(0, &soap, "1")], None).unwrap();

    assert_eq!(out[0].promo.as_ref().unwrap().name, "Older 10%");
}
