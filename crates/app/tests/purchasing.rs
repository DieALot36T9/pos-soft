//! Purchasing through the app (C-M10, Req - Purchasing PUR-01 to PUR-06): suppliers,
//! purchase orders with suggestions from low stock, receiving goods with batches and
//! the cost update (ADR-018), supplier returns, payments, aging and statements.

mod common;

use app::commands::inventory as stock_commands;
use app::commands::purchasing as purchasing_commands;
use app::commands::settings as settings_commands;
use app::state::AppState;
use app::workflows::products::{ProductInput, ReorderInput};
use app::workflows::purchasing_admin::{
    OrderLineInput, OrderSave, SupplierPaymentInput, SupplierSave,
};
use app::workflows::receiving::{
    GrnInput, GrnLineInput, SupplierReturnInput, SupplierReturnLineInput,
};
use app::workflows::settings_change::SettingsChange;
use chrono::NaiveDate;
use common::*;
use pos_core::{AppError, Id, Money, Permission, Qty};
use purchasing::{LedgerKind, OrderStatus, SupplierInput};
use settings::{CostMethod, InventorySettings, Setting};

fn shop() -> (tempfile::TempDir, AppState) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    add_cashier(&state);
    (folder, state)
}

fn supplier(state: &AppState, name: &str, terms_days: u32) -> Id {
    let save = SupplierSave {
        id: None,
        input: SupplierInput {
            name: name.into(),
            phone: Some("01711-000000".into()),
            payment_terms_days: terms_days,
            ..SupplierInput::default()
        },
    };
    purchasing_commands::supplier_save(state, &save).expect("supplier saved")
}

/// A product costing `cost_minor`, sold at double, from `supplier`.
fn product(
    state: &AppState,
    name: &str,
    cost_minor: i64,
    track_expiry: bool,
    supplier: Option<&Id>,
    reorder: Option<(i64, i64)>,
) -> Id {
    let input = product_input(name, cost_minor, track_expiry, supplier, reorder);
    app::commands::catalog::product_save(state, input).expect("product saved")
}

fn product_input(
    name: &str,
    cost_minor: i64,
    track_expiry: bool,
    supplier: Option<&Id>,
    reorder: Option<(i64, i64)>,
) -> ProductInput {
    ProductInput {
        id: None,
        kind: catalog::ProductKind::Standard,
        name: name.into(),
        sku: String::new(),
        category_id: None,
        brand_id: None,
        unit_id: Id::parse(catalog::UNIT_PIECE).expect("unit"),
        sale_price: Money(cost_minor * 2),
        cost_price: Money(cost_minor),
        tax_rate_id: None,
        is_weighed: false,
        scale_item_code: None,
        track_stock: true,
        track_expiry,
        preferred_supplier_id: supplier.cloned(),
        description: None,
        barcodes: Vec::new(),
        reorder: reorder.map(|(level, qty)| ReorderInput {
            level: Qty(level * 1000),
            qty: Qty(qty * 1000),
        }),
    }
}

fn on_hand(state: &AppState, product: &Id) -> Qty {
    stock_commands::stock_product(state, product)
        .expect("stock")
        .on_hand
}

fn cost(state: &AppState, product: &Id) -> Money {
    stock_commands::stock_product(state, product)
        .expect("stock")
        .cost_price
}

fn balance(state: &AppState, supplier: &Id) -> Money {
    purchasing_commands::supplier_get(state, supplier)
        .expect("supplier")
        .balance
}

fn line(product: &Id, units: i64, cost_minor: i64) -> GrnLineInput {
    GrnLineInput {
        order_line_id: None,
        product_id: product.clone(),
        qty: Qty(units * 1000),
        unit_cost: Money(cost_minor),
        batch_no: None,
        expiry_date: None,
    }
}

fn delivery(supplier: &Id, invoice_minor: i64, lines: Vec<GrnLineInput>) -> GrnInput {
    GrnInput {
        supplier_id: supplier.clone(),
        order_id: None,
        supplier_invoice_no: Some("INV-77".into()),
        invoice_total: Money(invoice_minor),
        notes: None,
        lines,
    }
}

fn date(text: &str) -> NaiveDate {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("date")
}

fn code_of<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
    match result {
        Err(AppError::Validation { code, .. }) | Err(AppError::Conflict { code, .. }) => code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn pur_03_adr_018_receiving_updates_the_cost_by_moving_average() {
    let (_folder, state) = shop();
    let rahim = supplier(&state, "Rahim Traders", 30);
    let rice = product(&state, "Rice 5kg", 5_000, false, Some(&rahim), None);
    stock_in(&state, &rice, Qty(10_000));

    let input = delivery(&rahim, 112_000, vec![line(&rice, 20, 5_600)]);
    let preview = purchasing_commands::grn_preview(&state, &input).expect("preview");
    assert_eq!(preview.lines_total, Money(112_000));
    assert_eq!(preview.difference, Money::zero());
    assert_eq!(
        (preview.lines[0].old_cost, preview.lines[0].new_cost),
        (Money(5_000), Money(5_400))
    );

    let done = purchasing_commands::grn_save(&state, &input).expect("received");
    assert_eq!(done.number, "GRN-000001");
    assert!(!done.total_differs);
    assert_eq!(done.cost_changes.len(), 1);
    assert_eq!(
        done.cost_changes[0].new_cost,
        Money(5_400),
        "(10×50 + 20×56) ÷ 30"
    );
    assert_eq!(cost(&state, &rice), Money(5_400));
    assert_eq!(on_hand(&state, &rice), Qty(30_000));
    assert_eq!(balance(&state, &rahim), Money(112_000));
    let (_, _, details) = last_audit(&state, "grn.save");
    assert!(details.contains("GRN-000001"), "{details}");
    // The price history says the cost changed because of a delivery.
    let history = app::commands::catalog::product_get(&state, &rice)
        .expect("product")
        .price_history;
    assert!(
        history
            .iter()
            .any(|row| row.reason.as_deref() == Some("grn")),
        "{history:?}"
    );
}

#[test]
fn pur_03_adr_018_no_stock_or_last_cost_takes_the_received_cost() {
    let (_folder, state) = shop();
    let rahim = supplier(&state, "Rahim Traders", 0);
    let oil = product(&state, "Oil 1L", 15_000, false, None, None);
    purchasing_commands::grn_save(
        &state,
        &delivery(&rahim, 32_000, vec![line(&oil, 2, 16_000)]),
    )
    .expect("received");
    assert_eq!(
        cost(&state, &oil),
        Money(16_000),
        "no stock before: the received cost"
    );

    let mut rules = InventorySettings::default_value();
    rules.cost_method = CostMethod::LastCost;
    settings_commands::settings_save(&state, &SettingsChange::Inventory(rules)).expect("saved");
    purchasing_commands::grn_save(
        &state,
        &delivery(&rahim, 17_000, vec![line(&oil, 1, 17_000)]),
    )
    .expect("received");
    assert_eq!(cost(&state, &oil), Money(17_000), "last purchase cost");
}

#[test]
fn pur_03_dated_goods_need_an_expiry_and_come_in_as_a_batch() {
    let (_folder, state) = shop();
    let rahim = supplier(&state, "Rahim Traders", 0);
    let milk = product(&state, "Milk 1L", 7_000, true, None, None);
    let input = delivery(&rahim, 70_000, vec![line(&milk, 10, 7_000)]);
    assert_eq!(
        code_of(purchasing_commands::grn_save(&state, &input)),
        "expiry_needed"
    );
    assert_eq!(on_hand(&state, &milk), Qty(0), "nothing saved");
    assert_eq!(balance(&state, &rahim), Money::zero());

    let mut dated = input.clone();
    dated.lines[0].batch_no = Some("B-42".into());
    dated.lines[0].expiry_date = Some(date("2027-01-31"));
    let done = purchasing_commands::grn_save(&state, &dated).expect("received");
    assert_eq!(done.number, "GRN-000001", "the refused try used no number");
    let batches = stock_commands::stock_product(&state, &milk)
        .expect("stock")
        .batches;
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].batch_no.as_deref(), Some("B-42"));
    assert_eq!(batches[0].expiry_date, Some(date("2027-01-31")));
    assert_eq!(batches[0].on_hand, Qty(10_000));
}

#[test]
fn pur_03_a_different_invoice_total_is_a_warning_and_the_balance_uses_the_invoice() {
    let (_folder, state) = shop();
    let rahim = supplier(&state, "Rahim Traders", 0);
    let salt = product(&state, "Salt", 2_000, false, None, None);
    let input = delivery(&rahim, 21_000, vec![line(&salt, 10, 2_000)]);
    let preview = purchasing_commands::grn_preview(&state, &input).expect("preview");
    assert_eq!(preview.difference, Money(1_000));
    let done = purchasing_commands::grn_save(&state, &input).expect("received");
    assert!(done.total_differs);
    assert_eq!(balance(&state, &rahim), Money(21_000));
}

#[test]
fn pur_02_suggest_order_send_receive_in_parts() {
    let (_folder, state) = shop();
    let rahim = supplier(&state, "Rahim Traders", 0);
    let other = supplier(&state, "Karim Foods", 0);
    // Rice is low (5 below level 10) and comes from Rahim; sugar is low but from Karim.
    let rice = product(
        &state,
        "Rice 5kg",
        5_000,
        false,
        Some(&rahim),
        Some((10, 24)),
    );
    let sugar = product(
        &state,
        "Sugar 1kg",
        1_000,
        false,
        Some(&other),
        Some((10, 12)),
    );
    stock_in(&state, &rice, Qty(5_000));
    stock_in(&state, &sugar, Qty(1_000));

    let suggested = purchasing_commands::po_suggest(&state, &rahim).expect("suggested");
    assert_eq!(suggested.len(), 1);
    assert_eq!(suggested[0].product_id, rice);
    assert_eq!(suggested[0].qty, Qty(24_000));
    assert_eq!(suggested[0].unit_cost, Money(5_000));

    let save = OrderSave {
        id: None,
        supplier_id: rahim.clone(),
        expected_date: Some(date("2026-10-20")),
        notes: Some("Morning delivery".into()),
        lines: vec![OrderLineInput {
            product_id: rice.clone(),
            qty: Qty(24_000),
            unit_cost: Money(5_000),
        }],
    };
    let order = purchasing_commands::po_save(&state, &save).expect("saved");
    let view = purchasing_commands::po_get(&state, &order).expect("order");
    assert_eq!(view.order.number, "PO-000001");
    assert_eq!(view.order.status, OrderStatus::Draft);
    assert_eq!(
        view.order.lines[0].product_name, "Rice 5kg",
        "name from the catalog"
    );
    assert_eq!(view.total, Money(120_000));

    let folder = tempfile::tempdir().expect("folder");
    let file = folder.path().join("po.xlsx");
    purchasing_commands::po_export(&state, &order, file.to_str().expect("path")).expect("exported");
    let table = io::read_file(&file, None).expect("read back");
    assert_eq!(table.headers[1], "Product");
    assert_eq!(table.rows[0][1], "Rice 5kg");
    let view = purchasing_commands::po_get(&state, &order).expect("order");
    assert_eq!(view.order.status, OrderStatus::Sent, "exporting sends it");
    assert_eq!(
        code_of(purchasing_commands::po_save(
            &state,
            &OrderSave {
                id: Some(order.clone()),
                ..save
            }
        )),
        "order_not_draft"
    );

    let start = purchasing_commands::grn_start(&state, &order).expect("form");
    assert_eq!(start.lines[0].remaining, Qty(24_000));
    let mut first = delivery(&rahim, 50_000, vec![line(&rice, 10, 5_000)]);
    first.order_id = Some(order.clone());
    first.lines[0].order_line_id = Some(start.lines[0].order_line_id.clone());
    let done = purchasing_commands::grn_save(&state, &first).expect("part");
    assert_eq!(done.order_status, Some(OrderStatus::Partial));
    let start = purchasing_commands::grn_start(&state, &order).expect("form");
    assert_eq!(start.lines[0].remaining, Qty(14_000));
    let mut rest = delivery(&rahim, 75_000, vec![line(&rice, 15, 5_000)]);
    rest.order_id = Some(order.clone());
    rest.lines[0].order_line_id = Some(start.lines[0].order_line_id.clone());
    let done = purchasing_commands::grn_save(&state, &rest).expect("rest");
    assert_eq!(done.order_status, Some(OrderStatus::Received));
    assert_eq!(done.over_received, vec!["Rice 5kg".to_string()]);
    assert_eq!(on_hand(&state, &rice), Qty(30_000));
    let received = purchasing_commands::po_list(&state, Some(OrderStatus::Received)).expect("list");
    assert_eq!(received.rows.len(), 1);
}

#[test]
fn pur_02_a_draft_can_be_cancelled_and_an_archived_supplier_gets_no_orders() {
    let (_folder, state) = shop();
    let rahim = supplier(&state, "Rahim Traders", 0);
    let rice = product(&state, "Rice 5kg", 5_000, false, None, None);
    let save = OrderSave {
        id: None,
        supplier_id: rahim.clone(),
        expected_date: None,
        notes: None,
        lines: vec![OrderLineInput {
            product_id: rice,
            qty: Qty(1_000),
            unit_cost: Money(5_000),
        }],
    };
    let order = purchasing_commands::po_save(&state, &save).expect("saved");
    purchasing_commands::po_cancel(&state, &order).expect("cancelled");
    let view = purchasing_commands::po_get(&state, &order).expect("order");
    assert_eq!(view.order.status, OrderStatus::Cancelled);
    assert_eq!(audit_count(&state, "purchase_order.change"), 2);

    purchasing_commands::supplier_archive(&state, &rahim).expect("archived");
    assert_eq!(
        code_of(purchasing_commands::po_save(&state, &save)),
        "supplier_archived"
    );
    let listed = purchasing_commands::supplier_list(&state, false).expect("list");
    assert!(listed.rows.is_empty(), "archived suppliers are hidden");
    assert_eq!(
        purchasing_commands::supplier_list(&state, true)
            .expect("list")
            .rows
            .len(),
        1
    );
}

#[test]
fn pur_04_goods_sent_back_leave_stock_and_lower_the_balance_at_cost() {
    let (_folder, state) = shop();
    let rahim = supplier(&state, "Rahim Traders", 0);
    let milk = product(&state, "Milk 1L", 7_000, true, None, None);
    let mut input = delivery(&rahim, 70_000, vec![line(&milk, 10, 7_000)]);
    input.lines[0].expiry_date = Some(date("2027-01-31"));
    let grn = purchasing_commands::grn_save(&state, &input).expect("received");
    let received = purchasing_commands::grn_get(&state, &grn.grn_id).expect("grn");
    let batch = received.lines[0].inventory_batch_id.clone().expect("batch");

    let sent_back = SupplierReturnInput {
        supplier_id: rahim.clone(),
        grn_id: Some(grn.grn_id.clone()),
        reason: Some("Leaking".into()),
        lines: vec![SupplierReturnLineInput {
            product_id: milk.clone(),
            qty: Qty(2_000),
            unit_cost: None,
            batch_id: Some(batch),
        }],
    };
    let saved = purchasing_commands::supplier_return_save(&state, &sent_back).expect("sent back");
    assert_eq!(saved.number, "SR-000001");
    assert_eq!(saved.total, Money(14_000), "at the current cost");
    assert_eq!(on_hand(&state, &milk), Qty(8_000));
    assert_eq!(balance(&state, &rahim), Money(56_000));
    let listed = purchasing_commands::supplier_return_list(&state).expect("list");
    assert_eq!(listed[0].grn_number.as_deref(), Some(grn.number.as_str()));
    let mismatches = state
        .db
        .read(|conn| Ok(inventory::verify(conn)?))
        .expect("verify");
    assert!(mismatches.is_empty(), "{mismatches:?}");
}

#[test]
fn pur_05_a_cash_payment_from_the_drawer_goes_on_the_shift() {
    let (_folder, state) = shop();
    let rahim = supplier(&state, "Rahim Traders", 0);
    let salt = product(&state, "Salt", 2_000, false, None, None);
    purchasing_commands::grn_save(
        &state,
        &delivery(&rahim, 20_000, vec![line(&salt, 10, 2_000)]),
    )
    .expect("received");
    let pay = SupplierPaymentInput {
        supplier_id: rahim.clone(),
        amount: Money(5_000),
        method_id: cash_method(&state),
        reference: None,
        from_drawer: true,
    };
    assert_eq!(
        code_of(purchasing_commands::supplier_payment(
            &state,
            devices(),
            &pay
        )),
        "shift_not_open"
    );
    assert_eq!(balance(&state, &rahim), Money(20_000), "nothing saved");
    open_shift(&state, 10_000);
    let done = purchasing_commands::supplier_payment(&state, devices(), &pay).expect("paid");
    assert_eq!(done.balance, Money(15_000));
    assert!(done.drawer.is_some(), "the drawer opens");
    let expected = state
        .db
        .read(|conn| {
            let shift = app::workflows::till::till_shift(conn)?.expect("shift");
            Ok(shifts::expected_cash(conn, &shift.id)?)
        })
        .expect("cash");
    assert_eq!(expected, Money(5_000), "float 100.00 − 50.00 paid out");
    let (_, _, details) = last_audit(&state, "supplier.payment");
    assert!(details.contains("\"from_drawer\":true"), "{details}");
}

#[test]
fn pur_05_pur_06_aging_and_statement_with_opening_and_closing() {
    let (_folder, state) = shop();
    let rahim = supplier(&state, "Rahim Traders", 30);
    let salt = product(&state, "Salt", 2_000, false, None, None);
    purchasing_commands::grn_save(
        &state,
        &delivery(&rahim, 20_000, vec![line(&salt, 10, 2_000)]),
    )
    .expect("received");
    let pay = SupplierPaymentInput {
        supplier_id: rahim.clone(),
        amount: Money(8_000),
        method_id: cash_method(&state),
        reference: Some("Cheque 12".into()),
        from_drawer: false,
    };
    purchasing_commands::supplier_payment(&state, devices(), &pay).expect("paid");

    let aging = purchasing_commands::payables_aging(&state).expect("aging");
    assert_eq!(aging.rows.len(), 1);
    assert_eq!(aging.rows[0].current, Money(12_000), "not due for 30 days");
    assert_eq!(aging.rows[0].balance, Money(12_000));

    let today = aging.today;
    let statement =
        purchasing_commands::supplier_statement(&state, &rahim, today, today).expect("statement");
    assert_eq!(statement.opening, Money::zero());
    assert_eq!(statement.closing, Money(12_000));
    let kinds: Vec<LedgerKind> = statement.lines.iter().map(|line| line.kind).collect();
    assert_eq!(kinds, vec![LedgerKind::Invoice, LedgerKind::Payment]);
    let folder = tempfile::tempdir().expect("folder");
    let file = folder.path().join("statement.xlsx");
    let written = purchasing_commands::supplier_statement_export(
        &state,
        &rahim,
        today,
        today,
        file.to_str().expect("path"),
    )
    .expect("exported");
    assert_eq!(written, 2);
    assert!(io::read_file(&file, None).expect("read back").rows.len() >= 4);
}

#[test]
fn inv_09_a_product_without_stock_cant_be_received() {
    let (_folder, state) = shop();
    let rahim = supplier(&state, "Rahim Traders", 0);
    let mut input = product_input("Delivery fee", 1_000, false, None, None);
    input.track_stock = false;
    let service = app::commands::catalog::product_save(&state, input).expect("saved");
    let refused = purchasing_commands::grn_save(
        &state,
        &delivery(&rahim, 1_000, vec![line(&service, 1, 1_000)]),
    );
    assert_eq!(code_of(refused), "product_keeps_no_stock");
}

#[test]
fn pur_01_a_cashier_cant_buy_or_pay() {
    let (_folder, state) = shop();
    let rahim = supplier(&state, "Rahim Traders", 0);
    app::session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");
    let refused = purchasing_commands::supplier_get(&state, &rahim);
    assert!(
        matches!(
            refused,
            Err(AppError::PermissionDenied {
                permission: Permission::PurchasingManage
            })
        ),
        "{refused:?}"
    );
    let pay = SupplierPaymentInput {
        supplier_id: rahim,
        amount: Money(1_000),
        method_id: cash_method(&state),
        reference: None,
        from_drawer: false,
    };
    let refused = purchasing_commands::supplier_payment(&state, devices(), &pay);
    assert!(
        matches!(
            refused,
            Err(AppError::PermissionDenied {
                permission: Permission::PurchasingPay
            })
        ),
        "{refused:?}"
    );
}

#[test]
fn pur_01_suppliers_are_saved_changed_and_audited() {
    let (_folder, state) = shop();
    let rahim = supplier(&state, "Rahim Traders", 15);
    let save = SupplierSave {
        id: Some(rahim.clone()),
        input: SupplierInput {
            name: "Rahim Traders Ltd".into(),
            contact_person: Some("Rahim".into()),
            payment_terms_days: 45,
            ..SupplierInput::default()
        },
    };
    purchasing_commands::supplier_save(&state, &save).expect("changed");
    let page = purchasing_commands::supplier_get(&state, &rahim).expect("page");
    assert_eq!(page.supplier.name, "Rahim Traders Ltd");
    assert_eq!(page.supplier.payment_terms_days, 45);
    assert!(!page.payment_methods.is_empty());
    assert_eq!(audit_count(&state, "supplier.change"), 2);
    let empty = SupplierSave {
        id: None,
        input: SupplierInput::default(),
    };
    assert_eq!(
        code_of(purchasing_commands::supplier_save(&state, &empty)),
        "supplier_name_missing"
    );
}
