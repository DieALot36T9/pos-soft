//! The stock screens' workflows (Req - Inventory): the stock levels list, adjustments
//! with reasons and batches, history, low-stock and near-expiry alerts, stock counts
//! with their backup, and "Verify stock data" after many random changes.

mod common;

use app::commands::catalog as catalog_commands;
use app::commands::inventory as commands;
use app::commands::settings as settings_commands;
use app::session;
use app::state::AppState;
use app::workflows::products::{ProductInput, ReorderInput};
use app::workflows::settings_change::SettingsChange;
use app::workflows::stock::{AdjustBatch, AdjustInput, HistoryQuery, StockFilter, StockShow};
use app::workflows::stock_counts::{CountInput, CountLineInput};
use catalog::{ProductKind, ProductPage};
use chrono::NaiveDate;
use common::*;
use inventory::{CountScope, CountStatus, MoveKind};
use pos_core::{AppError, Id, Money, Qty};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use settings::{InventorySettings, Setting};

fn shop() -> (tempfile::TempDir, AppState) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    (folder, state)
}

fn pieces(count: i64) -> Qty {
    Qty(count * 1000)
}

fn page() -> ProductPage {
    ProductPage {
        offset: 0,
        limit: 50,
    }
}

fn date(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("date")
}

/// A product that keeps stock, with a cost of 20.00 and a Code 128 barcode.
fn product(state: &AppState, name: &str, code: &str) -> Id {
    let input = ProductInput {
        id: None,
        kind: ProductKind::Standard,
        name: name.into(),
        sku: String::new(),
        category_id: None,
        brand_id: None,
        unit_id: Id::parse(catalog::UNIT_PIECE).expect("id"),
        sale_price: Money(3_000),
        cost_price: Money(2_000),
        tax_rate_id: None,
        is_weighed: false,
        scale_item_code: None,
        track_stock: true,
        track_expiry: false,
        preferred_supplier_id: None,
        description: None,
        barcodes: vec![code.into()],
        reorder: None,
    };
    catalog_commands::product_save(state, input).expect("product saved")
}

fn change_product(state: &AppState, id: &Id, change: impl FnOnce(&mut ProductInput)) {
    let product = catalog_commands::product_get(state, id)
        .expect("details")
        .product;
    let mut input = ProductInput {
        id: Some(id.clone()),
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
    change(&mut input);
    catalog_commands::product_save(state, input).expect("product changed");
}

fn adjust(id: &Id, count: i64, reason: &str) -> AdjustInput {
    AdjustInput {
        product_id: id.clone(),
        qty_delta: pieces(count),
        reason_code: reason.into(),
        note: None,
        batch: AdjustBatch::Auto,
    }
}

fn on_hand(state: &AppState, id: &Id) -> Qty {
    commands::stock_product(state, id).expect("stock").on_hand
}

fn set_inventory(state: &AppState, change: impl FnOnce(&mut InventorySettings)) {
    let mut options = InventorySettings::default();
    change(&mut options);
    settings_commands::settings_save(state, &SettingsChange::Inventory(options)).expect("saved");
}

fn conflict_code(result: Result<impl std::fmt::Debug, AppError>) -> String {
    match result {
        Err(AppError::Conflict { code, .. }) => code,
        Err(AppError::Validation { code, .. }) => code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn inv_01_an_adjustment_changes_stock_and_shows_in_history_with_a_balance() {
    let (_folder, state) = shop();
    let milk = product(&state, "Milk 1L", "MILK-1");
    assert_eq!(
        commands::stock_adjust(&state, &adjust(&milk, 10, "opening")).expect("in"),
        pieces(10)
    );
    let mut out = adjust(&milk, -3, "damage");
    out.note = Some("  dropped  ".into());
    assert_eq!(
        commands::stock_adjust(&state, &out).expect("out"),
        pieces(7)
    );

    let query = HistoryQuery {
        product_id: milk.clone(),
        from: None,
        to: None,
        kinds: Vec::new(),
        page: page(),
    };
    let rows = commands::stock_history(&state, &query).expect("history");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].qty_delta, pieces(-3));
    assert_eq!(rows[0].balance_after, pieces(7));
    assert_eq!(rows[0].kind, MoveKind::Adjust);
    assert_eq!(rows[0].reason_name.as_deref(), Some("Damage"));
    assert_eq!(rows[0].note.as_deref(), Some("dropped"));
    assert_eq!(rows[0].unit_cost, Money(2_000));
    assert_eq!(rows[0].user_name.as_deref(), Some("Shop Owner"));
    assert_eq!(rows[1].balance_after, pieces(10));

    let (user, _, details) = last_audit(&state, "stock.adjust");
    assert!(user.is_some());
    assert!(details.contains("\"qty_delta_milli\":-3000"), "{details}");
    assert!(details.contains("\"reason\":\"damage\""), "{details}");
}

#[test]
fn inv_02_reasons_must_fit_and_a_refusal_saves_nothing() {
    let (_folder, state) = shop();
    let milk = product(&state, "Milk 1L", "MILK-1");
    let reasons = commands::stock_reasons(&state).expect("reasons");
    assert!(reasons.iter().any(|reason| reason.code == "damage"));

    let wrong_way = commands::stock_adjust(&state, &adjust(&milk, 5, "damage"));
    assert_eq!(conflict_code(wrong_way), "reason_wrong_direction");
    let no_note = commands::stock_adjust(&state, &adjust(&milk, 5, "other"));
    assert_eq!(conflict_code(no_note), "note_needed");
    let unknown = commands::stock_adjust(&state, &adjust(&milk, 5, "gift"));
    assert_eq!(conflict_code(unknown), "reason_unknown");
    let zero = commands::stock_adjust(&state, &adjust(&milk, 0, "count_error"));
    assert_eq!(conflict_code(zero), "qty_zero");

    assert_eq!(on_hand(&state, &milk), Qty(0));
    assert_eq!(audit_count(&state, "stock.adjust"), 0);
}

#[test]
fn inv_09_kits_and_products_without_stock_are_refused() {
    let (_folder, state) = shop();
    let milk = product(&state, "Milk 1L", "MILK-1");
    change_product(&state, &milk, |input| input.track_stock = false);
    let result = commands::stock_adjust(&state, &adjust(&milk, 5, "opening"));
    assert_eq!(conflict_code(result), "product_keeps_no_stock");
    let looked_up = commands::stock_lookup(&state, "MILK-1");
    assert_eq!(conflict_code(looked_up), "product_keeps_no_stock");
}

#[test]
fn inv_04_negative_stock_follows_the_setting() {
    let (_folder, state) = shop();
    let milk = product(&state, "Milk 1L", "MILK-1");
    commands::stock_adjust(&state, &adjust(&milk, 2, "opening")).expect("in");

    set_inventory(&state, |options| options.allow_negative = false);
    let refused = commands::stock_adjust(&state, &adjust(&milk, -3, "theft"));
    match refused {
        Err(AppError::Conflict { code, params }) => {
            assert_eq!(code, "not_enough_stock");
            assert_eq!(
                params.get("available_milli").map(String::as_str),
                Some("2000")
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(on_hand(&state, &milk), pieces(2));

    set_inventory(&state, |options| options.allow_negative = true);
    commands::stock_adjust(&state, &adjust(&milk, -3, "theft")).expect("allowed");
    assert_eq!(on_hand(&state, &milk), pieces(-1));
    let filter = StockFilter {
        show: StockShow::Negative,
        ..StockFilter::default()
    };
    let list = commands::stock_levels(&state, &filter, page()).expect("list");
    assert_eq!(list.rows.len(), 1);
    assert!(list.rows[0].negative);
    assert_eq!(
        commands::stock_alerts(&state)
            .expect("alerts")
            .negative_count,
        1
    );
}

#[test]
fn inv_05_expiry_tracked_stock_needs_a_dated_batch_and_goes_out_first_expiry_first() {
    let (_folder, state) = shop();
    let yoghurt = product(&state, "Yoghurt", "YOG-1");
    change_product(&state, &yoghurt, |input| input.track_expiry = true);

    let undated = commands::stock_adjust(&state, &adjust(&yoghurt, 5, "opening"));
    assert!(
        matches!(undated, Err(AppError::Validation { .. })),
        "{undated:?}"
    );
    let mut no_date = adjust(&yoghurt, 5, "opening");
    no_date.batch = AdjustBatch::New {
        batch_no: Some("B1".into()),
        expiry_date: None,
    };
    assert_eq!(
        conflict_code(commands::stock_adjust(&state, &no_date)),
        "expiry_needed"
    );

    for (batch_no, expiry, count) in [("LATE", "2026-12-31", 4), ("SOON", "2026-10-20", 3)] {
        let mut input = adjust(&yoghurt, count, "opening");
        input.batch = AdjustBatch::New {
            batch_no: Some(batch_no.into()),
            expiry_date: Some(date(expiry)),
        };
        commands::stock_adjust(&state, &input).expect("batch in");
    }
    // A new batch only makes sense for stock coming in.
    let mut new_out = adjust(&yoghurt, -1, "damage");
    new_out.batch = AdjustBatch::New {
        batch_no: None,
        expiry_date: Some(date("2027-01-01")),
    };
    assert_eq!(
        conflict_code(commands::stock_adjust(&state, &new_out)),
        "new_batch_only_for_stock_in"
    );

    // Out with "Auto": the soonest batch is emptied first, then the next.
    commands::stock_adjust(&state, &adjust(&yoghurt, -4, "damage")).expect("out");
    let stock = commands::stock_product(&state, &yoghurt).expect("stock");
    assert_eq!(stock.on_hand, pieces(3));
    assert_eq!(stock.batches.len(), 1);
    assert_eq!(stock.batches[0].batch_no.as_deref(), Some("LATE"));
    assert_eq!(stock.batches[0].on_hand, pieces(3));

    // A chosen batch.
    let mut chosen = adjust(&yoghurt, -1, "internal_use");
    chosen.batch = AdjustBatch::Existing {
        id: stock.batches[0].id.clone(),
    };
    commands::stock_adjust(&state, &chosen).expect("chosen");
    assert_eq!(on_hand(&state, &yoghurt), pieces(2));
}

#[test]
fn inv_05_near_expiry_shows_on_the_dashboard_and_in_the_list() {
    let (_folder, state) = shop();
    let yoghurt = product(&state, "Yoghurt", "YOG-1");
    change_product(&state, &yoghurt, |input| input.track_expiry = true);
    // Today is 2026-10-05; near expiry is 30 days by default.
    for (batch_no, expiry) in [
        ("OLD", "2026-10-01"),
        ("SOON", "2026-10-20"),
        ("FAR", "2027-03-01"),
    ] {
        let mut input = adjust(&yoghurt, 2, "opening");
        input.batch = AdjustBatch::New {
            batch_no: Some(batch_no.into()),
            expiry_date: Some(date(expiry)),
        };
        commands::stock_adjust(&state, &input).expect("in");
    }

    let expiring = commands::near_expiry(&state).expect("near expiry");
    let numbers: Vec<_> = expiring
        .iter()
        .map(|row| (row.batch_no.clone().unwrap_or_default(), row.expired))
        .collect();
    assert_eq!(
        numbers,
        vec![("OLD".to_string(), true), ("SOON".to_string(), false)]
    );
    let alerts = commands::stock_alerts(&state).expect("alerts");
    assert_eq!(alerts.near_expiry_count, 2);
    assert_eq!(alerts.near_expiry_days, 30);

    let stock = commands::stock_product(&state, &yoghurt).expect("stock");
    let flags: Vec<_> = stock
        .batches
        .iter()
        .map(|batch| (batch.expired, batch.near_expiry))
        .collect();
    assert_eq!(flags, vec![(true, true), (false, true), (false, false)]);

    let filter = StockFilter {
        show: StockShow::Expiring,
        ..StockFilter::default()
    };
    let list = commands::stock_levels(&state, &filter, page()).expect("list");
    assert_eq!(list.rows.len(), 1);
    assert!(list.rows[0].expiring);
}

#[test]
fn inv_03_reorder_level_10_and_stock_9_is_low_stock() {
    let (_folder, state) = shop();
    let milk = product(&state, "Milk 1L", "MILK-1");
    let bread = product(&state, "Bread", "BREAD-1");
    for id in [&milk, &bread] {
        change_product(&state, id, |input| {
            input.reorder = Some(ReorderInput {
                level: pieces(10),
                qty: pieces(24),
            });
        });
    }
    commands::stock_adjust(&state, &adjust(&milk, 10, "opening")).expect("in");
    commands::stock_adjust(&state, &adjust(&bread, 10, "opening")).expect("in");
    assert_eq!(
        commands::stock_alerts(&state)
            .expect("alerts")
            .low_stock_count,
        0
    );

    commands::stock_adjust(&state, &adjust(&milk, -1, "damage")).expect("out");
    let alerts = commands::stock_alerts(&state).expect("alerts");
    assert_eq!(alerts.low_stock_count, 1);
    assert_eq!(alerts.low_stock[0].name, "Milk 1L");
    assert_eq!(alerts.low_stock[0].on_hand, pieces(9));
    assert_eq!(alerts.low_stock[0].reorder_qty, pieces(24));
    assert_eq!(commands::low_stock(&state, page()).expect("low").len(), 1);

    let filter = StockFilter {
        show: StockShow::Low,
        ..StockFilter::default()
    };
    let list = commands::stock_levels(&state, &filter, page()).expect("list");
    let names: Vec<_> = list.rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, vec!["Milk 1L"]);
}

#[test]
fn inv_08_the_levels_list_shows_value_and_filters_by_name() {
    let (_folder, state) = shop();
    let milk = product(&state, "Milk 1L", "MILK-1");
    let bread = product(&state, "Bread", "BREAD-1");
    commands::stock_adjust(&state, &adjust(&milk, 3, "opening")).expect("in");
    commands::stock_adjust(&state, &adjust(&bread, 2, "opening")).expect("in");

    let all = commands::stock_levels(&state, &StockFilter::default(), page()).expect("list");
    assert_eq!(all.total, 2);
    // Sorted by name; each at cost 20.00.
    assert_eq!(all.rows[0].name, "Bread");
    assert_eq!(all.rows[0].value, Money(4_000));
    assert_eq!(all.rows[1].value, Money(6_000));
    assert_eq!(all.total_value, Money(10_000));

    let filter = StockFilter {
        text: Some("MIL".into()),
        ..StockFilter::default()
    };
    let found = commands::stock_levels(&state, &filter, page()).expect("list");
    assert_eq!(found.total, 1);
    assert_eq!(found.rows[0].product_id, milk);
}

#[test]
fn inv_06_a_full_count_scans_reviews_and_posts_after_a_backup() {
    let (_folder, state) = shop();
    let milk = product(&state, "Milk 1L", "MILK-1");
    let bread = product(&state, "Bread", "BREAD-1");
    commands::stock_adjust(&state, &adjust(&milk, 10, "opening")).expect("in");
    commands::stock_adjust(&state, &adjust(&bread, 4, "opening")).expect("in");

    let count = commands::count_create(
        &state,
        &CountInput {
            name: "October count".into(),
            scope: CountScope::Full,
            category_id: None,
            brand_id: None,
            location: None,
        },
    )
    .expect("count");
    let (_, _, details) = last_audit(&state, "stock_count.create");
    assert!(details.contains("SC-000001"), "{details}");

    // Scan the milk twice: 6 + 2. The till then sells 1 (an adjustment here), which
    // must not count as a difference, because expected was kept at the first scan.
    let scanned = commands::stock_lookup(&state, "MILK-1").expect("found");
    for qty in [6, 2] {
        let line = CountLineInput {
            product_id: scanned.product_id.clone(),
            batch_id: None,
            qty: pieces(qty),
        };
        commands::count_line(&state, &count, &line).expect("counted");
    }
    commands::stock_adjust(&state, &adjust(&milk, -1, "damage")).expect("sold");
    let view = commands::count_get(&state, &count).expect("view");
    assert_eq!(view.lines.len(), 1);
    assert_eq!(view.lines[0].counted, pieces(8));
    assert_eq!(view.count.number, "SC-000001");

    let report = commands::count_variances(&state, &count).expect("variances");
    assert_eq!(report.uncounted, 1);
    let milk_row = report
        .rows
        .iter()
        .find(|row| row.product_id == milk)
        .expect("milk");
    assert_eq!(milk_row.expected, pieces(10));
    assert_eq!(milk_row.difference, pieces(-2));
    assert_eq!(milk_row.cost_value, Money(-4_000));
    // Bread was never counted: the full count sets it to 0 (4 × 20.00 missing).
    assert_eq!(report.total_value, Money(-12_000));

    // Bread nobody counted: posting asks first.
    let unconfirmed = commands::count_post(&state, &count, false);
    match unconfirmed {
        Err(AppError::Conflict { code, params }) => {
            assert_eq!(code, "count_uncounted_needs_confirm");
            assert_eq!(params.get("uncounted").map(String::as_str), Some("1"));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(audit_count(&state, "stock_count.post"), 0);

    let posted = commands::count_post(&state, &count, true).expect("posted");
    assert_eq!(posted.rows_written, 2);
    assert!(
        posted.backup_file.ends_with("-pre-count-post.sqlite"),
        "{}",
        posted.backup_file
    );
    assert!(std::path::Path::new(&posted.backup_file).exists());
    assert_eq!(on_hand(&state, &milk), pieces(7));
    assert_eq!(on_hand(&state, &bread), Qty(0));
    assert_eq!(audit_count(&state, "stock_count.post"), 1);

    let history = commands::stock_history(
        &state,
        &HistoryQuery {
            product_id: milk.clone(),
            from: None,
            to: None,
            kinds: vec![MoveKind::Count],
            page: page(),
        },
    )
    .expect("history");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].ref_type.as_deref(), Some("count"));
    assert_eq!(history[0].ref_label.as_deref(), Some("SC-000001"));

    let listed = commands::count_list(&state, page()).expect("list");
    assert_eq!(listed[0].status, CountStatus::Posted);
    assert_eq!(listed[0].posted_by_name.as_deref(), Some("Shop Owner"));
    let again = commands::count_post(&state, &count, true);
    assert_eq!(conflict_code(again), "count_not_open");
}

#[test]
fn inv_06_a_partial_count_leaves_other_products_and_can_be_cancelled() {
    let (_folder, state) = shop();
    let milk = product(&state, "Milk 1L", "MILK-1");
    let bread = product(&state, "Bread", "BREAD-1");
    commands::stock_adjust(&state, &adjust(&milk, 5, "opening")).expect("in");
    commands::stock_adjust(&state, &adjust(&bread, 4, "opening")).expect("in");

    let input = CountInput {
        name: "Aisle 3".into(),
        scope: CountScope::Partial,
        category_id: None,
        brand_id: None,
        location: Some(" Aisle 3 ".into()),
    };
    let first = commands::count_create(&state, &input).expect("count");
    let line = CountLineInput {
        product_id: milk.clone(),
        batch_id: None,
        qty: pieces(6),
    };
    commands::count_line(&state, &first, &line).expect("counted");
    let report = commands::count_variances(&state, &first).expect("report");
    assert_eq!(report.uncounted, 0);
    assert_eq!(report.count.covers, "Aisle 3");
    commands::count_post(&state, &first, false).expect("posted");
    assert_eq!(on_hand(&state, &milk), pieces(6));
    assert_eq!(on_hand(&state, &bread), pieces(4));

    let second = commands::count_create(&state, &input).expect("count");
    commands::count_line(&state, &second, &line).expect("counted");
    commands::count_cancel(&state, &second).expect("cancelled");
    assert_eq!(audit_count(&state, "stock_count.cancel"), 1);
    assert_eq!(on_hand(&state, &milk), pieces(6));
    let closed = commands::count_line(&state, &second, &line);
    assert_eq!(conflict_code(closed), "count_not_open");
    let view = commands::count_get(&state, &second).expect("view");
    assert_eq!(view.count.status, CountStatus::Cancelled);
    assert_eq!(view.count.number, "SC-000002");
}

#[test]
fn counting_and_viewing_need_their_permissions() {
    let (_folder, state) = shop();
    let milk = product(&state, "Milk 1L", "MILK-1");
    let input = CountInput {
        name: "Full".into(),
        scope: CountScope::Full,
        category_id: None,
        brand_id: None,
        location: None,
    };
    let count = commands::count_create(&state, &input).expect("count");
    add_cashier(&state);
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");

    // A cashier counts and sees stock, but can't adjust, start or post counts.
    let line = CountLineInput {
        product_id: milk.clone(),
        batch_id: None,
        qty: pieces(1),
    };
    commands::count_line(&state, &count, &line).expect("a cashier counts");
    commands::stock_levels(&state, &StockFilter::default(), page()).expect("sees stock");
    for result in [
        commands::stock_adjust(&state, &adjust(&milk, 1, "opening")).map(|_| ()),
        commands::count_post(&state, &count, true).map(|_| ()),
        commands::count_create(&state, &input).map(|_| ()),
        commands::stock_verify(&state).map(|_| ()),
    ] {
        assert!(
            matches!(result, Err(AppError::PermissionDenied { .. })),
            "{result:?}"
        );
    }
}

/// M5's exit check: after many random adjustments, batch changes and counts, every
/// stock level still equals the sum of its ledger rows (INV-10).
#[test]
fn inv_10_levels_equal_ledger_after_random_changes() {
    let (_folder, state) = shop();
    let mut random = StdRng::seed_from_u64(5);
    let plain: Vec<Id> = (0..4)
        .map(|n| product(&state, &format!("Plain {n}"), &format!("PLAIN-{n}")))
        .collect();
    let dated: Vec<Id> = (0..2)
        .map(|n| {
            let id = product(&state, &format!("Dated {n}"), &format!("DATED-{n}"));
            change_product(&state, &id, |input| input.track_expiry = true);
            id
        })
        .collect();
    let mut open_count: Option<Id> = None;

    for step in 0..300 {
        let negative_allowed = random.random_bool(0.7);
        if step % 50 == 0 {
            set_inventory(&state, |options| options.allow_negative = negative_allowed);
        }
        match random.random_range(0..10) {
            0..=3 => {
                let id = &plain[random.random_range(0..plain.len())];
                let qty = random.random_range(-5_i64..=8);
                let reason = if qty > 0 { "opening" } else { "damage" };
                // Refusals (zero, not enough stock) are fine: they must save nothing.
                let _ = commands::stock_adjust(&state, &adjust(id, qty, reason));
            }
            4..=6 => {
                let id = &dated[random.random_range(0..dated.len())];
                let qty = random.random_range(-4_i64..=6);
                let mut input = adjust(id, qty, if qty > 0 { "opening" } else { "expiry" });
                if qty > 0 {
                    let day = random.random_range(1..=28);
                    input.batch = AdjustBatch::New {
                        batch_no: Some(format!("B{step}")),
                        expiry_date: NaiveDate::from_ymd_opt(2026, 11, day),
                    };
                }
                let _ = commands::stock_adjust(&state, &input);
            }
            7..=8 => {
                let count = open_count.get_or_insert_with(|| {
                    let input = CountInput {
                        name: format!("Count {step}"),
                        scope: if random.random_bool(0.5) {
                            CountScope::Full
                        } else {
                            CountScope::Partial
                        },
                        category_id: None,
                        brand_id: None,
                        location: None,
                    };
                    commands::count_create(&state, &input).expect("count")
                });
                let id = &plain[random.random_range(0..plain.len())];
                let line = CountLineInput {
                    product_id: id.clone(),
                    batch_id: None,
                    qty: pieces(random.random_range(0..=6)),
                };
                let _ = commands::count_line(&state, count, &line);
            }
            _ => {
                if let Some(count) = open_count.take() {
                    if random.random_bool(0.8) {
                        commands::count_post(&state, &count, true).expect("posted");
                    } else {
                        commands::count_cancel(&state, &count).expect("cancelled");
                    }
                }
            }
        }
    }
    assert!(audit_count(&state, "stock.adjust") > 50);
    assert!(audit_count(&state, "stock_count.post") > 3);
    assert_eq!(commands::stock_verify(&state).expect("verify"), Vec::new());
}
