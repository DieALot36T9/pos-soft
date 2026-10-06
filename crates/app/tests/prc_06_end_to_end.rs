//! The PRC-06 worked examples end to end (Req - Pricing and Promotions): each promotion
//! is saved through the pricing commands, the product is sold at the till, and the cart
//! the screen gets shows the expected discount. The pricing crate tests the same
//! examples on the engine alone (`crates/pricing/tests/prc_06_examples.rs`).

mod common;

use app::commands::checkout::{self as till, WeightInput};
use app::commands::pricing as pricing_commands;
use app::state::AppState;
use app::workflows::cart_view::CartView;
use app::workflows::pricing_admin::PromotionSave;
use common::*;
use pos_core::{Bps, Clock, Id, Money, Qty};
use pricing::{EVERY_DAY, PromoRule, PromoTarget, PromotionInput};

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// A shop with the admin's shift open and one product at `price_minor`.
fn shop_selling(
    name: &str,
    price_minor: i64,
    weighed: bool,
) -> (
    tempfile::TempDir,
    AppState,
    std::sync::Arc<pos_core::FixedClock>,
    Id,
) {
    let folder = tempfile::tempdir().expect("folder");
    let (state, clock) = start_with_clock(folder.path());
    finish_setup(&state);
    let unit = if weighed {
        catalog::UNIT_KG
    } else {
        catalog::UNIT_PIECE
    };
    let item = till_product(&state, name, "ITEM-1", price_minor, unit, weighed, None);
    stock_in(&state, &item, Qty(1_000_000));
    open_shift(&state, 0);
    (folder, state, clock, item)
}

fn promote(state: &AppState, name: &str, rule: PromoRule, item: &Id, ends_at: Option<i64>) {
    let input = PromotionSave {
        id: None,
        promotion: PromotionInput {
            name: name.into(),
            rule,
            starts_at: None,
            ends_at: ends_at.map(pos_core::Timestamp),
            days_mask: EVERY_DAY,
            tier_id: None,
            targets: vec![PromoTarget::Product(item.clone())],
        },
    };
    pricing_commands::promotion_save(state, &input).expect("promotion saved");
}

/// Scans the item and sets the line to `qty` pieces; gives the cart the screen shows.
fn sell(state: &AppState, pieces: i64) -> CartView {
    till::checkout_scan(state, "ITEM-1").expect("scan");
    let line = till::checkout_state(state)
        .expect("state")
        .cart
        .expect("cart")
        .lines[0]
        .line_id
        .clone();
    till::checkout_set_qty(state, &line, Qty(pieces * 1000)).expect("qty")
}

#[test]
fn prc_06_buy_2_get_1() {
    let (_folder, state, _clock, soap) = shop_selling("Soap", 5_000, false);
    promote(
        &state,
        "Buy 2 get 1",
        PromoRule::BuyXGetY { buy: 2, get: 1 },
        &soap,
        None,
    );
    let cart = sell(&state, 7);
    assert_eq!(cart.lines[0].discount, Money(10_000));
    assert_eq!(cart.lines[0].amount, Money(25_000));
    assert_eq!(cart.lines[0].promotion_name.as_deref(), Some("Buy 2 get 1"));
}

#[test]
fn prc_06_multi_buy() {
    let (_folder, state, _clock, noodles) = shop_selling("Noodles", 4_000, false);
    let rule = PromoRule::MultiBuy {
        bundle_qty: 3,
        bundle_price: Money(10_000),
    };
    promote(&state, "3 for 100", rule, &noodles, None);
    let cart = sell(&state, 7);
    assert_eq!(cart.lines[0].discount, Money(4_000));
    assert_eq!(cart.lines[0].amount, Money(24_000));
}

#[test]
fn prc_06_percent_off() {
    let (_folder, state, _clock, detergent) = shop_selling("Detergent", 25_000, false);
    let rule = PromoRule::PercentOff {
        percent: Bps(1_000),
    };
    promote(&state, "10% off", rule, &detergent, None);
    let cart = sell(&state, 2);
    assert_eq!(cart.lines[0].discount, Money(5_000));
}

#[test]
fn prc_06_a_fixed_price_above_retail_is_ignored() {
    let (_folder, state, _clock, oil) = shop_selling("Oil", 17_000, false);
    let rule = PromoRule::FixedPrice {
        price_per_unit: Money(18_000),
    };
    promote(&state, "Oil 180", rule, &oil, None);
    let cart = sell(&state, 1);
    assert_eq!(cart.lines[0].discount, Money::zero());
    assert_eq!(cart.lines[0].amount, Money(17_000));
    assert_eq!(cart.lines[0].promotion_name, None);
}

#[test]
fn prc_06_the_best_of_two_promotions_wins() {
    let (_folder, state, _clock, soap) = shop_selling("Soap", 5_000, false);
    let ten = PromoRule::PercentOff {
        percent: Bps(1_000),
    };
    promote(&state, "10% off", ten, &soap, None);
    promote(
        &state,
        "Buy 2 get 1",
        PromoRule::BuyXGetY { buy: 2, get: 1 },
        &soap,
        None,
    );
    let cart = sell(&state, 3);
    assert_eq!(cart.lines[0].discount, Money(5_000));
    assert_eq!(cart.lines[0].promotion_name.as_deref(), Some("Buy 2 get 1"));
}

#[test]
fn prc_06_a_weighed_item_with_percent_off() {
    let (_folder, state, _clock, beef) = shop_selling("Beef", 75_000, true);
    let rule = PromoRule::PercentOff {
        percent: Bps(1_000),
    };
    promote(&state, "10% off", rule, &beef, None);
    let typed = WeightInput::Manual {
        text: "1.234".into(),
    };
    let cart = till::checkout_add_weighed(&state, devices(), &beef, &typed).expect("weighed");
    assert_eq!(cart.lines[0].base, Money(92_550));
    assert_eq!(cart.lines[0].discount, Money(9_255));
}

#[test]
fn prc_06_an_expired_promotion_is_not_applied() {
    let (_folder, state, clock, soap) = shop_selling("Soap", 5_000, false);
    let yesterday = clock.now().0 - DAY_MS;
    let rule = PromoRule::PercentOff {
        percent: Bps(1_000),
    };
    promote(&state, "Last week", rule, &soap, Some(yesterday));
    let cart = sell(&state, 2);
    assert_eq!(cart.lines[0].discount, Money::zero());
}
