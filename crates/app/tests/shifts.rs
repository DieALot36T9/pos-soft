//! Shifts and cash (Req - Shifts and Cash): opening with a float or a count, one
//! shift per till, cash in and out with admin approval, the no-sale drawer, closing
//! with the count and its difference, and force close.

mod common;

use app::commands::auth as auth_commands;
use app::commands::checkout as till;
use app::commands::shifts as shift_commands;
use app::session::{self, ApprovalResult};
use app::state::AppState;
use app::workflows::shifts::{CashMoveInput, CashMoveKind, CountLine};
use common::*;
use pos_core::{AppError, Money, Permission, Qty};

fn shop() -> (tempfile::TempDir, AppState) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let item = till_product(
        &state,
        "Item",
        "ITEM-1",
        10_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    stock_in(&state, &item, Qty(1_000_000));
    add_cashier(&state);
    (folder, state)
}

fn count(denomination_minor: i64, count: u32) -> CountLine {
    CountLine {
        denomination: Money(denomination_minor),
        count,
    }
}

fn code_of<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
    match result {
        Err(AppError::Validation { code, .. }) | Err(AppError::Conflict { code, .. }) => code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn as_cashier(state: &AppState) {
    session::logout(state).expect("out");
    sign_in(state, "rina", "1234");
}

fn approval(state: &AppState, permission: Permission) -> String {
    match auth_commands::request_override(state, permission, "owner", ADMIN_PASSWORD.into())
        .expect("asked")
    {
        ApprovalResult::Approved { token } => token,
        other => panic!("{other:?}"),
    }
}

fn move_cash(kind: CashMoveKind, amount_minor: i64, reason: &str) -> CashMoveInput {
    CashMoveInput {
        kind,
        amount: Money(amount_minor),
        reason: reason.into(),
    }
}

#[test]
fn shf_01_open_with_a_count_and_a_business_date() {
    let (_folder, state) = shop();
    as_cashier(&state);
    let total = shift_commands::shift_count_total(&state, &[count(50_000, 2), count(10_000, 1)])
        .expect("total");
    assert_eq!(total, Money(110_000));
    let shift = shift_commands::shift_open(&state, Money(0), &[count(50_000, 2), count(10_000, 1)])
        .expect("opened");
    assert_eq!(shift.opening_float, Money(110_000));
    assert_eq!(shift.business_date.0.to_string(), "2026-10-05");
    assert!(shift.mine);
    assert_eq!(shift.cashier_name, "rina name");
    assert_eq!(audit_count(&state, "shift.open"), 1);
    let current = shift_commands::shift_current(&state).expect("current");
    assert_eq!(current.shift.expect("open").shift_id, shift.shift_id);
    assert!(current.blind_close);
    assert_eq!(
        current.expected_cash, None,
        "count first hides the expected cash"
    );
    assert_eq!(current.denominations.first(), Some(&Money(100_000)));
}

#[test]
fn shf_02_one_open_shift_per_till() {
    let (_folder, state) = shop();
    open_shift(&state, 0);
    assert_eq!(
        code_of(shift_commands::shift_open(&state, Money(0), &[])),
        "shift_already_open"
    );
    as_cashier(&state);
    assert_eq!(
        code_of(shift_commands::shift_open(&state, Money(0), &[])),
        "shift_already_open"
    );
    assert_eq!(
        code_of(shift_commands::shift_close(&state, &[], None)),
        "shift_not_yours"
    );
}

#[test]
fn shf_03_cash_in_and_out_need_an_admin_and_a_reason() {
    let (_folder, state) = shop();
    as_cashier(&state);
    open_shift(&state, 100_000);
    let paid_out = move_cash(CashMoveKind::CashOut, 20_000, "Delivery man");
    let result = shift_commands::shift_cash_move(&state, &paid_out, None);
    assert!(
        matches!(result, Err(AppError::OverrideRequired { .. })),
        "{result:?}"
    );
    let token = approval(&state, Permission::ShiftCashInOut);
    shift_commands::shift_cash_move(&state, &paid_out, Some(&token)).expect("paid out");
    let (user, approved_by, details) = last_audit(&state, "shift.cash_move");
    assert_eq!(user, Some(user_id(&state, "rina").to_string()));
    assert_eq!(approved_by, Some(user_id(&state, "owner").to_string()));
    assert!(
        details.contains("Delivery man") && details.contains("-20000"),
        "{details}"
    );

    session::logout(&state).expect("out");
    sign_in(&state, "owner", ADMIN_PASSWORD);
    let no_reason = move_cash(CashMoveKind::CashIn, 5_000, "  ");
    assert_eq!(
        code_of(shift_commands::shift_cash_move(&state, &no_reason, None)),
        "reason_missing"
    );
    shift_commands::shift_cash_move(
        &state,
        &move_cash(CashMoveKind::CashIn, 5_000, "Change"),
        None,
    )
    .expect("cash in");
    shift_commands::shift_cash_move(
        &state,
        &move_cash(CashMoveKind::CashDrop, 30_000, "Safe"),
        None,
    )
    .expect("drop");
    let closed = shift_commands::shift_force_close(&state, &[], None, None).expect("closed");
    // 1,000.00 float − 200.00 paid out + 50.00 in − 300.00 to the safe.
    assert_eq!(closed.expected_cash, Money(55_000));
}

#[test]
fn shf_04_the_no_sale_drawer_is_audited_with_its_reason() {
    let (_folder, state) = shop();
    as_cashier(&state);
    open_shift(&state, 0);
    let result = shift_commands::shift_no_sale(&state, "Change for a note", None);
    assert!(
        matches!(result, Err(AppError::OverrideRequired { .. })),
        "{result:?}"
    );
    let token = approval(&state, Permission::ShiftNoSaleDrawer);
    shift_commands::shift_no_sale(&state, "Change for a note", Some(&token)).expect("opened");
    let (_, approved_by, details) = last_audit(&state, "drawer.no_sale");
    assert!(approved_by.is_some());
    assert!(details.contains("Change for a note"));
}

#[test]
fn shf_06_close_with_the_count_and_its_shortage() {
    let (_folder, state) = shop();
    as_cashier(&state);
    open_shift(&state, 50_000);
    for _ in 0..3 {
        till::checkout_scan(&state, "ITEM-1").expect("scan");
        till::checkout_complete(&state, &[cash(&state, 10_000)]).expect("sold");
    }
    till::checkout_scan(&state, "ITEM-1").expect("an open cart");
    assert_eq!(
        code_of(shift_commands::shift_close(
            &state,
            &[count(10_000, 8)],
            None
        )),
        "shift_has_open_carts"
    );
    till::checkout_hold(&state, None).expect("held");
    assert_eq!(
        code_of(shift_commands::shift_close(&state, &[], None)),
        "count_missing"
    );
    // Expected 800.00 (500.00 float + 300.00 sales); counted 750.00.
    let closed =
        shift_commands::shift_close(&state, &[count(50_000, 1), count(25_000, 1)], Some("short"))
            .expect("closed");
    assert_eq!(closed.expected_cash, Money(80_000));
    assert_eq!(closed.counted_cash, Some(Money(75_000)));
    assert_eq!(closed.difference, Some(Money(-5_000)));
    assert_eq!(audit_count(&state, "shift.close"), 1);
    assert!(
        shift_commands::shift_current(&state)
            .expect("current")
            .shift
            .is_none()
    );
    let cart_rows: i64 = state
        .db
        .read(|conn| {
            Ok(conn
                .query_row(
                    "SELECT count(*) FROM sales_carts WHERE status = 'open'",
                    [],
                    |row| row.get(0),
                )
                .expect("count"))
        })
        .expect("read");
    assert_eq!(cart_rows, 0, "the empty cart went with the shift");
}

#[test]
fn shf_07_an_admin_force_closes_and_the_cart_is_held() {
    let (_folder, state) = shop();
    as_cashier(&state);
    open_shift(&state, 0);
    till::checkout_scan(&state, "2*ITEM-1").expect("scan");
    session::logout(&state).expect("out");
    sign_in(&state, "owner", ADMIN_PASSWORD);
    let closed = shift_commands::shift_force_close(&state, &[], Some("went home"), None)
        .expect("force closed");
    assert!(closed.forced);
    assert_eq!(closed.counted_cash, None);
    assert_eq!(audit_count(&state, "shift.force_close"), 1);

    open_shift(&state, 0);
    let held = till::checkout_held_list(&state).expect("held");
    assert_eq!(held.len(), 1);
    let resumed = till::checkout_resume(&state, &held[0].cart_id).expect("resumed");
    assert_eq!(resumed.cart.totals.total, Money(20_000));
}

#[test]
fn shf_06_without_count_first_the_expected_cash_shows() {
    let (_folder, state) = shop();
    let mut checkout = <settings::CheckoutSettings as settings::Setting>::default_value();
    checkout.blind_close = false;
    app::commands::settings::settings_save(
        &state,
        &app::workflows::settings_change::SettingsChange::Checkout(checkout),
    )
    .expect("saved");
    open_shift(&state, 25_000);
    let current = shift_commands::shift_current(&state).expect("current");
    assert_eq!(current.expected_cash, Some(Money(25_000)));
}
