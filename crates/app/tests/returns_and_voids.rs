//! Returns, exchanges and voids (C-M9, Req - Returns and Voids RET-01 to RET-07):
//! returns with and without a receipt, exact refunds, write-offs, the customer's debt,
//! store credit and points, exchanges, voids, and the return and void receipts.

mod common;

use app::commands::auth as auth_commands;
use app::commands::checkout as till;
use app::commands::customers as customer_commands;
use app::commands::returns as return_commands;
use app::commands::settings as settings_commands;
use app::commands::shifts as shift_commands;
use app::session::{self, ApprovalResult};
use app::state::AppState;
use app::workflows::checkout::MethodKind;
use app::workflows::complete_sale::{PaymentEntry, SaleDone};
use app::workflows::customer_records::CustomerSave;
use app::workflows::receipts::Printable;
use app::workflows::returns::{ReturnInput, ReturnLineInput, ReturnSale};
use app::workflows::settings_change::SettingsChange;
use common::*;
use customers::CustomerDetails;
use pos_core::{AppError, Id, Money, Permission, Qty};
use pricing::ManualDiscount;
use sales::PaymentKind;
use settings::{CreditSettings, LoyaltySettings};

/// A shop selling "Item" at `price_minor` with plenty of stock. The admin is signed in
/// with the till's shift open and a float of 1,000.00.
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
    stock_in(&state, &item, Qty(100_000));
    open_shift(&state, 100_000);
    (folder, state, item)
}

/// Sells `count` items, paid with these payments.
fn sell(state: &AppState, count: i64, payments: &[PaymentEntry]) -> SaleDone {
    till::checkout_scan(state, &format!("{count}*ITEM-1")).expect("scan");
    till::checkout_complete(state, devices(), payments).expect("sold")
}

fn method_of(state: &AppState, kind: PaymentKind) -> Id {
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

/// A return of `qty` thousandths of the sale's first line, back on the shelf.
fn input(sale: &ReturnSale, qty: i64, refund_to: &Id) -> ReturnInput {
    ReturnInput {
        sale_id: Some(sale.sale_id.clone()),
        lines: vec![ReturnLineInput {
            sale_line_id: Some(sale.lines[0].sale_line_id.clone()),
            product_id: None,
            qty: Qty(qty),
            restock: true,
            unit_price: None,
        }],
        reason: Some("Wrong size".into()),
        refund_method_id: refund_to.clone(),
        reference: None,
    }
}

fn lookup(state: &AppState, invoice_no: &str) -> ReturnSale {
    return_commands::return_lookup(state, invoice_no).expect("found")
}

fn on_hand(state: &AppState, product: &Id) -> i64 {
    state
        .db
        .read(|conn| Ok(inventory::on_hand(conn, product)?))
        .expect("stock")
        .0
}

fn expected_cash(state: &AppState) -> Money {
    state
        .db
        .read(|conn| {
            let till: settings::TillSettings = settings::get(conn)?;
            let till_id = till.till_id.expect("till");
            let shift = shifts::current(conn, &till_id)?.expect("open shift");
            Ok(shifts::expected_cash(conn, &shift.id)?)
        })
        .expect("expected cash")
}

fn approve(state: &AppState, permission: Permission) -> String {
    let result = auth_commands::request_override(state, permission, "owner", ADMIN_PASSWORD.into())
        .expect("runs");
    match result {
        ApprovalResult::Approved { token } => token,
        other => panic!("expected an approval, got {other:?}"),
    }
}

fn code_of<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
    match result {
        Err(AppError::Validation { code, .. }) | Err(AppError::Conflict { code, .. }) => code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn verify_stock(state: &AppState) {
    let mismatches = state
        .db
        .read(|conn| Ok(inventory::verify(conn)?))
        .expect("verify");
    assert!(mismatches.is_empty(), "{mismatches:?}");
}

/// Signs the admin out and the cashier rina in, with her own shift (the admin's is closed).
fn as_cashier(state: &AppState) {
    close_shift(state);
    session::logout(state).expect("out");
    sign_in(state, "rina", "1234");
    open_shift(state, 0);
}

/// Closes the signed-in user's shift with a count of 1,000.00.
fn close_shift(state: &AppState) {
    let counts = [app::workflows::shifts::CountLine {
        denomination: Money(100),
        count: 1_000,
    }];
    shift_commands::shift_close(state, devices(), &counts, None).expect("closed");
}

fn customer(state: &AppState, limit_minor: i64) -> Id {
    let mut details = CustomerDetails::quick("01711123456", "Asha");
    details.credit_limit = Money(limit_minor);
    let input = CustomerSave { id: None, details };
    customer_commands::customer_save(state, &input).expect("customer")
}

fn customer_balances(state: &AppState, id: &Id) -> (i64, Money) {
    let found = state
        .db
        .read(|conn| Ok(customers::get(conn, id)?))
        .expect("customer");
    (found.points, found.credit)
}

fn switch_on_credit_and_loyalty(state: &AppState) {
    settings_commands::settings_save(
        state,
        &SettingsChange::Credit(CreditSettings {
            enabled: true,
            default_limit_minor: Money::zero(),
        }),
    )
    .expect("credit on");
    // 1 point for each 100.00, each point worth 1.00.
    settings_commands::settings_save(
        state,
        &SettingsChange::Loyalty(LoyaltySettings {
            enabled: true,
            spend_unit_minor: Money(10_000),
            points_per_unit: 1,
            value_per_point_minor: Money(100),
            min_redeem: 0,
        }),
    )
    .expect("loyalty on");
}

#[test]
fn ret_01_return_with_a_receipt_refunds_restocks_and_numbers_the_return() {
    let (_folder, state, item) = shop(10_000);
    let cash = cash_method(&state);
    let sale = sell(&state, 3, &[paying(&cash, 30_000)]);
    let stock_after_sale = on_hand(&state, &item);
    let drawer_after_sale = expected_cash(&state);

    let found = lookup(&state, &sale.invoice_no);
    assert_eq!(found.lines[0].returnable, Qty(3_000));
    assert_eq!(found.refund_method_id, cash, "back the way it was paid");
    assert_eq!(found.void_blocked, None);

    let done =
        return_commands::return_complete(&state, devices(), &input(&found, 1_000, &cash), None)
            .expect("returned");
    assert_eq!(done.return_no, "T1-R000001");
    assert_eq!(done.total, Money(10_000));
    assert_eq!(done.cash_back, Money(10_000));
    assert_eq!(done.refunds.len(), 1);
    assert_eq!(done.refunds[0].kind, MethodKind::Cash);
    assert_eq!(on_hand(&state, &item), stock_after_sale + 1_000);
    assert_eq!(
        expected_cash(&state),
        drawer_after_sale.checked_sub(Money(10_000)).expect("money")
    );
    let receipt = done.receipt.expect("printed");
    assert!(receipt.printed, "{receipt:?}");
    assert_eq!(
        receipt.printable,
        Some(Printable::Return {
            return_id: done.return_id.clone()
        })
    );
    let printed = devices().preview.take_jobs();
    let last = printed.last().expect("the return receipt");
    assert!(last.drawer_opened, "a cash refund opens the drawer");
    check_golden("return_receipt_80mm", &last.pictures[0]);
    let (user, _, details) = last_audit(&state, "return.complete");
    assert_eq!(user, Some(user_id(&state, "owner").to_string()));
    assert!(details.contains("T1-R000001"), "{details}");

    // Sold × 3 with 1 returned: returning 3 is refused, at most 2.
    let found = lookup(&state, &sale.invoice_no);
    assert_eq!(found.lines[0].returned, Qty(1_000));
    assert_eq!(found.lines[0].returnable, Qty(2_000));
    assert_eq!(found.earlier_returns, vec!["T1-R000001".to_string()]);
    let too_many =
        return_commands::return_complete(&state, devices(), &input(&found, 3_000, &cash), None);
    match too_many {
        Err(AppError::Validation { code, params, .. }) => {
            assert_eq!(code, "return_qty_too_large");
            assert_eq!(params.get("max_milli").map(String::as_str), Some("2000"));
        }
        other => panic!("expected max 2, got {other:?}"),
    }
    verify_stock(&state);
}

#[test]
fn ret_01_a_return_needs_a_reason_and_saves_nothing_without_one() {
    let (_folder, state, item) = shop(10_000);
    let cash = cash_method(&state);
    let sale = sell(&state, 1, &[paying(&cash, 10_000)]);
    let stock = on_hand(&state, &item);
    let mut no_reason = input(&lookup(&state, &sale.invoice_no), 1_000, &cash);
    no_reason.reason = Some("  ".into());
    assert_eq!(
        code_of(return_commands::return_complete(
            &state,
            devices(),
            &no_reason,
            None
        )),
        "return_reason_missing"
    );
    assert_eq!(on_hand(&state, &item), stock);
    assert_eq!(audit_count(&state, "return.complete"), 0);
}

#[test]
fn ret_01_a_cashier_needs_an_admin_and_the_audit_names_both() {
    let (_folder, state, _item) = shop(10_000);
    as_cashier(&state);
    let cash = cash_method(&state);
    let sale = sell(&state, 1, &[paying(&cash, 10_000)]);
    let found = lookup(&state, &sale.invoice_no);
    let wanted = input(&found, 1_000, &cash);
    let refused = return_commands::return_complete(&state, devices(), &wanted, None);
    assert!(
        matches!(
            &refused,
            Err(AppError::OverrideRequired {
                permission: Permission::ReturnWithReceipt
            })
        ),
        "{refused:?}"
    );
    let token = approve(&state, Permission::ReturnWithReceipt);
    return_commands::return_complete(&state, devices(), &wanted, Some(&token)).expect("approved");
    let (user, approved_by, _) = last_audit(&state, "return.complete");
    assert_eq!(user, Some(user_id(&state, "rina").to_string()));
    assert_eq!(approved_by, Some(user_id(&state, "owner").to_string()));
}

#[test]
fn ret_02_partial_returns_refund_the_stored_amounts_exactly() {
    // 3 × 40.00 with 20.00 off the sale: the line is 100.00 (tax 13.04 inside).
    let (_folder, state, _item) = shop(4_000);
    let card = app::commands::payment_methods::payment_method_save(
        &state,
        &app::workflows::payment_methods::PaymentMethodSave {
            id: None,
            name: "Card".into(),
            kind: MethodKind::Card,
            requires_reference: false,
            active: true,
            sort: 2,
        },
    )
    .expect("card");
    till::checkout_scan(&state, "3*ITEM-1").expect("scan");
    till::checkout_set_invoice_discount(&state, Some(ManualDiscount::Amount(Money(2_000))), None)
        .expect("discount");
    let sale = till::checkout_complete(&state, devices(), &[paying(&card, 10_000)]).expect("sold");
    let stored = state
        .db
        .read(|conn| Ok(sales::get_sale(conn, &sale.sale_id)?))
        .expect("sale");

    let mut grosses = Vec::new();
    let (mut net, mut tax) = (0, 0);
    for _ in 0..3 {
        let found = lookup(&state, &sale.invoice_no);
        let wanted = input(&found, 1_000, &card);
        let preview = return_commands::return_preview(&state, &wanted).expect("preview");
        let done =
            return_commands::return_complete(&state, devices(), &wanted, None).expect("returned");
        assert_eq!(
            done.total, preview.total,
            "the preview matches what is saved"
        );
        grosses.push(preview.gross.0);
        net += preview.net.0;
        tax += preview.tax.0;
    }
    assert_eq!(grosses, vec![3_333, 3_333, 3_334], "E14");
    assert_eq!(net, stored.totals.net.0);
    assert_eq!(tax, stored.totals.tax.0);
    assert_eq!(lookup(&state, &sale.invoice_no).lines[0].returnable, Qty(0));
}

#[test]
fn ret_01_damaged_goods_come_back_and_are_written_off() {
    let (_folder, state, item) = shop(10_000);
    let cash = cash_method(&state);
    let sale = sell(&state, 2, &[paying(&cash, 20_000)]);
    let stock = on_hand(&state, &item);
    let mut damaged = input(&lookup(&state, &sale.invoice_no), 1_000, &cash);
    damaged.lines[0].restock = false;
    return_commands::return_complete(&state, devices(), &damaged, None).expect("returned");
    assert_eq!(on_hand(&state, &item), stock, "back in, then written off");
    let kinds: Vec<String> = state
        .db
        .read(|conn| {
            Ok(conn
                .prepare(
                    "SELECT kind || ':' || coalesce(reason_code, '') FROM inventory_ledger
                     WHERE ref_type = 'return' ORDER BY rowid",
                )
                .expect("query")
                .query_map([], |row| row.get(0))
                .expect("rows")
                .collect::<Result<_, _>>()
                .expect("read"))
        })
        .expect("history");
    assert_eq!(
        kinds,
        vec!["return:".to_string(), "adjust:damage".to_string()]
    );
    verify_stock(&state);
}

#[test]
fn ret_03_return_without_a_receipt_uses_todays_price_and_its_permission() {
    let (_folder, state, item) = shop(10_000);
    as_cashier(&state);
    let cash = cash_method(&state);
    let stock = on_hand(&state, &item);
    let mut wanted = ReturnInput {
        sale_id: None,
        lines: vec![ReturnLineInput {
            sale_line_id: None,
            product_id: Some(item.clone()),
            qty: Qty(2_000),
            restock: true,
            unit_price: None,
        }],
        reason: Some("No receipt".into()),
        refund_method_id: cash.clone(),
        reference: None,
    };
    let preview = return_commands::return_preview(&state, &wanted).expect("preview");
    assert_eq!(preview.gross, Money(20_000), "today's price");
    let refused = return_commands::return_complete(&state, devices(), &wanted, None);
    assert!(
        matches!(
            &refused,
            Err(AppError::OverrideRequired {
                permission: Permission::ReturnWithoutReceipt
            })
        ),
        "{refused:?}"
    );
    // A typed price needs PriceOverride: the approving admin has it.
    wanted.lines[0].unit_price = Some(Money(9_000));
    let token = approve(&state, Permission::ReturnWithoutReceipt);
    let done =
        return_commands::return_complete(&state, devices(), &wanted, Some(&token)).expect("ok");
    assert_eq!(done.total, Money(18_000));
    assert_eq!(on_hand(&state, &item), stock + 2_000);
    let saved = state
        .db
        .read(|conn| Ok(sales::get_return(conn, &done.return_id)?))
        .expect("return");
    assert_eq!(saved.sale_id, None);
}

#[test]
fn ret_03_a_typed_price_without_price_override_is_refused() {
    let (_folder, state, item) = shop(10_000);
    // A role that may return without a receipt, but not type prices.
    let role = app::commands::users::role_save(
        &state,
        app::workflows::users::RoleInput {
            id: None,
            name: "Returns desk".into(),
            permissions: vec![
                Permission::CheckoutSell,
                Permission::ShiftOpenCloseOwn,
                Permission::ReturnWithoutReceipt,
            ],
            max_discount: pos_core::Bps(0),
        },
    )
    .expect("role");
    add_user(&state, "desk", role.as_str(), None, Some("4321"));
    close_shift(&state);
    session::logout(&state).expect("out");
    sign_in(&state, "desk", "4321");
    open_shift(&state, 0);
    let wanted = ReturnInput {
        sale_id: None,
        lines: vec![ReturnLineInput {
            sale_line_id: None,
            product_id: Some(item),
            qty: Qty(1_000),
            restock: true,
            unit_price: Some(Money(5_000)),
        }],
        reason: Some("No receipt".into()),
        refund_method_id: cash_method(&state),
        reference: None,
    };
    let refused = return_commands::return_complete(&state, devices(), &wanted, None);
    assert!(
        matches!(
            &refused,
            Err(AppError::PermissionDenied {
                permission: Permission::PriceOverride
            })
        ),
        "{refused:?}"
    );
}

#[test]
fn ret_04_void_reverses_stock_shift_and_keeps_the_number() {
    let (_folder, state, item) = shop(10_000);
    let cash = cash_method(&state);
    let stock = on_hand(&state, &item);
    let drawer = expected_cash(&state);
    let sale = sell(&state, 2, &[paying(&cash, 50_000)]);
    assert_eq!(
        code_of(return_commands::sale_void(
            &state,
            devices(),
            &sale.sale_id,
            " ",
            None
        )),
        "void_reason_missing"
    );
    devices().preview.take_jobs();

    let done = return_commands::sale_void(&state, devices(), &sale.sale_id, "Rang up twice", None)
        .expect("voided");
    assert_eq!(done.invoice_no, sale.invoice_no);
    assert_eq!(done.cash_back, Money(20_000));
    assert_eq!(on_hand(&state, &item), stock);
    assert_eq!(expected_cash(&state), drawer);
    let receipt = done.receipt.expect("printed");
    assert!(receipt.printed);
    let printed = devices().preview.take_jobs();
    assert!(printed.last().expect("void slip").drawer_opened);
    check_golden(
        "void_receipt_80mm",
        &printed.last().expect("slip").pictures[0],
    );
    let found = lookup(&state, &sale.invoice_no);
    assert!(found.voided);
    assert_eq!(found.void_blocked.as_deref(), Some("sale_already_voided"));
    assert_eq!(
        code_of(return_commands::sale_void(
            &state,
            devices(),
            &sale.sale_id,
            "Again",
            None
        )),
        "sale_already_voided"
    );
    let wanted = input(&found, 1_000, &cash);
    assert_eq!(
        code_of(return_commands::return_complete(
            &state,
            devices(),
            &wanted,
            None
        )),
        "sale_voided"
    );
    let (_, _, details) = last_audit(&state, "sale.void");
    assert!(details.contains("Rang up twice"), "{details}");
    verify_stock(&state);
}

#[test]
fn ret_04_after_the_shift_closes_or_a_return_a_void_is_refused() {
    let (_folder, state, _item) = shop(10_000);
    let cash = cash_method(&state);
    let returned = sell(&state, 2, &[paying(&cash, 20_000)]);
    let found = lookup(&state, &returned.invoice_no);
    return_commands::return_complete(&state, devices(), &input(&found, 1_000, &cash), None)
        .expect("returned");
    assert_eq!(
        lookup(&state, &returned.invoice_no).void_blocked.as_deref(),
        Some("sale_has_returns")
    );
    assert_eq!(
        code_of(return_commands::sale_void(
            &state,
            devices(),
            &returned.sale_id,
            "No",
            None
        )),
        "sale_has_returns"
    );

    let earlier = sell(&state, 1, &[paying(&cash, 10_000)]);
    let counts = [app::workflows::shifts::CountLine {
        denomination: Money(100),
        count: 1_100,
    }];
    shift_commands::shift_close(&state, devices(), &counts, None).expect("closed");
    open_shift(&state, 0);
    assert_eq!(
        lookup(&state, &earlier.invoice_no).void_blocked.as_deref(),
        Some("sale_shift_closed")
    );
    assert_eq!(
        code_of(return_commands::sale_void(
            &state,
            devices(),
            &earlier.sale_id,
            "Late",
            None
        )),
        "sale_shift_closed"
    );
}

#[test]
fn ret_04_a_cashier_voids_only_with_an_admin() {
    let (_folder, state, _item) = shop(10_000);
    as_cashier(&state);
    let cash = cash_method(&state);
    let sale = sell(&state, 1, &[paying(&cash, 10_000)]);
    let refused = return_commands::sale_void(&state, devices(), &sale.sale_id, "Mistake", None);
    assert!(
        matches!(
            &refused,
            Err(AppError::OverrideRequired {
                permission: Permission::SaleVoid
            })
        ),
        "{refused:?}"
    );
    let token = approve(&state, Permission::SaleVoid);
    return_commands::sale_void(&state, devices(), &sale.sale_id, "Mistake", Some(&token))
        .expect("ok");
    let (user, approved_by, _) = last_audit(&state, "sale.void");
    assert_eq!(user, Some(user_id(&state, "rina").to_string()));
    assert_eq!(approved_by, Some(user_id(&state, "owner").to_string()));
}

#[test]
fn ret_07_void_reverses_points_and_credit() {
    let (_folder, state, _item) = shop(10_000);
    switch_on_credit_and_loyalty(&state);
    let asha = customer(&state, 100_000);
    till::checkout_set_customer(&state, Some(&asha)).expect("customer");
    let credit = method_of(&state, PaymentKind::Credit);
    let sale = sell(&state, 3, &[paying(&credit, 30_000)]);
    assert_eq!(customer_balances(&state, &asha), (0, Money(30_000)));
    till::checkout_set_customer(&state, Some(&asha)).expect("customer");
    let cash = cash_method(&state);
    sell(&state, 2, &[paying(&cash, 20_000)]);
    assert_eq!(customer_balances(&state, &asha).0, 2, "2 points on 200.00");

    return_commands::sale_void(&state, devices(), &sale.sale_id, "Wrong customer", None)
        .expect("voided");
    assert_eq!(customer_balances(&state, &asha), (2, Money::zero()));
}

#[test]
fn ret_07_a_credit_sale_return_takes_the_debt_off_first_then_pays_the_rest() {
    let (_folder, state, _item) = shop(10_000);
    switch_on_credit_and_loyalty(&state);
    let asha = customer(&state, 100_000);
    let credit = method_of(&state, PaymentKind::Credit);
    let cash = cash_method(&state);
    // 300.00: 100.00 on credit, 200.00 in cash. 2 points earned on the cash part.
    till::checkout_set_customer(&state, Some(&asha)).expect("customer");
    let sale = sell(&state, 3, &[paying(&credit, 10_000), paying(&cash, 20_000)]);
    assert_eq!(customer_balances(&state, &asha), (2, Money(10_000)));

    let found = lookup(&state, &sale.invoice_no);
    assert_eq!(
        found.customer.as_ref().map(|each| each.id.clone()),
        Some(asha.clone())
    );
    let wanted = input(&found, 2_000, &cash);
    let preview = return_commands::return_preview(&state, &wanted).expect("preview");
    let parts: Vec<(MethodKind, i64)> = preview
        .refunds
        .iter()
        .map(|refund| (refund.kind, refund.amount.0))
        .collect();
    assert_eq!(
        parts,
        vec![(MethodKind::Credit, 10_000), (MethodKind::Cash, 10_000)]
    );
    // 2 of 3 returned: the customer keeps floor(2 × 1/3) = 0 of the 2 points.
    assert_eq!(preview.points_taken_back, 2);
    let done = return_commands::return_complete(&state, devices(), &wanted, None).expect("ok");
    assert_eq!(done.cash_back, Money(10_000));
    assert_eq!(customer_balances(&state, &asha), (0, Money::zero()));

    // The last item: no debt left on this sale, so all of it by the chosen method.
    let found = lookup(&state, &sale.invoice_no);
    let store_credit = input(&found, 1_000, &credit);
    let done =
        return_commands::return_complete(&state, devices(), &store_credit, None).expect("ok");
    assert_eq!(done.refunds[0].kind, MethodKind::Credit);
    assert_eq!(
        customer_balances(&state, &asha),
        (0, Money(-10_000)),
        "no debt left, so the last item is store credit"
    );
}

#[test]
fn ret_07_points_paid_go_back_as_points() {
    let (_folder, state, _item) = shop(10_000);
    switch_on_credit_and_loyalty(&state);
    let asha = customer(&state, 0);
    customer_commands::points_adjust(&state, &asha, 50, "Welcome").expect("points");
    let points = method_of(&state, PaymentKind::Points);
    let cash = cash_method(&state);
    till::checkout_set_customer(&state, Some(&asha)).expect("customer");
    let sale = sell(&state, 1, &[paying(&points, 3_000), paying(&cash, 7_000)]);
    assert_eq!(
        customer_balances(&state, &asha).0,
        20,
        "50 − 30 paid, 0 earned on 70.00"
    );
    let found = lookup(&state, &sale.invoice_no);
    let done =
        return_commands::return_complete(&state, devices(), &input(&found, 1_000, &cash), None)
            .expect("returned");
    let parts: Vec<(MethodKind, i64)> = done
        .refunds
        .iter()
        .map(|each| (each.kind, each.amount.0))
        .collect();
    assert_eq!(
        parts,
        vec![(MethodKind::Points, 3_000), (MethodKind::Cash, 7_000)]
    );
    assert_eq!(customer_balances(&state, &asha).0, 50);
}

#[test]
fn ret_01_store_credit_needs_a_customer_and_points_are_never_chosen() {
    let (_folder, state, _item) = shop(10_000);
    switch_on_credit_and_loyalty(&state);
    let cash = cash_method(&state);
    let sale = sell(&state, 1, &[paying(&cash, 10_000)]);
    let found = lookup(&state, &sale.invoice_no);
    let credit = method_of(&state, PaymentKind::Credit);
    assert_eq!(
        code_of(return_commands::return_preview(
            &state,
            &input(&found, 1_000, &credit)
        )),
        "payment_needs_customer"
    );
    let points = method_of(&state, PaymentKind::Points);
    assert_eq!(
        code_of(return_commands::return_preview(
            &state,
            &input(&found, 1_000, &points)
        )),
        "refund_method_not_allowed"
    );
}

#[test]
fn ret_05_exchange_pays_only_the_difference() {
    let (_folder, state, item) = shop(10_000);
    let dearer = till_product(
        &state,
        "Dearer",
        "DEAR-1",
        15_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    stock_in(&state, &dearer, Qty(10_000));
    let cash = cash_method(&state);
    let sale = sell(&state, 1, &[paying(&cash, 10_000)]);
    let stock = on_hand(&state, &item);
    let drawer = expected_cash(&state);
    let found = lookup(&state, &sale.invoice_no);
    let started =
        return_commands::exchange_start(&state, devices(), &input(&found, 1_000, &cash), None)
            .expect("exchange");
    assert_eq!(started.refunds[0].kind, MethodKind::ExchangeCredit);
    assert_eq!(started.cash_back, Money::zero());
    let cart = started.cart.expect("the new cart");
    let exchange = cart.exchange.expect("credit on the cart");
    assert_eq!(exchange.credit, Money(10_000));
    assert_eq!(exchange.return_no, started.return_no);
    assert_eq!(on_hand(&state, &item), stock + 1_000);
    assert_eq!(expected_cash(&state), drawer, "no money moved yet");

    till::checkout_scan(&state, "DEAR-1").expect("scan");
    let preview = till::checkout_payment_preview(&state, &[]).expect("preview");
    assert_eq!(preview.exchange_credit, Money(10_000));
    assert_eq!(preview.remaining, Money(5_000));
    let done = till::checkout_complete(&state, devices(), &[paying(&cash, 5_000)]).expect("sold");
    assert_eq!(done.exchange_cash_back, Money::zero());
    assert_eq!(
        expected_cash(&state),
        drawer.checked_add(Money(5_000)).expect("money")
    );
    let new_sale = state
        .db
        .read(|conn| Ok(sales::get_sale(conn, &done.sale_id)?))
        .expect("sale");
    assert_eq!(new_sale.exchange_return_id, Some(started.return_id.clone()));
    assert!(
        done.cart.exchange.is_none(),
        "the next cart is an ordinary one"
    );
    verify_stock(&state);
}

#[test]
fn ret_05_exchange_for_less_gives_the_difference_back_in_cash() {
    let (_folder, state, _item) = shop(10_000);
    let cheaper = till_product(
        &state,
        "Cheaper",
        "CHEAP-1",
        6_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    stock_in(&state, &cheaper, Qty(10_000));
    let cash = cash_method(&state);
    let sale = sell(&state, 1, &[paying(&cash, 10_000)]);
    let drawer = expected_cash(&state);
    let found = lookup(&state, &sale.invoice_no);
    return_commands::exchange_start(&state, devices(), &input(&found, 1_000, &cash), None)
        .expect("exchange");
    till::checkout_scan(&state, "CHEAP-1").expect("scan");
    let preview = till::checkout_payment_preview(&state, &[]).expect("preview");
    assert_eq!(
        (
            preview.exchange_credit,
            preview.exchange_cash_back,
            preview.remaining
        ),
        (Money(6_000), Money(4_000), Money::zero())
    );
    let done = till::checkout_complete(&state, devices(), &[]).expect("sold");
    assert_eq!(done.exchange_cash_back, Money(4_000));
    assert!(done.cash_taken, "the drawer opens for the difference");
    assert_eq!(
        expected_cash(&state),
        drawer.checked_sub(Money(4_000)).expect("money")
    );

    // Voiding the exchange sale gives its credit back in cash too.
    let voided = return_commands::sale_void(&state, devices(), &done.sale_id, "Changed mind", None)
        .expect("voided");
    assert_eq!(voided.cash_back, Money(6_000));
    assert_eq!(
        expected_cash(&state),
        drawer.checked_sub(Money(10_000)).expect("money")
    );
}

#[test]
fn ret_05_an_open_exchange_cant_be_held_cancelled_or_closed_but_can_be_cashed_out() {
    let (_folder, state, _item) = shop(10_000);
    let cash = cash_method(&state);
    let sale = sell(&state, 1, &[paying(&cash, 10_000)]);
    let drawer = expected_cash(&state);
    let found = lookup(&state, &sale.invoice_no);
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    assert_eq!(
        code_of(return_commands::exchange_start(
            &state,
            devices(),
            &input(&found, 1_000, &cash),
            None
        )),
        "cart_in_progress"
    );
    till::checkout_cancel(&state).expect("cancelled");
    return_commands::exchange_start(&state, devices(), &input(&found, 1_000, &cash), None)
        .expect("exchange");
    till::checkout_scan(&state, "ITEM-1").expect("scan");
    assert_eq!(
        code_of(till::checkout_hold(&state, None)),
        "exchange_in_progress"
    );
    assert_eq!(
        code_of(till::checkout_cancel(&state)),
        "exchange_in_progress"
    );
    let counts = [app::workflows::shifts::CountLine {
        denomination: Money(100),
        count: 1_000,
    }];
    let line = till::checkout_state(&state)
        .expect("till")
        .cart
        .expect("cart")
        .lines[0]
        .line_id
        .clone();
    till::checkout_remove_line(&state, &line).expect("removed");
    assert_eq!(
        code_of(shift_commands::shift_close(
            &state,
            devices(),
            &counts,
            None
        )),
        "exchange_in_progress"
    );

    let out = return_commands::exchange_cash_out(&state, devices()).expect("cashed out");
    assert_eq!(out.cash_back, Money(10_000));
    assert!(out.cart.exchange.is_none());
    assert_eq!(
        expected_cash(&state),
        drawer.checked_sub(Money(10_000)).expect("money")
    );
    assert_eq!(audit_count(&state, "exchange.cash_out"), 1);
    let again = return_commands::exchange_cash_out(&state, devices()).expect("nothing left");
    assert_eq!(again.cash_back, Money::zero());
    assert_eq!(audit_count(&state, "exchange.cash_out"), 1);
}

#[test]
fn ret_05_force_close_gives_an_open_exchange_back_in_cash() {
    let (_folder, state, _item) = shop(10_000);
    let cash = cash_method(&state);
    let sale = sell(&state, 1, &[paying(&cash, 10_000)]);
    let found = lookup(&state, &sale.invoice_no);
    return_commands::exchange_start(&state, devices(), &input(&found, 1_000, &cash), None)
        .expect("exchange");
    let closed =
        shift_commands::shift_force_close(&state, devices(), &[], None, None).expect("closed");
    assert_eq!(
        closed.expected_cash,
        Money(100_000),
        "float + 100.00 sold − 100.00 back"
    );
    assert_eq!(audit_count(&state, "exchange.cash_out"), 1);
}
