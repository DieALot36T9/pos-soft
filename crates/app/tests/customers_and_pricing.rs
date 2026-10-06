//! Pricing at the till and customers (C-M8): manual discounts and their limit (CHK-12,
//! PRC-07), typed prices (CHK-13), tiers (CHK-11, PRC-03), price lists (CHK-17), selling
//! on credit (PAY-04), loyalty points (PAY-05, CUS-04), debt payments (CUS-05) and the
//! customer page (CUS-03).

mod common;

use app::commands::auth as auth_commands;
use app::commands::checkout as till;
use app::commands::customers as customer_commands;
use app::commands::pricing as pricing_commands;
use app::commands::settings as settings_commands;
use app::session::{self, ApprovalResult};
use app::state::AppState;
use app::workflows::checkout::MethodKind;
use app::workflows::complete_sale::PaymentEntry;
use app::workflows::customer_records::{CustomerPaymentInput, CustomerSave};
use app::workflows::pricing_admin::{PriceListSave, TierSave};
use app::workflows::settings_change::SettingsChange;
use common::*;
use customers::CustomerDetails;
use pos_core::{AppError, Bps, Id, Money, Permission, Qty};
use pricing::{ManualDiscount, PriceListItem};
use settings::{CreditSettings, LoyaltySettings};

/// A shop selling one item at `price_minor`, with lots of stock. The admin is signed in
/// and has the till's shift open.
fn shop(price_minor: i64) -> (tempfile::TempDir, AppState, Id) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    add_cashier(&state);
    let item = till_product(
        &state,
        "Item",
        "ITEM-1",
        price_minor,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    stock_in(&state, &item, Qty(1_000_000));
    (folder, state, item)
}

/// Signs the admin out and the cashier rina in, with her own shift.
fn as_cashier(state: &AppState) {
    session::logout(state).expect("out");
    sign_in(state, "rina", "1234");
    open_shift(state, 0);
}

fn approve(state: &AppState, permission: Permission) -> String {
    let result = auth_commands::request_override(state, permission, "owner", ADMIN_PASSWORD.into())
        .expect("runs");
    match result {
        ApprovalResult::Approved { token } => token,
        other => panic!("expected an approval, got {other:?}"),
    }
}

fn first_line(state: &AppState) -> Id {
    till::checkout_state(state)
        .expect("state")
        .cart
        .expect("cart")
        .lines[0]
        .line_id
        .clone()
}

fn customer(state: &AppState, phone: &str, name: &str, tier: Option<Id>, limit: i64) -> Id {
    let mut details = CustomerDetails::quick(phone, name);
    details.tier_id = tier;
    details.credit_limit = Money(limit);
    let input = CustomerSave { id: None, details };
    customer_commands::customer_save(state, &input).expect("customer saved")
}

fn method_of(state: &AppState, kind: sales::PaymentKind) -> Id {
    state
        .db
        .read(|conn| Ok(sales::system_payment_method(conn, kind)?))
        .expect("method")
        .id
}

fn paying(method_id: &Id, amount_minor: i64) -> PaymentEntry {
    PaymentEntry {
        method_id: method_id.clone(),
        amount: Money(amount_minor),
        reference: None,
    }
}

fn needs(result: Result<impl std::fmt::Debug, AppError>, permission: Permission) {
    assert!(
        matches!(&result, Err(AppError::OverrideRequired { permission: p }) if *p == permission),
        "{result:?}"
    );
}

fn code_of<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
    match result {
        Err(AppError::Validation { code, .. }) | Err(AppError::Conflict { code, .. }) => code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn switch_on_credit(state: &AppState) {
    let change = SettingsChange::Credit(CreditSettings {
        enabled: true,
        default_limit_minor: Money::zero(),
    });
    settings_commands::settings_save(state, &change).expect("credit on");
}

/// Loyalty on: 1 point for each 100.00, each point worth 1.00.
fn switch_on_loyalty(state: &AppState) {
    let change = SettingsChange::Loyalty(LoyaltySettings {
        enabled: true,
        spend_unit_minor: Money(10_000),
        points_per_unit: 1,
        value_per_point_minor: Money(100),
        min_redeem: 0,
    });
    settings_commands::settings_save(state, &change).expect("loyalty on");
}

#[test]
fn chk_12_a_discount_above_the_cashiers_limit_needs_an_admin_and_is_audited() {
    let (_folder, state, _item) = shop(10_000);
    as_cashier(&state);
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    let line = first_line(&state);
    let ten_percent = Some(ManualDiscount::Percent(Bps(1_000)));
    needs(
        till::checkout_set_discount(&state, &line, ten_percent, None),
        Permission::DiscountApply,
    );
    let unchanged = till::checkout_state(&state)
        .expect("state")
        .cart
        .expect("cart");
    assert_eq!(
        unchanged.lines[0].discount,
        Money::zero(),
        "nothing was saved"
    );

    let token = approve(&state, Permission::DiscountApply);
    let view =
        till::checkout_set_discount(&state, &line, ten_percent, Some(&token)).expect("approved");
    assert_eq!(view.lines[0].discount, Money(1_000));
    assert_eq!(view.lines[0].manual_discount, ten_percent);
    let (user, approved_by, details) = last_audit(&state, "discount.over_limit");
    assert_eq!(user, Some(user_id(&state, "rina").to_string()));
    assert_eq!(approved_by, Some(user_id(&state, "owner").to_string()));
    assert!(details.contains("\"discount_bps\":1000"), "{details}");

    // The token was used up; taking the discount off needs no approval.
    let five = Some(ManualDiscount::Percent(Bps(1_500)));
    needs(
        till::checkout_set_discount(&state, &line, five, Some(&token)),
        Permission::DiscountApply,
    );
    let view = till::checkout_set_discount(&state, &line, None, None).expect("removed");
    assert_eq!(view.lines[0].discount, Money::zero());
}

#[test]
fn chk_12_the_invoice_discount_counts_against_the_same_limit() {
    let (_folder, state, _item) = shop(10_000);
    as_cashier(&state);
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    let amount = Some(ManualDiscount::Amount(Money(2_000)));
    needs(
        till::checkout_set_invoice_discount(&state, amount, None),
        Permission::DiscountApply,
    );
    let token = approve(&state, Permission::DiscountApply);
    let view = till::checkout_set_invoice_discount(&state, amount, Some(&token)).expect("ok");
    assert_eq!(view.invoice_discount, amount);
    assert_eq!(view.totals.discounts, Money(2_000));
}

#[test]
fn chk_12_an_admin_gives_any_discount_alone() {
    let (_folder, state, _item) = shop(10_000);
    open_shift(&state, 0);
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    let line = first_line(&state);
    let half = Some(ManualDiscount::Percent(Bps(5_000)));
    let view = till::checkout_set_discount(&state, &line, half, None).expect("admin");
    assert_eq!(view.lines[0].discount, Money(5_000));
    assert_eq!(audit_count(&state, "discount.over_limit"), 0);
}

#[test]
fn chk_13_a_typed_price_needs_permission_and_audits_old_and_new() {
    let (_folder, state, _item) = shop(10_000);
    as_cashier(&state);
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    let line = first_line(&state);
    needs(
        till::checkout_price_override(&state, &line, Some(Money(8_000)), None),
        Permission::PriceOverride,
    );
    let token = approve(&state, Permission::PriceOverride);
    let view = till::checkout_price_override(&state, &line, Some(Money(8_000)), Some(&token))
        .expect("approved");
    assert_eq!(view.lines[0].unit_price, Money(8_000));
    assert!(view.lines[0].price_overridden);
    let (_user, approved_by, details) = last_audit(&state, "price.override");
    assert_eq!(approved_by, Some(user_id(&state, "owner").to_string()));
    assert!(
        details.contains("\"old_unit_price_minor\":10000"),
        "{details}"
    );
    assert!(
        details.contains("\"new_unit_price_minor\":8000"),
        "{details}"
    );
}

#[test]
fn chk_11_a_customers_tier_gives_its_discount_at_once() {
    let (_folder, state, _item) = shop(10_000);
    open_shift(&state, 0);
    let gold = TierSave {
        id: None,
        name: "Gold".into(),
        price_list_id: None,
        discount: Bps(500),
        sort: 0,
    };
    let tier = pricing_commands::tier_save(&state, &gold).expect("tier");
    let asha = customer(&state, "01711123456", "Asha", Some(tier), 0);
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    let view = till::checkout_set_customer(&state, Some(&asha)).expect("attached");
    let attached = view.customer.expect("customer");
    assert_eq!(attached.name, "Asha");
    assert_eq!(attached.tier_name.as_deref(), Some("Gold"));
    assert_eq!(view.lines[0].discount, Money(500));
    let view = till::checkout_set_customer(&state, None).expect("taken off");
    assert!(view.customer.is_none());
    assert_eq!(view.lines[0].discount, Money::zero());

    customer_commands::customer_archive(&state, &asha, true).expect("archived");
    assert_eq!(
        code_of(till::checkout_set_customer(&state, Some(&asha))),
        "customer_archived"
    );
}

#[test]
fn chk_17_switching_the_price_list_reprices_and_needs_permission() {
    let (_folder, state, item) = shop(10_000);
    let input = PriceListSave {
        id: None,
        name: "Wholesale".into(),
        active: true,
    };
    let list = pricing_commands::price_list_save(&state, &input).expect("list");
    let price = PriceListItem {
        product_id: item.clone(),
        price: Money(9_000),
    };
    pricing_commands::price_list_set_items(&state, &list, &[price]).expect("prices");
    let rows = pricing_commands::price_list_items(&state, &list).expect("rows");
    assert_eq!(rows[0].retail_price, Money(10_000));
    assert_eq!(rows[0].price, Money(9_000));
    assert_eq!(audit_count(&state, "pricing.change"), 2);

    as_cashier(&state);
    let view = till::checkout_state(&state).expect("state");
    assert_eq!(
        view.price_lists.len(),
        1,
        "the till offers the switched-on list"
    );
    assert_eq!(view.max_discount, Bps(0), "the cashier role's limit");
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    needs(
        till::checkout_set_price_list(&state, Some(&list), None),
        Permission::PriceListSwitch,
    );
    let token = approve(&state, Permission::PriceListSwitch);
    let view = till::checkout_set_price_list(&state, Some(&list), Some(&token)).expect("ok");
    assert_eq!(view.lines[0].unit_price, Money(9_000));
    assert_eq!(view.price_list.expect("list").name, "Wholesale");
}

#[test]
fn pay_04_credit_sales_stop_at_the_limit_unless_an_admin_approves() {
    let (_folder, state, _item) = shop(10_000);
    switch_on_credit(&state);
    let asha = customer(&state, "01711123456", "Asha", None, 15_000);
    as_cashier(&state);
    let credit = method_of(&state, sales::PaymentKind::Credit);

    till::checkout_scan(&state, "ITEM-1").expect("scan");
    assert_eq!(
        code_of(till::checkout_complete(
            &state,
            devices(),
            &[paying(&credit, 10_000)]
        )),
        "payment_needs_customer"
    );
    till::checkout_set_customer(&state, Some(&asha)).expect("attached");
    let preview = till::checkout_payment_preview(&state, &[]).expect("preview");
    assert_eq!(preview.credit_available, Some(Money(15_000)));
    till::checkout_complete(&state, devices(), &[paying(&credit, 10_000)]).expect("on credit");
    let page = customer_commands::customer_get(&state, &asha).expect("page");
    assert_eq!(page.credit, Money(10_000));

    till::checkout_scan(&state, "ITEM-1").expect("scan");
    till::checkout_set_customer(&state, Some(&asha)).expect("attached");
    let over = till::checkout_complete(&state, devices(), &[paying(&credit, 10_000)]);
    needs(over, Permission::CreditLimitExceed);
    let token = approve(&state, Permission::CreditLimitExceed);
    till::checkout_complete_approved(&state, devices(), &[paying(&credit, 10_000)], Some(&token))
        .expect("approved");
    let page = customer_commands::customer_get(&state, &asha).expect("page");
    assert_eq!(page.credit, Money(20_000));
    assert_eq!(page.visits, 2);
    assert_eq!(page.total_spend, Money(20_000));
    assert_eq!(page.purchases.len(), 2);
}

#[test]
fn pay_05_cus_04_points_are_earned_on_cash_and_pay_in_whole_points() {
    let (_folder, state, _item) = shop(100_000);
    switch_on_loyalty(&state);
    let asha = customer(&state, "01711123456", "Asha", None, 0);
    open_shift(&state, 0);
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    till::checkout_set_customer(&state, Some(&asha)).expect("attached");
    let done = till::checkout_complete(&state, devices(), &[cash(&state, 100_000)]).expect("sold");
    assert_eq!(done.points_earned, 10);
    assert!(
        done.cart.customer.is_none(),
        "the next cart starts without her"
    );

    till::checkout_scan(&state, "ITEM-1").expect("scan");
    till::checkout_set_customer(&state, Some(&asha)).expect("attached");
    let offer = till::checkout_payment_preview(&state, &[])
        .expect("preview")
        .points_offer
        .expect("offer");
    assert_eq!((offer.balance, offer.max_points), (10, 10));
    let points = method_of(&state, sales::PaymentKind::Points);
    assert_eq!(
        code_of(till::checkout_complete(
            &state,
            devices(),
            &[paying(&points, 250), cash(&state, 99_750)]
        )),
        "points_whole_only"
    );
    assert_eq!(
        code_of(till::checkout_complete(
            &state,
            devices(),
            &[paying(&points, 1_100), cash(&state, 98_900)]
        )),
        "points_over_limit"
    );
    let done = till::checkout_complete(
        &state,
        devices(),
        &[paying(&points, 500), cash(&state, 99_500)],
    )
    .expect("sold");
    // 995.00 paid in cash earns 9 points; 5 were used.
    assert_eq!(done.points_earned, 9);
    let page = customer_commands::customer_get(&state, &asha).expect("page");
    assert_eq!(page.points, 14);
    assert_eq!(page.points_history.len(), 3);
}

#[test]
fn cus_04_a_points_correction_needs_a_reason_and_is_audited() {
    let (_folder, state, _item) = shop(10_000);
    let asha = customer(&state, "01711123456", "Asha", None, 0);
    assert_eq!(
        code_of(customer_commands::points_adjust(&state, &asha, 50, " ")),
        "reason_missing"
    );
    let balance =
        customer_commands::points_adjust(&state, &asha, 50, "Missed on paper card").expect("ok");
    assert_eq!(balance, 50);
    assert_eq!(audit_count(&state, "points.adjust"), 1);
    as_cashier(&state);
    assert!(matches!(
        customer_commands::points_adjust(&state, &asha, 5, "gift"),
        Err(AppError::PermissionDenied {
            permission: Permission::LoyaltyAdjust
        })
    ));
}

#[test]
fn cus_05_a_debt_payment_lowers_the_debt_and_cash_goes_in_the_drawer() {
    let (_folder, state, _item) = shop(10_000);
    switch_on_credit(&state);
    let asha = customer(&state, "01711123456", "Asha", None, 50_000);
    as_cashier(&state);
    let credit = method_of(&state, sales::PaymentKind::Credit);
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    till::checkout_set_customer(&state, Some(&asha)).expect("attached");
    till::checkout_complete(&state, devices(), &[paying(&credit, 10_000)]).expect("on credit");

    let pay = |amount| CustomerPaymentInput {
        customer_id: asha.clone(),
        amount: Money(amount),
        method_id: cash_method(&state),
        reference: None,
    };
    assert_eq!(
        code_of(customer_commands::customer_payment(&state, &pay(20_000))),
        "payment_more_than_debt"
    );
    let done = customer_commands::customer_payment(&state, &pay(4_000)).expect("paid");
    assert_eq!(done.credit, Money(6_000));
    assert!(done.cash_taken);
    let expected = state
        .db
        .read(|conn| {
            let open = app::workflows::till::till_shift(conn)?.expect("shift");
            Ok(shifts::expected_cash(conn, &open.id)?)
        })
        .expect("expected cash");
    assert_eq!(expected, Money(4_000), "a float of 0 plus the 40.00 paid");
    assert_eq!(audit_count(&state, "customer.payment"), 1);
}

#[test]
fn cus_02_a_cashier_adds_a_customer_without_tier_or_limit_but_may_not_change_one() {
    let (_folder, state, _item) = shop(10_000);
    let gold = TierSave {
        id: None,
        name: "Gold".into(),
        price_list_id: None,
        discount: Bps(500),
        sort: 0,
    };
    let tier = pricing_commands::tier_save(&state, &gold).expect("tier");
    as_cashier(&state);
    let id = customer(&state, "01711123456", "Asha", Some(tier), 99_000);
    let page = customer_commands::customer_get(&state, &id).expect("page");
    assert_eq!(page.customer.tier_id, None);
    assert_eq!(page.customer.credit_limit, Money::zero());
    assert_eq!(page.tiers.len(), 1, "Gold can be chosen in the edit form");
    assert!(
        page.payment_methods
            .iter()
            .all(|method| !matches!(method.kind, MethodKind::Credit | MethodKind::Points)),
        "a debt is never paid with credit or points"
    );
    assert_eq!(audit_count(&state, "customer.create"), 1);
    let found = customer_commands::customer_lookup(&state, "1711").expect("lookup");
    assert_eq!(found.len(), 1);

    let change = CustomerSave {
        id: Some(id),
        details: CustomerDetails::quick("01711123456", "Asha Begum"),
    };
    assert!(matches!(
        customer_commands::customer_save(&state, &change),
        Err(AppError::PermissionDenied {
            permission: Permission::CustomerManage
        })
    ));
}

#[test]
fn cus_08_the_receipt_shows_the_customers_points_and_balance_due() {
    let (_folder, state, _item) = shop(10_000);
    switch_on_credit(&state);
    switch_on_loyalty(&state);
    let asha = customer(&state, "01711123456", "Asha", None, 50_000);
    open_shift(&state, 0);
    let credit = method_of(&state, sales::PaymentKind::Credit);
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    till::checkout_set_customer(&state, Some(&asha)).expect("attached");
    let done = till::checkout_complete(
        &state,
        devices(),
        &[paying(&credit, 4_000), cash(&state, 6_000)],
    )
    .expect("sold");
    let doc = state
        .db
        .read(|conn| app::workflows::receipts::receipt_doc(conn, 360, &done.sale_id, false))
        .expect("receipt");
    let printed = format!("{doc:?}");
    assert!(printed.contains("Asha"), "{printed}");
    assert!(printed.contains("Points"), "{printed}");
    assert!(printed.contains("Balance due"), "{printed}");
    assert!(
        printed.contains("40.00"),
        "the 40.00 on credit is owed: {printed}"
    );
}
