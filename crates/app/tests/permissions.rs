//! Every command with a permission, called for real: signed out it is refused,
//! and a cashier is refused exactly where the Cashier role lacks the permission
//! (Roles and Permissions, Security Model). A new command with a permission must
//! be added to `calls` below, or `every_guarded_command_is_called_here` fails.

mod common;

use app::commands::checkout::{self as till_commands, WeightInput};
use app::commands::{
    COMMANDS, audit as audit_commands, backup as backup_commands, catalog as catalog_commands,
    inventory as stock_commands, settings as settings_commands, tax as tax_commands,
    users as user_commands,
};
use app::commands::{
    customers as customer_commands, payment_methods as payment_commands,
    pricing as pricing_commands, purchasing as purchasing_commands, receipts as receipt_commands,
    reports as report_commands, returns as return_commands, shifts as shift_commands,
};
use app::commands::{io as io_commands, labels as label_commands};
use app::devices::Devices;
use app::session;
use app::state::AppState;
use app::workflows::catalog_lists::{BrandInput, CategoryInput, UnitInput};
use app::workflows::checkout::MethodKind;
use app::workflows::customer_records::{CustomerPaymentInput, CustomerSave};
use app::workflows::exports::{ExportEntity, ExportRequest, FileFormat};
use app::workflows::import_file::{ImportEntity, ImportRequest, MatchBy};
use app::workflows::labels::{LabelCopies, LabelKind, LabelPick, LabelRequest, LabelSettings};
use app::workflows::payment_methods::PaymentMethodSave;
use app::workflows::pricing_admin::{PriceListSave, PromotionSave, TierSave};
use app::workflows::products::{ProductInput, VariantAttribute};
use app::workflows::purchasing_admin::{OrderSave, SupplierPaymentInput, SupplierSave};
use app::workflows::receiving::{GrnInput, SupplierReturnInput};
use app::workflows::report_runs::ReportRequest;
use app::workflows::returns::{ReturnInput, ReturnLineInput};
use app::workflows::settings_change::SettingsChange;
use app::workflows::shifts::{CashMoveInput, CashMoveKind, CountLine};
use app::workflows::stock::{AdjustBatch, AdjustInput, HistoryQuery, StockFilter};
use app::workflows::stock_counts::{CountInput, CountLineInput};
use app::workflows::users::{RoleInput, UserInput};
use audit::{AuditFilter, AuditPage};
use catalog::{CategoryFields, ProductFilter, ProductKind, ProductPage, UnitFields};
use common::*;
use inventory::CountScope;
use pos_core::{AppError, Bps, Id, Money, Permission, Qty};
use settings::{CreditSettings, Setting};

type Call = fn(&AppState, &str) -> Result<(), AppError>;

fn ok<T>(result: Result<T, AppError>) -> Result<(), AppError> {
    result.map(|_| ())
}

/// One real call for each command that needs a permission. `file` is a backup file.
fn calls() -> Vec<(&'static str, Call)> {
    let mut calls: Vec<(&'static str, Call)> = vec![
        ("settings_get", |state, _| {
            ok(settings_commands::settings_get(state))
        }),
        ("settings_save", |state, _| {
            let change = SettingsChange::Credit(CreditSettings::default_value());
            ok(settings_commands::settings_save(state, &change))
        }),
        ("logs_set_detailed", |state, _| {
            ok(settings_commands::logs_set_detailed(state, false))
        }),
        ("tax_rate_list", |state, _| {
            ok(tax_commands::tax_rate_list(state, false))
        }),
        ("tax_rate_save", |state, _| {
            ok(tax_commands::tax_rate_save(state, rate("Reduced 5%", 500)))
        }),
        ("tax_rate_deactivate", |state, _| {
            ok(tax_commands::tax_rate_deactivate(state, &Id::new()))
        }),
        ("backup_now", |state, _| {
            ok(backup_commands::backup_now(state))
        }),
        ("backup_list", |state, _| {
            ok(backup_commands::backup_list(state))
        }),
        ("backup_restore", |state, file| {
            ok(backup_commands::backup_restore(state, file, None))
        }),
        ("user_list", |state, _| ok(user_commands::user_list(state))),
        ("user_save", |state, _| {
            let input = UserInput {
                id: None,
                username: "karim".into(),
                display_name: "Karim".into(),
                role_id: Id::parse(auth::CASHIER_ROLE_ID).expect("id"),
                active: true,
                pin: Some(pos_core::Secret::new("2468".into())),
                password: None,
            };
            ok(user_commands::user_save(state, input))
        }),
        ("role_list", |state, _| ok(user_commands::role_list(state))),
        ("role_save", |state, _| {
            let input = RoleInput {
                id: None,
                name: "Supervisor".into(),
                permissions: vec![Permission::CheckoutSell],
                max_discount: Bps(0),
            };
            ok(user_commands::role_save(state, input))
        }),
        ("audit_query", |state, _| {
            let page = AuditPage {
                limit: 10,
                offset: 0,
            };
            ok(audit_commands::audit_query(
                state,
                &AuditFilter::default(),
                page,
            ))
        }),
        ("audit_export", |state, file| {
            let target = std::path::Path::new(file).with_extension("xlsx");
            ok(audit_commands::audit_export(
                state,
                &AuditFilter::default(),
                &target.to_string_lossy(),
            ))
        }),
    ];
    calls.extend(catalog_calls());
    calls.extend(stock_calls());
    calls.extend(till_calls());
    calls.extend(till_pricing_calls());
    calls.extend(shift_calls());
    calls.extend(receipt_calls());
    calls.extend(pricing_calls());
    calls.extend(customer_calls());
    calls.extend(return_calls());
    calls.extend(purchasing_calls());
    calls.extend(receiving_calls());
    calls.extend(report_calls());
    calls.extend(io_calls());
    calls.extend(label_calls());
    calls
}

/// The stock and stock count commands (C-M5).
fn stock_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("stock_levels", |state, _| {
            ok(stock_commands::stock_levels(
                state,
                &StockFilter::default(),
                ProductPage {
                    offset: 0,
                    limit: 10,
                },
            ))
        }),
        ("stock_product", |state, _| {
            ok(stock_commands::stock_product(state, &first_product(state)))
        }),
        ("stock_lookup", |state, _| {
            ok(stock_commands::stock_lookup(state, "P000001"))
        }),
        ("stock_history", |state, _| {
            let query = HistoryQuery {
                product_id: first_product(state),
                from: None,
                to: None,
                kinds: Vec::new(),
                page: ProductPage {
                    offset: 0,
                    limit: 10,
                },
            };
            ok(stock_commands::stock_history(state, &query))
        }),
        ("stock_reasons", |state, _| {
            ok(stock_commands::stock_reasons(state))
        }),
        ("stock_adjust", |state, _| {
            let input = AdjustInput {
                product_id: first_product(state),
                qty_delta: Qty(1000),
                reason_code: "opening".into(),
                note: None,
                batch: AdjustBatch::Auto,
            };
            ok(stock_commands::stock_adjust(state, &input))
        }),
        ("low_stock", |state, _| {
            let page = ProductPage {
                offset: 0,
                limit: 10,
            };
            ok(stock_commands::low_stock(state, page))
        }),
        ("near_expiry", |state, _| {
            ok(stock_commands::near_expiry(state))
        }),
        ("stock_alerts", |state, _| {
            ok(stock_commands::stock_alerts(state))
        }),
        ("stock_verify", |state, _| {
            ok(stock_commands::stock_verify(state))
        }),
        ("count_list", |state, _| {
            let page = ProductPage {
                offset: 0,
                limit: 10,
            };
            ok(stock_commands::count_list(state, page))
        }),
        ("count_get", |state, _| {
            ok(stock_commands::count_get(state, &first_count(state)))
        }),
        ("count_create", |state, _| {
            ok(stock_commands::count_create(state, &count_input()))
        }),
        ("count_line", |state, _| {
            let line = CountLineInput {
                product_id: first_product(state),
                batch_id: None,
                qty: Qty(1000),
            };
            ok(stock_commands::count_line(
                state,
                &first_count(state),
                &line,
            ))
        }),
        ("count_variances", |state, _| {
            ok(stock_commands::count_variances(state, &first_count(state)))
        }),
        ("count_post", |state, _| {
            ok(stock_commands::count_post(state, &first_count(state), true))
        }),
        ("count_cancel", |state, _| {
            ok(stock_commands::count_cancel(state, &first_count(state)))
        }),
    ]
}

fn count_input() -> CountInput {
    CountInput {
        name: "Shelf check".into(),
        scope: CountScope::Partial,
        category_id: None,
        brand_id: None,
        location: None,
    }
}

/// The oldest count, read straight from inventory (no permission check). `shop()` made it.
fn first_count(state: &AppState) -> Id {
    state
        .db
        .read(|conn| Ok(inventory::list_counts(conn, inventory::Page::first(100))?))
        .expect("counts")
        .last()
        .expect("a count")
        .id
        .clone()
}

/// The product, unit, category and brand commands (C-M4).
fn catalog_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("product_list", |state, _| {
            let page = ProductPage {
                offset: 0,
                limit: 10,
            };
            ok(catalog_commands::product_list(
                state,
                &ProductFilter::default(),
                page,
            ))
        }),
        ("product_search", |state, _| {
            ok(catalog_commands::product_search(state, "milk", 10))
        }),
        ("product_get", |state, _| {
            ok(catalog_commands::product_get(state, &first_product(state)))
        }),
        ("product_choices", |state, _| {
            ok(catalog_commands::product_choices(state))
        }),
        ("product_save", |state, _| {
            ok(catalog_commands::product_save(state, new_product("Bread")))
        }),
        ("product_archive", |state, _| {
            ok(catalog_commands::product_archive(state, &Id::new()))
        }),
        ("product_unarchive", |state, _| {
            ok(catalog_commands::product_unarchive(state, &Id::new()))
        }),
        ("product_delete", |state, _| {
            ok(catalog_commands::product_delete(state, &Id::new()))
        }),
        ("barcode_generate", |state, _| {
            ok(catalog_commands::barcode_generate(state, &Id::new()))
        }),
        ("variants_generate", |state, _| {
            let size = VariantAttribute {
                name: "Size".into(),
                values: vec!["Small".into()],
            };
            ok(catalog_commands::variants_generate(
                state,
                &Id::new(),
                &[size],
            ))
        }),
        ("kit_save", |state, _| {
            ok(catalog_commands::kit_save(state, &Id::new(), &[]))
        }),
        ("product_image_set", |state, file| {
            ok(catalog_commands::product_image_set(state, &Id::new(), file))
        }),
        ("unit_list", |state, _| {
            ok(catalog_commands::unit_list(state, false))
        }),
        ("unit_save", |state, _| {
            let input = UnitInput {
                id: None,
                fields: UnitFields {
                    name: "Dozen".into(),
                    symbol: "dz".into(),
                    unit_code: None,
                    decimals: 0,
                },
            };
            ok(catalog_commands::unit_save(state, input))
        }),
        ("unit_archive", |state, _| {
            ok(catalog_commands::unit_archive(state, &Id::new()))
        }),
        ("category_list", |state, _| {
            ok(catalog_commands::category_list(state, false))
        }),
        ("category_save", |state, _| {
            let input = CategoryInput {
                id: None,
                fields: CategoryFields::new("Dairy"),
            };
            ok(catalog_commands::category_save(state, input))
        }),
        ("category_archive", |state, _| {
            ok(catalog_commands::category_archive(state, &Id::new()))
        }),
        ("brand_list", |state, _| {
            ok(catalog_commands::brand_list(state, false))
        }),
        ("brand_save", |state, _| {
            let input = BrandInput {
                id: None,
                name: "Aarong".into(),
            };
            ok(catalog_commands::brand_save(state, input))
        }),
        ("brand_archive", |state, _| {
            ok(catalog_commands::brand_archive(state, &Id::new()))
        }),
    ]
}

/// The signed-in user's id, also while locked. None when nobody is signed in.
fn me(state: &AppState) -> Option<Id> {
    session::current(state)
        .ok()
        .flatten()
        .map(|view| view.user_id)
}

/// Makes sure the signed-in user has the till's open shift, without permission checks,
/// so each till command below meets only its own check.
fn ensure_shift(state: &AppState) {
    let Some(user) = me(state) else {
        return;
    };
    let shift = state
        .db
        .read(app::workflows::till::till_shift)
        .expect("shift");
    match shift {
        Some(open) if open.user_id == user => {}
        Some(_) => {
            app::workflows::shifts::force_close(state, &user, None, &[], None).expect("closed");
            app::workflows::shifts::open(state, &user, Money(0), &[]).expect("opened");
        }
        None => {
            app::workflows::shifts::open(state, &user, Money(0), &[]).expect("opened");
        }
    }
}

/// A line in the user's cart: scans the bread first.
fn a_line(state: &AppState) -> Result<Id, AppError> {
    ensure_shift(state);
    till_commands::checkout_scan(state, "BREAD-1")?;
    let cart = till_commands::checkout_state(state)?.cart.expect("cart");
    Ok(cart.lines[0].line_id.clone())
}

fn product_by_code(state: &AppState, code: &str) -> Id {
    state
        .db
        .read(|conn| Ok(catalog::find_by_code(conn, code)?))
        .expect("find")
        .expect("product")
        .id
}

/// The checkout commands (C-M6).
fn till_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("checkout_state", |state, _| {
            ensure_shift(state);
            ok(till_commands::checkout_state(state))
        }),
        ("checkout_scan", |state, _| {
            ensure_shift(state);
            ok(till_commands::checkout_scan(state, "BREAD-1"))
        }),
        ("checkout_search", |state, _| {
            ok(till_commands::checkout_search(state, "bread"))
        }),
        ("checkout_add_product", |state, _| {
            ensure_shift(state);
            let bread = product_by_code(state, "BREAD-1");
            ok(till_commands::checkout_add_product(state, &bread, None))
        }),
        ("checkout_read_scale", |state, _| {
            let potato = product_by_code(state, "POTATO-1");
            ok(till_commands::checkout_read_scale(
                state,
                &Devices::new(None),
                &potato,
            ))
        }),
        ("checkout_add_weighed", |state, _| {
            ensure_shift(state);
            let potato = product_by_code(state, "POTATO-1");
            let typed = WeightInput::Manual { text: "1.5".into() };
            ok(till_commands::checkout_add_weighed(
                state,
                &Devices::new(None),
                &potato,
                &typed,
            ))
        }),
        ("checkout_set_qty", |state, _| {
            let line = a_line(state)?;
            ok(till_commands::checkout_set_qty(state, &line, Qty(2000)))
        }),
        ("checkout_remove_line", |state, _| {
            let line = a_line(state)?;
            ok(till_commands::checkout_remove_line(state, &line))
        }),
        ("checkout_hold", |state, _| {
            ensure_shift(state);
            till_commands::checkout_hold(state, None).ok();
            a_line(state).ok();
            ok(till_commands::checkout_hold(state, Some("later")))
        }),
        ("checkout_held_list", |state, _| {
            ok(till_commands::checkout_held_list(state))
        }),
        ("checkout_resume", |state, _| {
            ensure_shift(state);
            a_line(state).ok();
            till_commands::checkout_hold(state, None)?;
            let held = till_commands::checkout_held_list(state)?;
            ok(till_commands::checkout_resume(state, &held[0].cart_id))
        }),
        ("checkout_cancel", |state, _| {
            ensure_shift(state);
            ok(till_commands::checkout_cancel(state))
        }),
        ("checkout_payment_preview", |state, _| {
            ensure_shift(state);
            ok(till_commands::checkout_payment_preview(state, &[]))
        }),
        ("checkout_complete", |state, _| {
            ensure_shift(state);
            till_commands::checkout_cancel(state).ok();
            a_line(state).ok();
            ok(till_commands::checkout_complete(
                state,
                devices(),
                &[cash(state, 5_000)],
            ))
        }),
    ]
}

/// The till's pricing commands (C-M8): discounts, typed prices, customer, price list.
fn till_pricing_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("checkout_set_discount", |state, _| {
            let line = a_line(state)?;
            ok(till_commands::checkout_set_discount(
                state, &line, None, None,
            ))
        }),
        ("checkout_set_invoice_discount", |state, _| {
            ensure_shift(state);
            ok(till_commands::checkout_set_invoice_discount(
                state, None, None,
            ))
        }),
        ("checkout_price_override", |state, _| {
            // Without a line (signed out) the command must still refuse on its own check.
            let line = a_line(state).unwrap_or_else(|_| Id::new());
            ok(till_commands::checkout_price_override(
                state,
                &line,
                Some(Money(4_000)),
                None,
            ))
        }),
        ("checkout_set_customer", |state, _| {
            ensure_shift(state);
            ok(till_commands::checkout_set_customer(state, None))
        }),
        ("checkout_set_price_list", |state, _| {
            ensure_shift(state);
            ok(till_commands::checkout_set_price_list(state, None, None))
        }),
    ]
}

/// The pricing screens (C-M8).
fn pricing_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("price_list_list", |state, _| {
            ok(pricing_commands::price_list_list(state))
        }),
        ("price_list_save", |state, _| {
            let input = PriceListSave {
                id: None,
                name: format!("List {}", Id::new()),
                active: true,
            };
            ok(pricing_commands::price_list_save(state, &input))
        }),
        ("price_list_items", |state, _| {
            ok(pricing_commands::price_list_items(state, &Id::new()))
        }),
        ("price_list_set_items", |state, _| {
            ok(pricing_commands::price_list_set_items(
                state,
                &Id::new(),
                &[],
            ))
        }),
        ("price_list_remove_items", |state, _| {
            ok(pricing_commands::price_list_remove_items(
                state,
                &Id::new(),
                &[],
            ))
        }),
        ("tier_list", |state, _| {
            ok(pricing_commands::tier_list(state))
        }),
        ("tier_save", |state, _| {
            let input = TierSave {
                id: None,
                name: format!("Tier {}", Id::new()),
                price_list_id: None,
                discount: Bps(200),
                sort: 0,
            };
            ok(pricing_commands::tier_save(state, &input))
        }),
        ("tier_archive", |state, _| {
            ok(pricing_commands::tier_archive(state, &Id::new()))
        }),
        ("promotion_list", |state, _| {
            ok(pricing_commands::promotion_list(state))
        }),
        ("promotion_save", |state, _| {
            let input = PromotionSave {
                id: None,
                promotion: pricing::PromotionInput {
                    name: "Bread week".into(),
                    rule: pricing::PromoRule::PercentOff {
                        percent: Bps(1_000),
                    },
                    starts_at: None,
                    ends_at: None,
                    days_mask: pricing::EVERY_DAY,
                    tier_id: None,
                    targets: vec![pricing::PromoTarget::Product(product_by_code(
                        state, "BREAD-1",
                    ))],
                },
            };
            ok(pricing_commands::promotion_save(state, &input))
        }),
        ("promotion_set_active", |state, _| {
            ok(pricing_commands::promotion_set_active(
                state,
                &Id::new(),
                false,
            ))
        }),
    ]
}

/// The customer set up by `shop`, who owes 500.00.
fn a_customer(state: &AppState) -> Id {
    state
        .db
        .read(|conn| Ok(customers::lookup(conn, "Asha", 1)?))
        .expect("lookup")[0]
        .id
        .clone()
}

/// The customer screens (C-M8).
fn customer_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("customer_lookup", |state, _| {
            ok(customer_commands::customer_lookup(state, "Asha"))
        }),
        ("customer_list", |state, _| {
            ok(customer_commands::customer_list(state, "", true))
        }),
        ("customer_get", |state, _| {
            ok(customer_commands::customer_get(state, &a_customer(state)))
        }),
        ("customer_save", |state, _| {
            let input = CustomerSave {
                id: None,
                details: customers::CustomerDetails::quick("01811000222", "Babul"),
            };
            ok(customer_commands::customer_save(state, &input))
        }),
        ("customer_archive", |state, _| {
            ok(customer_commands::customer_archive(
                state,
                &a_customer(state),
                false,
            ))
        }),
        ("customer_payment", |state, _| {
            ensure_shift(state);
            let input = CustomerPaymentInput {
                customer_id: a_customer(state),
                amount: Money(100),
                method_id: cash_method(state),
                reference: None,
            };
            ok(customer_commands::customer_payment(state, &input))
        }),
        ("points_adjust", |state, _| {
            ok(customer_commands::points_adjust(
                state,
                &a_customer(state),
                10,
                "Welcome gift",
            ))
        }),
        ("customer_statement", |state, _| {
            let day = pos_core::BusinessDate::parse("2026-01-01").expect("date");
            ok(customer_commands::customer_statement(
                state,
                &a_customer(state),
                day,
                day,
            ))
        }),
    ]
}

/// The payment method and shift commands (C-M6).
fn shift_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("payment_method_list", |state, _| {
            ok(payment_commands::payment_method_list(state))
        }),
        ("payment_method_save", |state, _| {
            let input = PaymentMethodSave {
                id: None,
                name: "Card".into(),
                kind: MethodKind::Card,
                requires_reference: false,
                active: true,
                sort: 1,
            };
            ok(payment_commands::payment_method_save(state, &input))
        }),
        ("shift_open", |state, _| {
            if let Some(user) = me(state)
                && let Ok(Some(_)) = state.db.read(app::workflows::till::till_shift)
            {
                app::workflows::shifts::force_close(state, &user, None, &[], None).expect("closed");
            }
            ok(shift_commands::shift_open(state, Money(0), &[]))
        }),
        ("shift_cash_move", |state, _| {
            ensure_shift(state);
            let input = CashMoveInput {
                kind: CashMoveKind::CashIn,
                amount: Money(1_000),
                reason: "Change".into(),
            };
            ok(shift_commands::shift_cash_move(
                state,
                devices(),
                &input,
                None,
            ))
        }),
        ("shift_no_sale", |state, _| {
            ensure_shift(state);
            ok(shift_commands::shift_no_sale(
                state,
                devices(),
                "Change",
                None,
            ))
        }),
        ("shift_close", |state, _| {
            ensure_shift(state);
            till_commands::checkout_cancel(state).ok();
            let counted = CountLine {
                denomination: Money(10_000),
                count: 1,
            };
            ok(shift_commands::shift_close(
                state,
                devices(),
                &[counted],
                None,
            ))
        }),
        ("shift_force_close", |state, _| {
            ensure_shift(state);
            ok(shift_commands::shift_force_close(
                state,
                devices(),
                &[],
                None,
                None,
            ))
        }),
    ]
}

/// Makes sure the till has a sale to reprint, without permission checks.
fn ensure_sale(state: &AppState) {
    let Some(user) = me(state) else {
        return;
    };
    ensure_shift(state);
    app::workflows::cart_lines::scan(state, &user, "BREAD-1").expect("scanned");
    let seller = state
        .db
        .read(|conn| Ok(auth::session_user(conn, &user)?))
        .expect("seller");
    app::workflows::complete_sale::complete(state, &seller, &[cash(state, 5_000)], None)
        .expect("sold");
}

/// Printing, reprinting and the X report (C-M7).
fn receipt_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("hardware_test_print", |state, _| {
            ok(app::commands::hardware::hardware_test_print(
                state,
                devices(),
                settings::PaperWidth::Mm80,
            ))
        }),
        ("shift_x_report", |state, _| {
            ensure_shift(state);
            ok(shift_commands::shift_x_report(state, devices()))
        }),
        ("receipt_reprint_last", |state, _| {
            ensure_sale(state);
            ok(receipt_commands::receipt_reprint_last(state, devices()))
        }),
        ("receipt_reprint", |state, _| {
            ok(receipt_commands::receipt_reprint(
                state,
                devices(),
                "T1-000001",
            ))
        }),
    ]
}

/// A new product with no prices, so only `CatalogManage` is needed to save it.
fn new_product(name: &str) -> ProductInput {
    ProductInput {
        id: None,
        kind: ProductKind::Standard,
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
        track_stock: true,
        track_expiry: false,
        preferred_supplier_id: None,
        description: None,
        barcodes: Vec::new(),
        reorder: None,
    }
}

/// The product `shop()` made, read straight from the catalog (no permission check).
fn first_product(state: &AppState) -> Id {
    let page = ProductPage {
        offset: 0,
        limit: 1,
    };
    state
        .db
        .read(|conn| Ok(catalog::list(conn, &ProductFilter::default(), page)?))
        .expect("list")
        .rows[0]
        .id
        .clone()
}

fn needed(name: &str) -> Permission {
    COMMANDS
        .iter()
        .find(|(each, _)| *each == name)
        .and_then(|(_, permission)| *permission)
        .unwrap_or_else(|| panic!("{name} has no permission in COMMANDS"))
}

/// The permissions the Cashier role has by default ("C" in Roles and Permissions).
fn cashier_has(permission: Permission) -> bool {
    matches!(
        permission,
        Permission::CheckoutSell
            | Permission::CheckoutCancelCart
            | Permission::CheckoutHoldResume
            | Permission::CheckoutManualWeight
            | Permission::DiscountApply
            | Permission::SellOnCredit
            | Permission::ReceiptReprintLast
            | Permission::ShiftOpenCloseOwn
            | Permission::CustomerViewCreate
            | Permission::CustomerReceivePayment
            | Permission::StockView
            | Permission::StockCountEnter
            | Permission::LabelsPrint
            | Permission::BackupRun
    )
}

/// Commands that take an admin's approval answer `OverrideRequired` instead.
/// The returns commands (C-M9).
fn return_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("return_lookup", |state, _| {
            // A sale of bread to look up (nothing is sold when signed out or locked).
            a_line(state).ok();
            till_commands::checkout_complete(state, devices(), &[cash(state, 5_000)]).ok();
            let invoice = state
                .db
                .read(|conn| {
                    let till: settings::TillSettings = settings::get(conn)?;
                    let till_id = till.till_id.expect("till");
                    Ok(sales::last_sale(conn, &till_id)?)
                })
                .expect("read")
                .map_or_else(|| "T1-000001".to_string(), |sale| sale.invoice_no);
            ok(return_commands::return_lookup(state, &invoice))
        }),
        ("return_preview", |state, _| {
            let bread = product_by_code(state, "BREAD-1");
            ok(return_commands::return_preview(
                state,
                &without_receipt(state, &bread),
            ))
        }),
        ("return_complete", |state, _| {
            ensure_shift(state);
            ok(return_commands::return_complete(
                state,
                devices(),
                &with_unknown_receipt(state),
                None,
            ))
        }),
        ("exchange_start", |state, _| {
            ensure_shift(state);
            ok(return_commands::exchange_start(
                state,
                devices(),
                &with_unknown_receipt(state),
                None,
            ))
        }),
        ("exchange_cash_out", |state, _| {
            ensure_shift(state);
            ok(return_commands::exchange_cash_out(state, devices()))
        }),
        ("sale_void", |state, _| {
            ok(return_commands::sale_void(
                state,
                devices(),
                &Id::new(),
                "Mistake",
                None,
            ))
        }),
    ]
}

/// One loaf back without a receipt, refunded in cash.
fn without_receipt(state: &AppState, product: &Id) -> ReturnInput {
    ReturnInput {
        sale_id: None,
        lines: vec![ReturnLineInput {
            sale_line_id: None,
            product_id: Some(product.clone()),
            qty: Qty(1_000),
            restock: true,
            unit_price: None,
        }],
        reason: Some("Stale".into()),
        refund_method_id: cash_method(state),
        reference: None,
    }
}

/// A return against a sale that doesn't exist: only the permission check is tested.
fn with_unknown_receipt(state: &AppState) -> ReturnInput {
    ReturnInput {
        sale_id: Some(Id::new()),
        lines: vec![ReturnLineInput {
            sale_line_id: Some(Id::new()),
            product_id: None,
            qty: Qty(1_000),
            restock: true,
            unit_price: None,
        }],
        reason: Some("Wrong size".into()),
        refund_method_id: cash_method(state),
        reference: None,
    }
}

/// The supplier and purchase order commands (C-M10). Each call meets its permission check first; with the
/// permission, unknown ids are refused as not found, which is fine here.
fn purchasing_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("supplier_list", |state, _| {
            ok(purchasing_commands::supplier_list(state, true))
        }),
        ("supplier_get", |state, _| {
            ok(purchasing_commands::supplier_get(state, &Id::new()))
        }),
        ("supplier_save", |state, _| {
            let input = SupplierSave {
                id: None,
                input: purchasing::SupplierInput {
                    name: "Rahim Traders".into(),
                    ..purchasing::SupplierInput::default()
                },
            };
            ok(purchasing_commands::supplier_save(state, &input))
        }),
        ("supplier_archive", |state, _| {
            ok(purchasing_commands::supplier_archive(state, &Id::new()))
        }),
        ("po_list", |state, _| {
            ok(purchasing_commands::po_list(state, None))
        }),
        ("po_get", |state, _| {
            ok(purchasing_commands::po_get(state, &Id::new()))
        }),
        ("po_save", |state, _| {
            let input = OrderSave {
                id: None,
                supplier_id: Id::new(),
                expected_date: None,
                notes: None,
                lines: Vec::new(),
            };
            ok(purchasing_commands::po_save(state, &input))
        }),
        ("po_suggest", |state, _| {
            ok(purchasing_commands::po_suggest(state, &Id::new()))
        }),
        ("po_mark_sent", |state, _| {
            ok(purchasing_commands::po_mark_sent(state, &Id::new()))
        }),
        ("po_export", |state, file| {
            ok(purchasing_commands::po_export(
                state,
                &Id::new(),
                &format!("{file}.xlsx"),
            ))
        }),
        ("po_cancel", |state, _| {
            ok(purchasing_commands::po_cancel(state, &Id::new()))
        }),
    ]
}

/// The receiving, supplier return and payables commands (C-M10).
fn receiving_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("grn_start", |state, _| {
            ok(purchasing_commands::grn_start(state, &Id::new()))
        }),
        ("grn_preview", |state, _| {
            ok(purchasing_commands::grn_preview(state, &empty_grn()))
        }),
        ("grn_save", |state, _| {
            ok(purchasing_commands::grn_save(state, &empty_grn()))
        }),
        ("grn_list", |state, _| {
            ok(purchasing_commands::grn_list(state))
        }),
        ("grn_get", |state, _| {
            ok(purchasing_commands::grn_get(state, &Id::new()))
        }),
        ("supplier_return_save", |state, _| {
            let input = SupplierReturnInput {
                supplier_id: Id::new(),
                grn_id: None,
                reason: None,
                lines: Vec::new(),
            };
            ok(purchasing_commands::supplier_return_save(state, &input))
        }),
        ("supplier_return_list", |state, _| {
            ok(purchasing_commands::supplier_return_list(state))
        }),
        ("payables_aging", |state, _| {
            ok(purchasing_commands::payables_aging(state))
        }),
        ("supplier_statement", |state, _| {
            let day = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).expect("date");
            ok(purchasing_commands::supplier_statement(
                state,
                &Id::new(),
                day,
                day,
            ))
        }),
        ("supplier_statement_export", |state, file| {
            let day = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).expect("date");
            let file = format!("{file}.xlsx");
            ok(purchasing_commands::supplier_statement_export(
                state,
                &Id::new(),
                day,
                day,
                &file,
            ))
        }),
        ("supplier_payment", |state, _| {
            let input = SupplierPaymentInput {
                supplier_id: Id::new(),
                amount: Money(1_000),
                method_id: cash_method(state),
                reference: None,
                from_drawer: false,
            };
            ok(purchasing_commands::supplier_payment(
                state,
                devices(),
                &input,
            ))
        }),
    ]
}

/// A delivery with no lines from a supplier that doesn't exist.
/// The report commands (C-M11). The request is the daily summary, which a cashier may
/// not see (RPT-10); their own shift's report is tested in `tests/reports.rs`.
fn report_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("report_choices", |state, _| {
            ok(report_commands::report_choices(state))
        }),
        ("report_customer_search", |state, _| {
            ok(report_commands::report_customer_search(state, "Asha"))
        }),
        ("report_shifts", |state, _| {
            ok(report_commands::report_shifts(state))
        }),
        ("report_run", |state, _| {
            ok(report_commands::report_run(state, &a_daily_summary()))
        }),
        ("report_export_xlsx", |state, file| {
            let excel = format!("{file}.xlsx");
            ok(report_commands::report_export_xlsx(
                state,
                &a_daily_summary(),
                &excel,
            ))
        }),
        ("report_print", |state, _| {
            ok(report_commands::report_print(
                state,
                devices(),
                &a_daily_summary(),
            ))
        }),
    ]
}

/// The import and export commands (C-M12). The file to import doesn't exist, so an
/// allowed call fails on the file, not on the permission.
fn io_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("import_preview", |state, file| {
            ok(io_commands::import_preview(state, &an_import(file)))
        }),
        ("import_commit", |state, file| {
            ok(io_commands::import_commit(state, &an_import(file)))
        }),
        ("import_progress", |state, _| {
            ok(io_commands::import_progress(state))
        }),
        ("import_cancel", |state, _| {
            ok(io_commands::import_cancel(state))
        }),
        ("import_errors_save", |state, file| {
            ok(io_commands::import_errors_save(
                state,
                &[],
                &format!("{file}-errors.csv"),
            ))
        }),
        ("template_download", |state, file| {
            ok(io_commands::template_download(
                state,
                ImportEntity::Products,
                FileFormat::Csv,
                &format!("{file}-template.csv"),
            ))
        }),
        ("export_entity", |state, file| {
            let request = ExportRequest {
                entity: ExportEntity::Customers,
                format: FileFormat::Csv,
                file: format!("{file}-customers.csv"),
                price_list_id: None,
            };
            ok(io_commands::export_entity(state, &request))
        }),
        ("io_price_lists", |state, _| {
            ok(io_commands::io_price_lists(state))
        }),
    ]
}

fn an_import(file: &str) -> ImportRequest {
    ImportRequest {
        entity: ImportEntity::Products,
        file: format!("{file}-missing.csv"),
        sheet: None,
        mapping: None,
        match_by: MatchBy::Sku,
        create_missing: false,
        skip_errors: false,
        price_list_id: None,
    }
}

/// The label commands (C-M12). A cashier may print labels.
fn label_calls() -> Vec<(&'static str, Call)> {
    vec![
        ("labels_pick", |state, _| {
            let pick = LabelPick::Products {
                ids: vec![first_product(state)],
            };
            ok(label_commands::labels_pick(state, &pick))
        }),
        ("labels_preview", |state, _| {
            ok(label_commands::labels_preview(
                state,
                devices(),
                &a_label_request(state),
            ))
        }),
        ("labels_print", |state, _| {
            ok(label_commands::labels_print(
                state,
                devices(),
                &a_label_request(state),
            ))
        }),
    ]
}

fn a_label_request(state: &AppState) -> LabelRequest {
    LabelRequest {
        settings: LabelSettings {
            kind: LabelKind::Label,
            width_mm: 58,
            height_mm: 40,
            gap_mm: 2,
            show_name: true,
            show_price: true,
            show_unit_price: false,
            show_barcode: true,
            packed_on: None,
            expires_on: None,
        },
        items: vec![LabelCopies {
            product_id: first_product(state),
            copies: 1,
        }],
    }
}

fn a_daily_summary() -> ReportRequest {
    ReportRequest::DailySummary {
        date: pos_core::BusinessDate::parse("2026-01-01").expect("date"),
        till_id: None,
    }
}

fn empty_grn() -> GrnInput {
    GrnInput {
        supplier_id: Id::new(),
        order_id: None,
        supplier_invoice_no: None,
        invoice_total: Money(0),
        notes: None,
        lines: Vec::new(),
    }
}

const TAKES_APPROVAL: &[&str] = &[
    "backup_restore",
    "shift_cash_move",
    "shift_no_sale",
    "shift_force_close",
    "checkout_price_override",
    "checkout_set_price_list",
    "return_complete",
    "exchange_start",
    "sale_void",
];

fn shop() -> (tempfile::TempDir, AppState, String) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    add_cashier(&state);
    catalog_commands::product_save(&state, new_product("Milk")).expect("a product");
    till_product(
        &state,
        "Bread",
        "BREAD-1",
        5_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    till_product(
        &state,
        "Potato",
        "POTATO-1",
        6_000,
        catalog::UNIT_KG,
        true,
        None,
    );
    stock_commands::count_create(&state, &count_input()).expect("a count");
    add_customer_who_owes(&state);
    let file = backup_commands::backup_now(&state).expect("backed up").file;
    (folder, state, file)
}

/// Asha, who owes 500.00, so a debt payment has something to pay off.
fn add_customer_who_owes(state: &AppState) {
    let input = CustomerSave {
        id: None,
        details: customers::CustomerDetails::quick("01711123456", "Asha"),
    };
    let id = customer_commands::customer_save(state, &input).expect("customer");
    state
        .db
        .write(|tx| {
            let ctx = customers::LedgerCtx {
                at: pos_core::Timestamp(0),
                business_date: pos_core::BusinessDate::parse("2026-01-01")?,
                user_id: None,
            };
            let kind = customers::CreditKind::CreditSale;
            customers::add_credit(tx, &id, Money(50_000), kind, None, None, &ctx)?;
            Ok(())
        })
        .expect("debt");
}

#[test]
fn every_guarded_command_is_called_here() {
    let mut called: Vec<&str> = calls().iter().map(|(name, _)| *name).collect();
    let mut guarded: Vec<&str> = COMMANDS
        .iter()
        .filter(|(_, permission)| permission.is_some())
        .map(|(name, _)| *name)
        .collect();
    called.sort_unstable();
    guarded.sort_unstable();
    assert_eq!(called, guarded);
}

#[test]
fn the_cashier_role_has_the_documented_defaults() {
    let (_folder, state, _file) = shop();
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");
    let view = session::current(&state)
        .expect("current")
        .expect("signed in");
    for permission in Permission::ALL {
        assert_eq!(
            view.permissions.contains(&permission),
            cashier_has(permission),
            "{permission:?}"
        );
    }
}

#[test]
fn signed_out_every_guarded_command_is_refused() {
    let (_folder, state, file) = shop();
    session::logout(&state).expect("out");
    for (name, call) in calls() {
        let permission = needed(name);
        let result = call(&state, &file);
        assert!(
            matches!(result, Err(AppError::PermissionDenied { permission: p }) if p == permission),
            "{name}: {result:?}"
        );
    }
}

#[test]
fn a_cashier_is_refused_exactly_where_the_role_lacks_the_permission() {
    let (_folder, state, file) = shop();
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");
    for (name, call) in calls() {
        let permission = needed(name);
        let result = call(&state, &file);
        if cashier_has(permission) {
            assert!(result.is_ok(), "{name}: {result:?}");
        } else if TAKES_APPROVAL.contains(&name) {
            assert!(
                matches!(result, Err(AppError::OverrideRequired { permission: p }) if p == permission),
                "{name}: {result:?}"
            );
        } else {
            assert!(
                matches!(result, Err(AppError::PermissionDenied { permission: p }) if p == permission),
                "{name}: {result:?}"
            );
        }
    }
}

#[test]
fn an_admin_passes_every_permission_check() {
    let (_folder, state, file) = shop();
    for (name, call) in calls() {
        let result = call(&state, &file);
        assert!(
            !matches!(
                result,
                Err(AppError::PermissionDenied { .. } | AppError::OverrideRequired { .. })
            ),
            "{name}: {result:?}"
        );
    }
}

#[test]
fn a_locked_till_refuses_every_guarded_command() {
    let (_folder, state, file) = shop();
    app::commands::auth::lock(&state).expect("locked");
    for (name, call) in calls() {
        let result = call(&state, &file);
        assert!(
            matches!(&result, Err(AppError::Conflict { code, .. }) if code == "till_locked"),
            "{name}: {result:?}"
        );
    }
}
