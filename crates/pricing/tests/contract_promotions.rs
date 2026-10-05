//! Contract tests for promotions: saving, checking and loading them (PRC-04, CHK-16).
//! Each test uses an in-memory database built by this module's own migrations.

mod common;

use common::*;
use pos_core::{AppError, Bps, Id, Money, Timestamp};
use pricing::{
    PricingError, PricingProduct, PromoRule, PromoTarget, PromotionInput, TierInput,
    create_promotion, create_tier, load_context, price_cart, promotion, promotions,
    set_promotion_active, update_promotion,
};

const CREATED: Timestamp = Timestamp(1_780_000_000_000);

fn input(name: &str, rule: PromoRule, targets: Vec<PromoTarget>) -> PromotionInput {
    PromotionInput {
        name: name.to_string(),
        rule,
        starts_at: None,
        ends_at: None,
        days_mask: 0b111_1111,
        tier_id: None,
        targets,
    }
}

fn tier_input(name: &str, price_list_id: Option<Id>, discount: u32) -> TierInput {
    TierInput {
        name: name.to_string(),
        price_list_id,
        discount: Bps(discount),
        sort: 0,
    }
}

/// The validation code of an error, or a note that it wasn't a validation error.
fn code_of(error: PricingError) -> String {
    match AppError::from(error) {
        AppError::Validation { code, .. } => code,
        AppError::Conflict { code, .. } => code,
        other => format!("not a validation or conflict error: {other:?}"),
    }
}

#[test]
fn prc_04_every_promotion_kind_is_saved_and_read_back() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let gold = create_tier(&tx, &tier_input("Gold", None, 0)).unwrap();
    let rules = vec![
        PromoRule::PercentOff { percent: Bps(1000) },
        PromoRule::AmountOff {
            amount_per_unit: money("5.00"),
        },
        PromoRule::FixedPrice {
            price_per_unit: money("180.00"),
        },
        PromoRule::BuyXGetY { buy: 2, get: 1 },
        PromoRule::MultiBuy {
            bundle_qty: 3,
            bundle_price: money("100.00"),
        },
    ];
    let mut ids = Vec::new();
    for rule in &rules {
        let mut promo = input(
            "Deal",
            rule.clone(),
            vec![
                PromoTarget::Product(Id::new()),
                PromoTarget::Category(Id::new()),
                PromoTarget::Brand(Id::new()),
            ],
        );
        promo.starts_at = Some(Timestamp(10));
        promo.ends_at = Some(Timestamp(20));
        promo.days_mask = 0b001_0101;
        promo.tier_id = Some(gold.clone());
        ids.push((create_promotion(&tx, &promo, CREATED).unwrap(), promo));
    }
    tx.commit().unwrap();

    for (id, saved_input) in &ids {
        let saved = promotion(&conn, id).unwrap();
        assert_eq!(&saved.id, id);
        assert_eq!(saved.name, saved_input.name);
        assert_eq!(saved.rule, saved_input.rule);
        assert_eq!(saved.starts_at, saved_input.starts_at);
        assert_eq!(saved.ends_at, saved_input.ends_at);
        assert_eq!(saved.days_mask, saved_input.days_mask);
        assert_eq!(saved.tier_id, saved_input.tier_id);
        let mut expected_targets = saved_input.targets.clone();
        expected_targets.sort();
        assert_eq!(saved.targets, expected_targets);
        assert!(saved.active);
        assert_eq!(saved.created_at, CREATED);
    }
    assert_eq!(promotions(&conn).unwrap().len(), rules.len());
}

#[test]
fn prc_04_update_promotion_replaces_its_settings_and_targets() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let first = PromoTarget::Product(Id::new());
    let second = PromoTarget::Brand(Id::new());
    let id =
        create_promotion(&tx, &input("Deal", percent_off(1000), vec![first]), CREATED).unwrap();
    let changed = input("Better deal", percent_off(1500), vec![second.clone()]);
    update_promotion(&tx, &id, &changed).unwrap();
    tx.commit().unwrap();

    let saved = promotion(&conn, &id).unwrap();
    assert_eq!(saved.name, "Better deal");
    assert_eq!(saved.rule, percent_off(1500));
    assert_eq!(saved.targets, vec![second]);
}

#[test]
fn prc_04_promotion_settings_are_checked() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let target = vec![PromoTarget::Product(Id::new())];
    let cases: Vec<(PromotionInput, &str)> = vec![
        (
            input(" ", percent_off(1000), target.clone()),
            "promotion_name_missing",
        ),
        (
            input("Deal", percent_off(0), target.clone()),
            "promotion_percent_invalid",
        ),
        (
            input("Deal", percent_off(10_001), target.clone()),
            "promotion_percent_invalid",
        ),
        (
            input(
                "Deal",
                PromoRule::AmountOff {
                    amount_per_unit: Money(0),
                },
                target.clone(),
            ),
            "promotion_amount_invalid",
        ),
        (
            input(
                "Deal",
                PromoRule::FixedPrice {
                    price_per_unit: Money(-1),
                },
                target.clone(),
            ),
            "promotion_price_invalid",
        ),
        (
            input(
                "Deal",
                PromoRule::BuyXGetY { buy: 0, get: 1 },
                target.clone(),
            ),
            "promotion_buy_get_invalid",
        ),
        (
            input(
                "Deal",
                PromoRule::BuyXGetY { buy: 2, get: 0 },
                target.clone(),
            ),
            "promotion_buy_get_invalid",
        ),
        (
            input(
                "Deal",
                PromoRule::MultiBuy {
                    bundle_qty: 1,
                    bundle_price: money("10.00"),
                },
                target.clone(),
            ),
            "promotion_bundle_invalid",
        ),
        (
            input("Deal", percent_off(1000), Vec::new()),
            "promotion_targets_missing",
        ),
    ];
    for (promo, code) in cases {
        let error = create_promotion(&tx, &promo, CREATED).unwrap_err();
        assert_eq!(code_of(error), code);
    }

    let mut no_days = input("Deal", percent_off(1000), target.clone());
    no_days.days_mask = 0;
    assert_eq!(
        code_of(create_promotion(&tx, &no_days, CREATED).unwrap_err()),
        "promotion_days_invalid"
    );
    let mut too_many_days = input("Deal", percent_off(1000), target.clone());
    too_many_days.days_mask = 0b1000_0000;
    assert_eq!(
        code_of(create_promotion(&tx, &too_many_days, CREATED).unwrap_err()),
        "promotion_days_invalid"
    );
    let mut backwards = input("Deal", percent_off(1000), target.clone());
    backwards.starts_at = Some(Timestamp(20));
    backwards.ends_at = Some(Timestamp(20));
    assert_eq!(
        code_of(create_promotion(&tx, &backwards, CREATED).unwrap_err()),
        "promotion_dates_invalid"
    );
    let mut unknown_tier = input("Deal", percent_off(1000), target);
    unknown_tier.tier_id = Some(Id::new());
    let error = create_promotion(&tx, &unknown_tier, CREATED).unwrap_err();
    assert!(matches!(AppError::from(error), AppError::NotFound { .. }));
}

#[test]
fn prc_04_unknown_promotion_is_not_found() {
    let mut conn = test_db();
    assert!(matches!(
        AppError::from(promotion(&conn, &Id::new()).unwrap_err()),
        AppError::NotFound { .. }
    ));
    let tx = conn.transaction().unwrap();
    let missing = Id::new();
    let deal = input(
        "Deal",
        percent_off(1000),
        vec![PromoTarget::Product(Id::new())],
    );
    assert!(update_promotion(&tx, &missing, &deal).is_err());
    assert!(set_promotion_active(&tx, &missing, false).is_err());
}

#[test]
fn chk_16_load_context_brings_only_switched_on_promotions_for_these_products() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let category = Id::new();
    let brand = Id::new();
    let soap = PricingProduct {
        id: Id::new(),
        retail_price: money("50.00"),
        category_path: vec![category.clone()],
        brand_id: Some(brand.clone()),
    };
    let by_product = create_promotion(
        &tx,
        &input(
            "Buy 2 get 1",
            PromoRule::BuyXGetY { buy: 2, get: 1 },
            vec![PromoTarget::Product(soap.id.clone())],
        ),
        CREATED,
    )
    .unwrap();
    let by_category = create_promotion(
        &tx,
        &input(
            "Category",
            percent_off(500),
            vec![PromoTarget::Category(category)],
        ),
        Timestamp(CREATED.0 + 1),
    )
    .unwrap();
    let by_brand = create_promotion(
        &tx,
        &input("Brand", percent_off(500), vec![PromoTarget::Brand(brand)]),
        Timestamp(CREATED.0 + 2),
    )
    .unwrap();
    let switched_off = create_promotion(
        &tx,
        &input(
            "Off",
            percent_off(500),
            vec![PromoTarget::Product(soap.id.clone())],
        ),
        CREATED,
    )
    .unwrap();
    set_promotion_active(&tx, &switched_off, false).unwrap();
    create_promotion(
        &tx,
        &input(
            "Other product",
            percent_off(500),
            vec![PromoTarget::Product(Id::new())],
        ),
        CREATED,
    )
    .unwrap();
    tx.commit().unwrap();

    let ctx = load_context(&conn, std::slice::from_ref(&soap), None, None, monday()).unwrap();
    let mut loaded: Vec<Id> = ctx
        .promotions
        .iter()
        .map(|promo| promo.id.clone())
        .collect();
    loaded.sort();
    let mut expected = vec![by_product, by_category, by_brand];
    expected.sort();
    assert_eq!(loaded, expected);

    // CHK-16: 3 soaps show a −50.00 "Buy 2 get 1" discount.
    let out = price_cart(&ctx, &[line(0, &soap, "3")], None).unwrap();
    let applied = out[0].promo.as_ref().unwrap();
    assert_eq!(applied.name, "Buy 2 get 1");
    assert_eq!(applied.discount, money("50.00"));
}
