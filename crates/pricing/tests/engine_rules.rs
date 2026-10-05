//! The pricing engine's price and discount rules (PRC-01 to PRC-03, PRC-05, PRC-07,
//! CHK-08, CHK-12, CHK-13), tested on the pure `price_cart` function. No database is needed.

mod common;

use common::*;
use pos_core::{Bps, Id, Money};
use pricing::{ManualDiscount, PromoRule, PromoTarget, TierRule, manual_discount_bps, price_cart};

#[test]
fn prc_01_retail_price_with_no_list_tier_or_promotion() {
    let milk = product("90.00");

    let out = price_cart(&retail_context(), &[line(7, &milk, "2")], None).unwrap();

    assert_eq!(out[0].key, 7);
    assert_eq!(out[0].unit_price, money("90.00"));
    assert_eq!(out[0].base, money("180.00"));
    assert_eq!(out[0].promo, None);
    assert_eq!(out[0].tier_discount, Money::zero());
    assert_eq!(out[0].manual_discount, Money::zero());
    assert_eq!(out[0].invoice_share, Money::zero());
    assert_eq!(out[0].amount, money("180.00"));
}

#[test]
fn prc_02_price_list_price_is_used_and_others_fall_back_to_retail() {
    let rice = product("100.00");
    let salt = product("30.00");
    let mut ctx = retail_context();
    ctx.list_prices.insert(rice.id.clone(), money("95.00"));

    let out = price_cart(&ctx, &[line(0, &rice, "1"), line(1, &salt, "1")], None).unwrap();

    assert_eq!(out[0].unit_price, money("95.00"));
    assert_eq!(out[1].unit_price, money("30.00"));
}

#[test]
fn prc_03_gold_tier_member_prices_and_2_percent_on_lines_without_a_promotion() {
    let rice = product("100.00");
    let soap = product("50.00");
    let mut ctx = retail_context();
    ctx.tier = Some(TierRule {
        id: Id::new(),
        discount: Bps(200),
    });
    ctx.list_prices.insert(rice.id.clone(), money("95.00"));
    ctx.promotions.push(promotion(
        "Buy 2 get 1",
        PromoRule::BuyXGetY { buy: 2, get: 1 },
        PromoTarget::Product(soap.id.clone()),
        1,
    ));

    let out = price_cart(&ctx, &[line(0, &rice, "2"), line(1, &soap, "3")], None).unwrap();

    // Rice: member price 95.00 × 2 = 190.00, then 2% = 3.80 off.
    assert_eq!(out[0].base, money("190.00"));
    assert_eq!(out[0].tier_discount, money("3.80"));
    assert_eq!(out[0].amount, money("186.20"));
    // Soap has a promotion, so no tier discount.
    assert_eq!(out[1].promo_discount(), money("50.00"));
    assert_eq!(out[1].tier_discount, Money::zero());
}

#[test]
fn prc_05_manual_discount_applies_after_the_promotion() {
    let soap = product("50.00");
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "Buy 2 get 1",
        PromoRule::BuyXGetY { buy: 2, get: 1 },
        PromoTarget::Product(soap.id.clone()),
        1,
    ));
    let mut soaps = line(0, &soap, "3");
    soaps.manual = manual_percent(1000);

    let out = price_cart(&ctx, &[soaps], None).unwrap();

    // 150.00 − 50.00 promotion = 100.00, then 10% = 10.00.
    assert_eq!(out[0].manual_discount, money("10.00"));
    assert_eq!(out[0].amount, money("90.00"));
}

#[test]
fn chk_12_a_discount_can_not_make_a_line_negative() {
    let soap = product("50.00");
    let mut soaps = line(0, &soap, "1");
    soaps.manual = manual_amount("80.00");

    let out = price_cart(&retail_context(), &[soaps], None).unwrap();

    assert_eq!(out[0].manual_discount, money("50.00"));
    assert_eq!(out[0].amount, Money::zero());
}

#[test]
fn chk_12_a_negative_discount_is_refused() {
    let soap = product("50.00");
    let mut soaps = line(0, &soap, "1");
    soaps.manual = manual_amount("-1.00");

    assert!(price_cart(&retail_context(), &[soaps], None).is_err());
}

#[test]
fn chk_13_price_override_turns_off_promotion_and_tier_discount() {
    let soap = product("50.00");
    let mut ctx = retail_context();
    ctx.tier = Some(TierRule {
        id: Id::new(),
        discount: Bps(200),
    });
    ctx.promotions.push(promotion(
        "Buy 2 get 1",
        PromoRule::BuyXGetY { buy: 2, get: 1 },
        PromoTarget::Product(soap.id.clone()),
        1,
    ));
    let mut overridden = line(0, &soap, "3");
    overridden.price_override = Some(money("45.00"));

    let out = price_cart(&ctx, &[overridden], None).unwrap();

    assert_eq!(out[0].unit_price, money("45.00"));
    assert_eq!(out[0].base, money("135.00"));
    assert_eq!(out[0].promo, None);
    assert_eq!(out[0].tier_discount, Money::zero());
}

#[test]
fn chk_13_overridden_line_does_not_count_toward_the_promotion_quantity() {
    let soap = product("50.00");
    let mut ctx = retail_context();
    ctx.promotions.push(promotion(
        "Buy 2 get 1",
        PromoRule::BuyXGetY { buy: 2, get: 1 },
        PromoTarget::Product(soap.id.clone()),
        1,
    ));
    let mut overridden = line(1, &soap, "1");
    overridden.price_override = Some(money("45.00"));

    let out = price_cart(&ctx, &[line(0, &soap, "2"), overridden], None).unwrap();

    // Only 2 soaps at the normal price: not enough for a free one.
    assert_eq!(out[0].promo, None);
    assert_eq!(out[1].promo, None);
}

#[test]
fn chk_08_scale_sticker_price_is_used_exactly_and_gets_no_promotion_or_tier() {
    let beef = product("750.00");
    let mut ctx = retail_context();
    ctx.tier = Some(TierRule {
        id: Id::new(),
        discount: Bps(200),
    });
    ctx.promotions.push(promotion(
        "10% off beef",
        percent_off(1000),
        PromoTarget::Product(beef.id.clone()),
        1,
    ));
    let mut sticker = line(0, &beef, "1.234");
    sticker.embedded_price = Some(money("925.49"));

    let out = price_cart(&ctx, &[sticker], None).unwrap();

    assert_eq!(out[0].base, money("925.49"));
    assert_eq!(out[0].promo, None);
    assert_eq!(out[0].tier_discount, Money::zero());
    assert_eq!(out[0].amount, money("925.49"));
}

#[test]
fn chk_12_invoice_discount_is_split_by_each_line_amount() {
    // Money Tax and Rounding E8: lines 30.00 and 70.00, discount 10.00 → 3.00 and 7.00.
    let first = product("30.00");
    let second = product("70.00");
    let lines = [line(0, &first, "1"), line(1, &second, "1")];

    let out = price_cart(&retail_context(), &lines, manual_amount("10.00")).unwrap();

    assert_eq!(out[0].invoice_share, money("3.00"));
    assert_eq!(out[1].invoice_share, money("7.00"));
    assert_eq!(out[0].amount, money("27.00"));
    assert_eq!(out[1].amount, money("63.00"));
}

#[test]
fn chk_12_invoice_discount_left_over_goes_to_the_earliest_line() {
    // Money Tax and Rounding E9: three lines of 10.00, discount 10.00 → 3.34, 3.33, 3.33.
    let item = product("10.00");
    let other = product("10.00");
    let third = product("10.00");
    let lines = [
        line(0, &item, "1"),
        line(1, &other, "1"),
        line(2, &third, "1"),
    ];

    let out = price_cart(&retail_context(), &lines, manual_amount("10.00")).unwrap();

    let shares: Vec<Money> = out.iter().map(|line| line.invoice_share).collect();
    assert_eq!(shares, vec![money("3.34"), money("3.33"), money("3.33")]);
}

#[test]
fn chk_12_invoice_percent_is_taken_from_the_amount_after_line_discounts() {
    let soap = product("50.00");
    let mut soaps = line(0, &soap, "2");
    soaps.manual = manual_amount("20.00");

    let out = price_cart(&retail_context(), &[soaps], manual_percent(1000)).unwrap();

    // 100.00 − 20.00 = 80.00, then 10% = 8.00.
    assert_eq!(out[0].invoice_share, money("8.00"));
    assert_eq!(out[0].amount, money("72.00"));
}

#[test]
fn chk_12_invoice_discount_can_not_go_below_zero() {
    let soap = product("50.00");

    let out = price_cart(
        &retail_context(),
        &[line(0, &soap, "1")],
        manual_amount("75.00"),
    )
    .unwrap();

    assert_eq!(out[0].invoice_share, money("50.00"));
    assert_eq!(out[0].amount, Money::zero());
}

#[test]
fn prc_07_manual_discount_percent_is_the_largest_line_or_invoice_discount() {
    let soap = product("50.00");
    let rice = product("100.00");
    let mut soaps = line(0, &soap, "2");
    soaps.manual = manual_percent(1000);
    let lines = [soaps, line(1, &rice, "1")];

    // A 10% line discount counts as 10%, even though it's a smaller part of the cart.
    let out = price_cart(&retail_context(), &lines, None).unwrap();
    assert_eq!(manual_discount_bps(&out), Bps(1000));

    // An invoice discount of 38.00 on 190.00 is 20%, which is larger.
    let out = price_cart(&retail_context(), &lines, manual_amount("38.00")).unwrap();
    assert_eq!(manual_discount_bps(&out), Bps(2000));
}

#[test]
fn prc_07_manual_discount_percent_rounds_up_so_the_limit_is_never_passed_by_a_fraction() {
    let rice = product("300.00");
    let mut rice_line = line(0, &rice, "1");
    // 15.01 of 300.00 is 5.0033%.
    rice_line.manual = manual_amount("15.01");

    let out = price_cart(&retail_context(), &[rice_line], None).unwrap();

    assert_eq!(manual_discount_bps(&out), Bps(501));
}

#[test]
fn prc_07_no_manual_discount_is_zero_percent() {
    let out = price_cart(&retail_context(), &[line(0, &product("5.00"), "1")], None).unwrap();
    assert_eq!(manual_discount_bps(&out), Bps(0));
    assert_eq!(manual_discount_bps(&[]), Bps(0));
}

#[test]
fn engine_refuses_negative_quantities_and_prices() {
    let soap = product("50.00");
    let mut negative_qty = line(0, &soap, "1");
    negative_qty.qty = qty("-1");
    assert!(price_cart(&retail_context(), &[negative_qty], None).is_err());

    let mut negative_override = line(0, &soap, "1");
    negative_override.price_override = Some(money("-1.00"));
    assert!(price_cart(&retail_context(), &[negative_override], None).is_err());

    let negative_invoice = Some(ManualDiscount::Amount(Money(-100)));
    assert!(price_cart(&retail_context(), &[line(0, &soap, "1")], negative_invoice).is_err());
}

#[test]
fn engine_returns_lines_in_the_order_given() {
    let soap = product("50.00");
    let rice = product("100.00");

    let out = price_cart(
        &retail_context(),
        &[line(5, &rice, "1"), line(3, &soap, "1")],
        None,
    )
    .unwrap();

    let keys: Vec<usize> = out.iter().map(|line| line.key).collect();
    assert_eq!(keys, vec![5, 3]);
}
