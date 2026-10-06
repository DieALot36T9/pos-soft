//! The checkout workflows (Req - Checkout): scanning, search, quantities, weights,
//! scale stickers, hold and resume, cancel, kits, totals, crash recovery and stock
//! warnings. Every amount checked here comes from the cart Rust sends back.

mod common;

use app::commands::checkout as till;
use app::commands::checkout::WeightInput;
use app::commands::settings as settings_commands;
use app::devices::Devices;
use app::session;
use app::state::AppState;
use app::workflows::cart_view::{CartView, StockWarning};
use app::workflows::checkout::ScanResult;
use app::workflows::settings_change::SettingsChange;
use common::*;
use hardware::WeightUnit;
use pos_core::{AppError, Id, Money, Qty};
use settings::{CheckoutSettings, InventorySettings, Setting};

/// "Milk 1L" from CHK-01, a valid EAN-13.
const MILK_CODE: &str = "8901234567890";
const POTATO_CODE: &str = "2000000000015";

fn shop() -> (tempfile::TempDir, AppState) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    (folder, state)
}

/// A shop with Milk 1L at 90.00 and a shift open for the admin.
fn selling_shop() -> (tempfile::TempDir, AppState, Id) {
    let (folder, state) = shop();
    let milk = till_product(
        &state,
        "Milk 1L",
        MILK_CODE,
        9_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    stock_in(&state, &milk, Qty(100_000));
    open_shift(&state, 0);
    (folder, state, milk)
}

fn added(result: ScanResult) -> (CartView, Id) {
    match result {
        ScanResult::Added { cart, line_id } => (cart, line_id),
        other => panic!("expected an added line, got {other:?}"),
    }
}

fn cart(state: &AppState) -> CartView {
    till::checkout_state(state)
        .expect("state")
        .cart
        .expect("a cart")
}

fn conflict_code(result: Result<impl std::fmt::Debug, AppError>) -> String {
    match result {
        Err(AppError::Conflict { code, .. }) | Err(AppError::Validation { code, .. }) => code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn chk_20_selling_needs_an_open_shift() {
    let (_folder, state) = shop();
    till_product(
        &state,
        "Milk 1L",
        MILK_CODE,
        9_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    let view = till::checkout_state(&state).expect("state");
    assert!(view.shift.is_none());
    assert!(view.cart.is_none());
    assert_eq!(
        conflict_code(till::checkout_scan(&state, MILK_CODE)),
        "shift_not_open"
    );
    open_shift(&state, 0);
    let view = till::checkout_state(&state).expect("state");
    assert!(view.shift.expect("shift").mine);
    assert!(view.cart.expect("cart").lines.is_empty());
}

#[test]
fn chk_01_a_scan_adds_the_item_and_a_repeat_scan_merges() {
    let (_folder, state, milk) = selling_shop();
    let (view, first_line) = added(till::checkout_scan(&state, MILK_CODE).expect("scan"));
    assert_eq!(view.lines.len(), 1);
    let line = &view.lines[0];
    assert_eq!(
        (line.product_id.clone(), line.name.as_str()),
        (milk, "Milk 1L")
    );
    assert_eq!((line.qty, line.amount), (Qty(1000), Money(9_000)));
    assert_eq!(view.totals.total, Money(9_000));

    let (view, second_line) = added(till::checkout_scan(&state, MILK_CODE).expect("scan"));
    assert_eq!(second_line, first_line);
    assert_eq!(view.lines.len(), 1);
    assert_eq!(
        (view.lines[0].qty, view.totals.total),
        (Qty(2000), Money(18_000))
    );

    let mut checkout = CheckoutSettings::default_value();
    checkout.merge_scans = false;
    settings_commands::settings_save(&state, &SettingsChange::Checkout(checkout)).expect("saved");
    let (view, third_line) = added(till::checkout_scan(&state, MILK_CODE).expect("scan"));
    assert_ne!(third_line, first_line);
    assert_eq!(view.lines.len(), 2);
}

#[test]
fn chk_02_an_unknown_code_changes_nothing() {
    let (_folder, state, _milk) = selling_shop();
    till::checkout_scan(&state, MILK_CODE).expect("scan");
    let result = till::checkout_scan(&state, "123");
    assert!(
        matches!(&result, Err(AppError::NotFound { entity, key }) if entity == "product_code" && key == "123"),
        "{result:?}"
    );
    assert_eq!(cart(&state).lines.len(), 1);
}

#[test]
fn chk_03_text_is_searched_and_archived_products_never_show() {
    let (_folder, state, milk) = selling_shop();
    let butter = till_product(
        &state,
        "Milk Butter",
        "",
        20_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    match till::checkout_scan(&state, "milk").expect("scan") {
        ScanResult::Search { hits } => assert_eq!(hits.len(), 2),
        other => panic!("{other:?}"),
    }
    let hits = till::checkout_search(&state, "mi").expect("search");
    assert!(
        hits.iter()
            .any(|hit| hit.product_id == milk && hit.on_hand == Qty(100_000))
    );
    assert!(
        till::checkout_search(&state, "m")
            .expect("one letter")
            .is_empty()
    );
    app::commands::catalog::product_archive(&state, &butter).expect("archived");
    let hits = till::checkout_search(&state, "milk").expect("search");
    assert!(hits.iter().all(|hit| hit.product_id != butter));
    let (view, _) = added(till::checkout_add_product(&state, &milk, None).expect("picked"));
    assert_eq!(view.lines[0].qty, Qty(1000));
}

#[test]
fn chk_04_quantity_star_decimals_and_zero() {
    let (_folder, state, _milk) = selling_shop();
    let (view, line) = added(till::checkout_scan(&state, &format!("3*{MILK_CODE}")).expect("scan"));
    assert_eq!(
        (view.lines[0].qty, view.totals.total),
        (Qty(3000), Money(27_000))
    );
    assert_eq!(
        conflict_code(till::checkout_set_qty(&state, &line, Qty(1500))),
        "qty_decimals_too_many"
    );
    let view = till::checkout_set_qty(&state, &line, Qty(5000)).expect("five");
    assert_eq!(view.totals.total, Money(45_000));
    let view = till::checkout_set_qty(&state, &line, Qty(0)).expect("zero");
    assert!(view.lines.is_empty());
}

#[test]
fn chk_05_remove_a_line_and_cancel_the_sale() {
    let (_folder, state, _milk) = selling_shop();
    let bread = till_product(
        &state,
        "Bread",
        "BREAD-1",
        5_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    let (_, milk_line) = added(till::checkout_scan(&state, MILK_CODE).expect("scan"));
    till::checkout_add_product(&state, &bread, None).expect("bread");
    let view = till::checkout_remove_line(&state, &milk_line).expect("removed");
    assert_eq!(view.lines.len(), 1);
    assert_eq!(view.lines[0].product_id, bread);

    let before = audit_count(&state, "cart.cancel");
    let view = till::checkout_cancel(&state).expect("cancelled");
    assert!(view.lines.is_empty());
    assert_eq!(audit_count(&state, "cart.cancel"), before + 1);
    let (_, _, details) = last_audit(&state, "cart.cancel");
    assert!(details.contains("Bread"), "{details}");
    let summary = state
        .db
        .read(|conn| Ok(sales::sales_summary(conn)?))
        .expect("summary");
    assert_eq!(summary.sale_count, 0);
}

#[test]
fn chk_06_a_weighed_item_asks_for_its_weight() {
    let (_folder, state, _milk) = selling_shop();
    let potato = till_product(
        &state,
        "Potato",
        POTATO_CODE,
        6_000,
        catalog::UNIT_KG,
        true,
        None,
    );
    match till::checkout_scan(&state, POTATO_CODE).expect("scan") {
        ScanResult::NeedsWeight { product } => {
            assert_eq!(product.product_id, potato);
            assert_eq!(
                (product.unit_symbol.as_str(), product.unit_price),
                ("kg", Money(6_000))
            );
        }
        other => panic!("{other:?}"),
    }
    let devices = Devices::new(None);
    let typed = WeightInput::Manual {
        text: "1250 g".into(),
    };
    let view = till::checkout_add_weighed(&state, &devices, &potato, &typed).expect("weighed");
    let line = &view.lines[0];
    assert!(line.is_weighed);
    assert_eq!((line.qty, line.amount), (Qty(1250), Money(7_500)));
    let typed = WeightInput::Manual {
        text: "1.25".into(),
    };
    let view = till::checkout_add_weighed(&state, &devices, &potato, &typed).expect("again");
    assert_eq!(view.lines.len(), 2, "each weighing is its own line");
}

#[test]
fn chk_07_the_scale_weight_counts_only_when_stable_and_above_zero() {
    let (_folder, state, _milk) = selling_shop();
    let potato = till_product(
        &state,
        "Potato",
        POTATO_CODE,
        6_000,
        catalog::UNIT_KG,
        true,
        None,
    );
    let devices = Devices::new(None);
    devices.fake_scale.set_connected(true);
    devices.fake_scale.set_weight(1250, WeightUnit::Kg);
    devices.fake_scale.set_stable(false);
    let live = till::checkout_read_scale(&state, &devices, &potato).expect("live");
    assert!(live.has_scale && !live.stable);
    assert_eq!(live.qty, Some(Qty(1250)));
    assert_eq!(
        conflict_code(till::checkout_add_weighed(
            &state,
            &devices,
            &potato,
            &WeightInput::Scale
        )),
        "weight_not_stable"
    );
    devices.fake_scale.set_stable(true);
    let view = till::checkout_add_weighed(&state, &devices, &potato, &WeightInput::Scale)
        .expect("stable weight");
    assert_eq!(view.lines[0].amount, Money(7_500));
    devices.fake_scale.set_weight(0, WeightUnit::Kg);
    assert_eq!(
        conflict_code(till::checkout_add_weighed(
            &state,
            &devices,
            &potato,
            &WeightInput::Scale
        )),
        "weight_not_positive"
    );
    devices.fake_scale.set_connected(false);
    let live = till::checkout_read_scale(&state, &devices, &potato).expect("live");
    assert!(live.problem.is_some() && live.qty.is_none());
}

#[test]
fn chk_08_scale_stickers_carry_the_weight_or_the_price() {
    let (_folder, state, _milk) = selling_shop();
    let beef = till_product(
        &state,
        "Beef",
        "",
        75_000,
        catalog::UNIT_KG,
        true,
        Some("00123"),
    );
    let cheese = till_product(
        &state,
        "Cheese",
        "",
        120_000,
        catalog::UNIT_KG,
        true,
        Some("00456"),
    );
    let (view, _) = added(till::checkout_scan(&state, "2100123012343").expect("weight label"));
    let line = &view.lines[0];
    assert_eq!(line.product_id, beef);
    assert_eq!((line.qty, line.amount), (Qty(1234), Money(92_550)));

    let first12 = "230045615000";
    let check = barcode::ean13_check_digit(first12).expect("check digit");
    let (view, _) =
        added(till::checkout_scan(&state, &format!("{first12}{check}")).expect("price"));
    let line = &view.lines[1];
    assert_eq!(line.product_id, cheese);
    assert!(line.sticker_price);
    assert_eq!((line.amount, line.qty), (Money(15_000), Qty(125)));

    let wrong = format!("{first12}{}", (check + 1) % 10);
    assert!(matches!(
        till::checkout_scan(&state, &wrong),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn chk_09_chk_10_held_carts_survive_a_restart_and_resume_repriced() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let milk = till_product(
        &state,
        "Milk 1L",
        MILK_CODE,
        9_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    open_shift(&state, 0);
    till::checkout_scan(&state, &format!("2*{MILK_CODE}")).expect("scan");
    let view = till::checkout_hold(&state, Some("blue shirt, forgot rice")).expect("held");
    assert!(view.lines.is_empty());
    assert_eq!(till::checkout_state(&state).expect("state").held_count, 1);
    drop(state);

    let (state, clock) = start_with_clock(folder.path());
    clock.advance_ms(60_000);
    sign_in(&state, "owner", ADMIN_PASSWORD);
    let held = till::checkout_held_list(&state).expect("list");
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].note.as_deref(), Some("blue shirt, forgot rice"));
    assert_eq!((held[0].item_count, held[0].total), (2, Money(18_000)));
    assert_eq!(held[0].cashier_name, "Shop Owner");

    let mut product = app::commands::catalog::product_get(&state, &milk)
        .expect("get")
        .product;
    product.sale_price = Money(9_500);
    resave(&state, product);
    let resumed = till::checkout_resume(&state, &held[0].cart_id).expect("resumed");
    assert_eq!(resumed.cart.totals.total, Money(19_000));
    assert_eq!(
        resumed.price_changed,
        vec![resumed.cart.lines[0].line_id.clone()]
    );
    assert!(till::checkout_held_list(&state).expect("list").is_empty());
}

#[test]
fn chk_10_resume_is_refused_while_the_current_cart_has_items() {
    let (_folder, state, _milk) = selling_shop();
    till::checkout_scan(&state, MILK_CODE).expect("scan");
    till::checkout_hold(&state, None).expect("held");
    till::checkout_scan(&state, MILK_CODE).expect("scan");
    let held = till::checkout_held_list(&state).expect("list");
    assert_eq!(
        conflict_code(till::checkout_resume(&state, &held[0].cart_id)),
        "cart_in_progress"
    );
}

#[test]
fn chk_14_a_kit_is_one_line_and_takes_its_parts_out_of_stock() {
    let (_folder, state, _milk) = selling_shop();
    let sugar = till_product(
        &state,
        "Sugar 1kg",
        "SUGAR-1",
        12_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    let oil = till_product(
        &state,
        "Oil 1L",
        "OIL-1",
        18_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    stock_in(&state, &sugar, Qty(10_000));
    stock_in(&state, &oil, Qty(10_000));
    let mut kit_input = kit("Eid Box");
    kit_input.sale_price = Money(40_000);
    let kit_id = app::commands::catalog::product_save(&state, kit_input).expect("kit");
    let parts = [
        app::workflows::products::KitPartInput {
            product_id: sugar.clone(),
            qty: Qty(2000),
        },
        app::workflows::products::KitPartInput {
            product_id: oil.clone(),
            qty: Qty(1000),
        },
    ];
    app::commands::catalog::kit_save(&state, &kit_id, &parts).expect("parts");
    let (view, _) = added(till::checkout_add_product(&state, &kit_id, None).expect("kit added"));
    assert_eq!(view.lines.len(), 1);
    let done = till::checkout_complete(&state, devices(), &[cash(&state, 40_000)]).expect("sold");
    let sale = state
        .db
        .read(|conn| Ok(sales::get_sale(conn, &done.sale_id)?))
        .expect("sale");
    assert_eq!(sale.lines.len(), 1);
    assert_eq!(sale.lines[0].snapshot.unit_cost, Money(7_200 * 2 + 10_800));
    let on_hand = |id: &Id| {
        state
            .db
            .read(|conn| Ok(inventory::on_hand(conn, id)?))
            .expect("on hand")
    };
    assert_eq!(on_hand(&sugar), Qty(8_000));
    assert_eq!(on_hand(&oil), Qty(9_000));
}

#[test]
fn chk_18_totals_show_tax_inside_and_cash_rounding() {
    let (_folder, state) = shop();
    till_product(
        &state,
        "Rice",
        "RICE-5",
        9_040,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    open_shift(&state, 0);
    let (view, _) = added(till::checkout_scan(&state, "RICE-5").expect("scan"));
    let totals = &view.totals;
    assert_eq!(view.tax_label, "VAT");
    assert_eq!(totals.subtotal, Money(9_040));
    // 90.40 includes 15%: tax = 90.40 × 15 / 115 = 11.79.
    assert_eq!((totals.tax, totals.net), (Money(1_179), Money(7_861)));
    assert_eq!((totals.rounding, totals.total), (Money(-40), Money(9_000)));
    assert_eq!(totals.item_count, 1);
}

#[test]
fn chk_19_the_cart_is_back_after_a_crash() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    for number in 1..=5 {
        till_product(
            &state,
            &format!("Item {number}"),
            &format!("ITEM-{number}"),
            1_000 * number,
            catalog::UNIT_PIECE,
            false,
            None,
        );
    }
    open_shift(&state, 0);
    for number in 1..=5 {
        till::checkout_scan(&state, &format!("{number}*ITEM-{number}")).expect("scan");
    }
    let before = cart(&state);
    // The process dies: nothing is closed or logged out.
    drop(state);
    let state = start(folder.path());
    sign_in(&state, "owner", ADMIN_PASSWORD);
    assert_eq!(cart(&state), before);
}

#[test]
fn chk_21_stock_warns_or_blocks_by_the_setting() {
    let (_folder, state) = shop();
    let tea = till_product(
        &state,
        "Tea",
        "TEA-1",
        5_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    stock_in(&state, &tea, Qty(1000));
    open_shift(&state, 0);
    let (view, line) = added(till::checkout_scan(&state, "TEA-1").expect("one in stock"));
    assert_eq!(view.lines[0].stock_warning, None);
    let view = till::checkout_set_qty(&state, &line, Qty(2000)).expect("warns only");
    assert_eq!(
        view.lines[0].stock_warning,
        Some(StockWarning::NotEnoughStock)
    );

    let mut rules = InventorySettings::default_value();
    rules.allow_negative = false;
    settings_commands::settings_save(&state, &SettingsChange::Inventory(rules)).expect("saved");
    let result = till::checkout_scan(&state, "TEA-1");
    assert!(
        matches!(&result, Err(AppError::Conflict { code, params }) if code == "not_enough_stock" && params["available_milli"] == "1000"),
        "{result:?}"
    );
}

#[test]
fn usr_03_switch_user_is_refused_while_the_locked_user_has_a_cart() {
    let (_folder, state, _milk) = selling_shop();
    till::checkout_scan(&state, MILK_CODE).expect("scan");
    app::commands::auth::lock(&state).expect("locked");
    assert_eq!(conflict_code(session::logout(&state)), "cart_open_on_lock");
    session::unlock(&state, ADMIN_PASSWORD.into()).expect("unlocked");
    till::checkout_cancel(&state).expect("cancelled");
    app::commands::auth::lock(&state).expect("locked");
    session::logout(&state).expect("switch user");
}

#[test]
fn shf_02_a_cashier_cannot_sell_on_someone_elses_shift() {
    let (_folder, state, _milk) = selling_shop();
    add_cashier(&state);
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");
    let view = till::checkout_state(&state).expect("state");
    assert!(!view.shift.expect("the owner's shift").mine);
    assert!(view.cart.is_none());
    assert_eq!(
        conflict_code(till::checkout_scan(&state, MILK_CODE)),
        "shift_not_yours"
    );
}

/// Saves a product again with changed fields, as the product form would.
fn resave(state: &AppState, product: catalog::Product) {
    let input = app::workflows::products::ProductInput {
        id: Some(product.id),
        kind: product.kind,
        name: product.name,
        sku: product.sku,
        category_id: Some(product.category_id),
        brand_id: product.brand_id,
        unit_id: product.unit_id,
        sale_price: product.sale_price,
        cost_price: product.cost_price,
        tax_rate_id: Some(product.tax_rate_id),
        is_weighed: product.is_weighed,
        scale_item_code: product.scale_item_code,
        track_stock: product.track_stock,
        track_expiry: product.track_expiry,
        preferred_supplier_id: product.preferred_supplier_id,
        description: product.description,
        barcodes: product.barcodes,
        reorder: None,
    };
    app::commands::catalog::product_save(state, input).expect("saved again");
}

fn kit(name: &str) -> app::workflows::products::ProductInput {
    app::workflows::products::ProductInput {
        id: None,
        kind: catalog::ProductKind::Kit,
        name: name.into(),
        sku: String::new(),
        category_id: None,
        brand_id: None,
        unit_id: Id::parse(catalog::UNIT_PIECE).expect("id"),
        sale_price: Money(0),
        cost_price: Money(0),
        tax_rate_id: None,
        is_weighed: false,
        scale_item_code: None,
        track_stock: false,
        track_expiry: false,
        preferred_supplier_id: None,
        description: None,
        barcodes: Vec::new(),
        reorder: None,
    }
}
