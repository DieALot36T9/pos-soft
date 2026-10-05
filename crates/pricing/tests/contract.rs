//! Contract tests: the public API and the published view behave as the module page says.
//! Each test uses an in-memory database built by this module's own migrations.

mod common;

use common::*;
use pos_core::{AppError, Bps, Id, Money, Timestamp};
use pricing::{
    PriceListItem, PriceListUpdate, PricingError, PromoRule, PromoTarget, PromotionInput,
    TierInput, create_price_list, create_promotion, create_tier, load_context, price_cart,
    price_list_items, price_lists, remove_price_list_items, set_price_list_items,
    set_promotion_active, tiers, update_price_list, update_tier,
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
fn migrations_belong_to_this_module() {
    assert_eq!(pricing::migrations().module, "pricing");
    assert_eq!(pricing::migrations().migrations[0].version, 1);
}

#[test]
fn migrations_build_every_table_and_the_view_on_an_empty_database() {
    let conn = test_db();
    let mut names: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE name LIKE 'pricing_%' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|name| name.unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "pricing_price_list_items",
            "pricing_price_lists",
            "pricing_promotion_targets",
            "pricing_promotions",
            "pricing_targets_lookup",
            "pricing_tiers",
            "pricing_v_promotions",
        ]
    );
}

#[test]
fn view_pricing_v_promotions_lists_every_promotion() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let soap = Id::new();
    let mut deal = input(
        "Buy 2 get 1",
        PromoRule::BuyXGetY { buy: 2, get: 1 },
        vec![PromoTarget::Product(soap)],
    );
    deal.starts_at = Some(Timestamp(100));
    deal.ends_at = Some(Timestamp(200));
    let id = create_promotion(&tx, &deal, CREATED).unwrap();
    set_promotion_active(&tx, &id, false).unwrap();
    tx.commit().unwrap();

    let row: (String, String, String, i64, i64, i64) = conn
        .query_row(
            "SELECT id, name, kind, starts_at, ends_at, active FROM pricing_v_promotions",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        row,
        (
            id.as_str().to_string(),
            "Buy 2 get 1".to_string(),
            "buy_x_get_y".to_string(),
            100,
            200,
            0
        )
    );
}

#[test]
fn prc_02_price_list_prices_are_saved_changed_and_removed() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let rice = Id::new();
    let salt = Id::new();
    let list = create_price_list(&tx, "Wholesale", CREATED).unwrap();
    set_price_list_items(
        &tx,
        &list,
        &[
            PriceListItem {
                product_id: rice.clone(),
                price: money("95.00"),
            },
            PriceListItem {
                product_id: salt.clone(),
                price: money("28.00"),
            },
        ],
    )
    .unwrap();
    // Setting a price again replaces it.
    set_price_list_items(
        &tx,
        &list,
        &[PriceListItem {
            product_id: rice.clone(),
            price: money("94.00"),
        }],
    )
    .unwrap();
    remove_price_list_items(&tx, &list, std::slice::from_ref(&salt)).unwrap();
    tx.commit().unwrap();

    let items = price_list_items(&conn, &list).unwrap();
    assert_eq!(
        items,
        vec![PriceListItem {
            product_id: rice,
            price: money("94.00"),
        }]
    );
    let lists = price_lists(&conn).unwrap();
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].name, "Wholesale");
    assert!(lists[0].active);
    assert_eq!(lists[0].created_at, CREATED);
}

#[test]
fn prc_02_price_list_needs_a_unique_name_and_prices_of_zero_or_more() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let list = create_price_list(&tx, "Wholesale", CREATED).unwrap();

    let empty = create_price_list(&tx, "  ", CREATED).unwrap_err();
    assert_eq!(code_of(empty), "price_list_name_missing");
    let taken = create_price_list(&tx, "Wholesale", CREATED).unwrap_err();
    assert_eq!(code_of(taken), "price_list_name_taken");
    let negative = set_price_list_items(
        &tx,
        &list,
        &[PriceListItem {
            product_id: Id::new(),
            price: Money(-1),
        }],
    )
    .unwrap_err();
    assert_eq!(code_of(negative), "price_negative");
    let missing = set_price_list_items(&tx, &Id::new(), &[]).unwrap_err();
    assert!(matches!(AppError::from(missing), AppError::NotFound { .. }));
}

#[test]
fn prc_02_load_context_uses_the_price_list_and_falls_back_to_retail() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let rice = product("100.00");
    let salt = product("30.00");
    let list = create_price_list(&tx, "Wholesale", CREATED).unwrap();
    set_price_list_items(
        &tx,
        &list,
        &[PriceListItem {
            product_id: rice.id.clone(),
            price: money("95.00"),
        }],
    )
    .unwrap();
    tx.commit().unwrap();

    let ctx = load_context(
        &conn,
        &[rice.clone(), salt.clone()],
        None,
        Some(&list),
        monday(),
    )
    .unwrap();
    let out = price_cart(&ctx, &[line(0, &rice, "1"), line(1, &salt, "1")], None).unwrap();

    assert_eq!(out[0].unit_price, money("95.00"));
    assert_eq!(out[1].unit_price, money("30.00"));
}

#[test]
fn prc_02_a_switched_off_price_list_is_not_used() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let rice = product("100.00");
    let list = create_price_list(&tx, "Wholesale", CREATED).unwrap();
    set_price_list_items(
        &tx,
        &list,
        &[PriceListItem {
            product_id: rice.id.clone(),
            price: money("95.00"),
        }],
    )
    .unwrap();
    update_price_list(
        &tx,
        &list,
        &PriceListUpdate {
            name: "Old wholesale".to_string(),
            active: false,
        },
    )
    .unwrap();
    tx.commit().unwrap();

    let ctx = load_context(
        &conn,
        std::slice::from_ref(&rice),
        None,
        Some(&list),
        monday(),
    )
    .unwrap();

    assert!(ctx.list_prices.is_empty());
    assert_eq!(price_lists(&conn).unwrap()[0].name, "Old wholesale");
}

#[test]
fn prc_03_gold_tier_brings_member_prices_and_its_discount() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let rice = product("100.00");
    let member = create_price_list(&tx, "Member", CREATED).unwrap();
    set_price_list_items(
        &tx,
        &member,
        &[PriceListItem {
            product_id: rice.id.clone(),
            price: money("95.00"),
        }],
    )
    .unwrap();
    let gold = create_tier(&tx, &tier_input("Gold", Some(member), 200)).unwrap();
    tx.commit().unwrap();

    let ctx = load_context(
        &conn,
        std::slice::from_ref(&rice),
        Some(&gold),
        None,
        monday(),
    )
    .unwrap();
    let out = price_cart(&ctx, &[line(0, &rice, "1")], None).unwrap();

    assert_eq!(out[0].unit_price, money("95.00"));
    assert_eq!(out[0].tier_discount, money("1.90"));
}

#[test]
fn chk_17_price_list_chosen_for_the_cart_wins_over_the_tier_price_list() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let rice = product("100.00");
    let member = create_price_list(&tx, "Member", CREATED).unwrap();
    let wholesale = create_price_list(&tx, "Wholesale", CREATED).unwrap();
    for (list, price) in [(&member, "95.00"), (&wholesale, "90.00")] {
        set_price_list_items(
            &tx,
            list,
            &[PriceListItem {
                product_id: rice.id.clone(),
                price: money(price),
            }],
        )
        .unwrap();
    }
    let gold = create_tier(&tx, &tier_input("Gold", Some(member), 200)).unwrap();
    tx.commit().unwrap();

    let ctx = load_context(
        &conn,
        std::slice::from_ref(&rice),
        Some(&gold),
        Some(&wholesale),
        monday(),
    )
    .unwrap();

    assert_eq!(ctx.list_prices.get(&rice.id), Some(&money("90.00")));
    // The tier's discount still applies.
    assert_eq!(ctx.tier.unwrap().discount, Bps(200));
}

#[test]
fn prc_03_tiers_are_saved_changed_archived_and_checked() {
    let mut conn = test_db();
    let tx = conn.transaction().unwrap();
    let gold = create_tier(&tx, &tier_input("Gold", None, 200)).unwrap();
    update_tier(&tx, &gold, &tier_input("Gold plus", None, 300)).unwrap();

    let taken = create_tier(&tx, &tier_input("Gold plus", None, 0)).unwrap_err();
    assert_eq!(code_of(taken), "tier_name_taken");
    let too_much = create_tier(&tx, &tier_input("Silver", None, 10_001)).unwrap_err();
    assert_eq!(code_of(too_much), "tier_discount_invalid");
    let no_list = create_tier(&tx, &tier_input("Silver", Some(Id::new()), 0)).unwrap_err();
    assert!(matches!(AppError::from(no_list), AppError::NotFound { .. }));

    pricing::archive_tier(&tx, &gold, CREATED).unwrap();
    tx.commit().unwrap();

    let saved = tiers(&conn).unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].name, "Gold plus");
    assert_eq!(saved[0].discount, Bps(300));
    assert_eq!(saved[0].archived_at, Some(CREATED));
    assert_eq!(pricing::tier(&conn, &gold).unwrap(), saved[0]);
    assert!(pricing::tier(&conn, &Id::new()).is_err());

    // An archived tier gives no discount, even if a customer still has it.
    let ctx = load_context(&conn, &[], Some(&gold), None, monday()).unwrap();
    assert_eq!(ctx.tier, None);
}

#[test]
fn load_context_refuses_an_unknown_tier_or_price_list() {
    let conn = test_db();
    let unknown = Id::new();
    assert!(load_context(&conn, &[], Some(&unknown), None, monday()).is_err());
    assert!(load_context(&conn, &[], None, Some(&unknown), monday()).is_err());
}

#[test]
fn load_context_keeps_the_local_time_it_was_given() {
    let conn = test_db();
    let ctx = load_context(&conn, &[], None, None, monday()).unwrap();
    assert_eq!(ctx.now, monday());
}
