//! Receipts after a sale, reprints, failed printouts, the cash drawer, X and Z reports
//! and the scale from settings (Req - Hardware and Printing, PAY-09, PAY-10, SHF-05,
//! SHF-06). Printouts go to the Preview printer and are compared with the approved
//! pictures in `tests/golden/` (after approval: `UPDATE_GOLDEN=1 cargo nextest run -p app`).

mod common;

use app::commands::checkout as till;
use app::commands::receipts as receipt_commands;
use app::commands::settings as settings_commands;
use app::commands::shifts as shift_commands;
use app::state::AppState;
use app::workflows::receipts::Printable;
use app::workflows::settings_change::SettingsChange;
use app::workflows::shifts::{CashMoveInput, CashMoveKind, CountLine};
use common::*;
use hardware::PreviewJob;
use pos_core::{AppError, Id, Money, Qty};
use settings::{HardwareSettings, PrinterConnection, ReceiptSettings, ScaleSettings, Setting};

/// A shop with bread (pieces) and potatoes (kg) in stock, and the owner's shift open.
fn shop_selling() -> (tempfile::TempDir, AppState) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let bread = till_product(
        &state,
        "Bread",
        "BREAD-1",
        5_000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    let potato = till_product(
        &state,
        "Potato",
        "POTATO-1",
        6_000,
        catalog::UNIT_KG,
        true,
        None,
    );
    stock_in(&state, &bread, Qty(100_000));
    stock_in(&state, &potato, Qty(100_000));
    open_shift(&state, 100_000);
    (folder, state)
}

/// Two breads and 1.25 kg of potatoes, paid with 200.00 cash.
fn sell_with_cash(state: &AppState) -> app::workflows::complete_sale::SaleDone {
    till::checkout_scan(state, "2*BREAD-1").expect("bread");
    let potato = product(state, "POTATO-1");
    till::checkout_add_weighed(
        state,
        devices(),
        &potato,
        &till::WeightInput::Manual {
            text: "1.25".into(),
        },
    )
    .expect("potato");
    till::checkout_complete(state, devices(), &[cash(state, 20_000)]).expect("sold")
}

fn product(state: &AppState, code: &str) -> Id {
    state
        .db
        .read(|conn| Ok(catalog::find_by_code(conn, code)?))
        .expect("find")
        .expect("product")
        .id
}

fn save(state: &AppState, change: SettingsChange) {
    settings_commands::settings_save(state, &change).expect("saved");
}

/// A printer that can't be reached: a file in a folder that doesn't exist.
fn broken_printer(state: &AppState) {
    let path = std::env::temp_dir()
        .join("no-such-folder-for-printing")
        .join("printer.bin");
    save(
        state,
        SettingsChange::Hardware(HardwareSettings {
            receipt_printer: Some(PrinterConnection::File {
                path: path.to_string_lossy().into_owned(),
            }),
            ..HardwareSettings::default_value()
        }),
    );
}

fn preview_printer(state: &AppState) {
    save(
        state,
        SettingsChange::Hardware(HardwareSettings {
            receipt_printer: Some(PrinterConnection::Preview),
            ..HardwareSettings::default_value()
        }),
    );
}

fn jobs() -> Vec<PreviewJob> {
    devices().preview.take_jobs()
}

fn code_of<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
    match result {
        Err(AppError::Validation { code, .. }) | Err(AppError::Conflict { code, .. }) => code,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn hw_04_a_cash_sale_prints_its_receipt_and_opens_the_drawer() {
    let (_folder, state) = shop_selling();

    let done = sell_with_cash(&state);

    let receipt = done.receipt.expect("printed after the sale");
    assert!(receipt.printed, "{receipt:?}");
    assert_eq!(
        receipt.printable,
        Some(Printable::Receipt {
            sale_id: done.sale_id.clone()
        })
    );
    let printed = jobs();
    assert_eq!(printed.len(), 1);
    assert!(printed[0].drawer_opened);
    assert_eq!(printed[0].pictures.len(), 1);
    check_golden("sale_receipt_80mm", &printed[0].pictures[0]);
}

#[test]
fn hw_04_a_card_sale_prints_without_opening_the_drawer() {
    let (_folder, state) = shop_selling();
    let card = app::commands::payment_methods::payment_method_save(
        &state,
        &app::workflows::payment_methods::PaymentMethodSave {
            id: None,
            name: "Card".into(),
            kind: app::workflows::checkout::MethodKind::Card,
            requires_reference: false,
            active: true,
            sort: 5,
        },
    )
    .expect("card");
    till::checkout_scan(&state, "BREAD-1").expect("bread");
    let entry = app::workflows::complete_sale::PaymentEntry {
        method_id: card,
        amount: Money(5_000),
        reference: None,
    };

    let done = till::checkout_complete(&state, devices(), &[entry]).expect("sold");

    assert!(done.receipt.expect("printed").printed);
    let printed = jobs();
    assert_eq!(printed.len(), 1);
    assert!(!printed[0].drawer_opened);
}

#[test]
fn set_14_copies_print_the_receipt_again_and_open_the_drawer_once() {
    let (_folder, state) = shop_selling();
    save(
        &state,
        SettingsChange::Receipt(ReceiptSettings {
            copies: 2,
            paper_width: settings::PaperWidth::Mm58,
            ..ReceiptSettings::default_value()
        }),
    );

    sell_with_cash(&state);

    let printed = jobs();
    assert_eq!(printed.len(), 2);
    assert!(printed[0].drawer_opened);
    assert!(!printed[1].drawer_opened);
    assert_eq!(printed[0].pictures, printed[1].pictures);
    assert_eq!(printed[0].pictures[0].width, 384);
    check_golden("sale_receipt_58mm", &printed[0].pictures[0]);
}

#[test]
fn pay_09_a_failed_printer_keeps_the_sale_and_offers_retry_and_the_system_printer() {
    let (_folder, state) = shop_selling();
    broken_printer(&state);

    let done = sell_with_cash(&state);

    // The sale is saved and the next customer can be served.
    assert_eq!(done.invoice_no, "T1-000001");
    assert!(done.cart.lines.is_empty());
    let receipt = done.receipt.expect("tried to print");
    assert!(!receipt.printed);
    assert!(matches!(
        receipt.problem,
        Some(AppError::Hardware { ref device, .. }) if device == "printer"
    ));
    let printable = receipt.printable.expect("what to retry");

    // Retry while the printer is still broken fails again and keeps waiting.
    let again = receipt_commands::print_retry(&state, devices(), &printable).expect("tried");
    assert!(!again.printed);

    // The system printer gets the same picture as PNG.
    let pictures =
        receipt_commands::print_system_pictures(&state, devices(), &printable).expect("pictures");
    assert_eq!(pictures.len(), 1);
    assert!(pictures[0].starts_with("data:image/png;base64,"));

    // Once the printer works, Retry prints the original (not a copy) and opens the drawer.
    preview_printer(&state);
    let fixed = receipt_commands::print_retry(&state, devices(), &printable).expect("printed");
    assert!(fixed.printed);
    let printed = jobs();
    assert_eq!(printed.len(), 1);
    assert!(printed[0].drawer_opened);
    check_golden("sale_receipt_80mm", &printed[0].pictures[0]);

    // Printed now, so it no longer waits.
    assert_eq!(
        code_of(receipt_commands::print_retry(&state, devices(), &printable)),
        "print_not_waiting"
    );
    assert_eq!(
        code_of(receipt_commands::print_system_pictures(
            &state,
            devices(),
            &printable
        )),
        "print_not_waiting"
    );
}

#[test]
fn pay_09_dismissing_the_banner_stops_the_retry() {
    let (_folder, state) = shop_selling();
    broken_printer(&state);
    let printable = sell_with_cash(&state)
        .receipt
        .and_then(|receipt| receipt.printable)
        .expect("printable");

    receipt_commands::print_dismiss(&state, devices(), &printable).expect("dismissed");

    assert_eq!(
        code_of(receipt_commands::print_retry(&state, devices(), &printable)),
        "print_not_waiting"
    );
}

#[test]
fn pay_09_retry_needs_someone_signed_in() {
    let (_folder, state) = shop_selling();
    broken_printer(&state);
    let printable = sell_with_cash(&state)
        .receipt
        .and_then(|receipt| receipt.printable)
        .expect("printable");
    app::session::logout(&state).expect("out");

    assert_eq!(
        code_of(receipt_commands::print_retry(&state, devices(), &printable)),
        "not_signed_in"
    );
}

#[test]
fn pay_10_reprint_last_is_marked_copy_and_audited() {
    let (_folder, state) = shop_selling();
    assert_eq!(
        code_of(receipt_commands::receipt_reprint_last(&state, devices())),
        "no_sale_to_reprint"
    );
    let done = sell_with_cash(&state);
    jobs();

    let reprint = receipt_commands::receipt_reprint_last(&state, devices()).expect("reprinted");

    assert!(reprint.printed);
    let printed = jobs();
    assert_eq!(printed.len(), 1);
    assert!(
        !printed[0].drawer_opened,
        "a reprint never opens the drawer"
    );
    check_golden("sale_receipt_copy_80mm", &printed[0].pictures[0]);
    let (user, _, details) = last_audit(&state, "receipt.reprint");
    assert_eq!(user, Some(user_id(&state, "owner").to_string()));
    assert!(details.contains(&done.invoice_no), "{details}");
}

#[test]
fn pay_10_reprint_any_finds_the_invoice_and_refuses_an_unknown_one() {
    let (_folder, state) = shop_selling();
    let first = sell_with_cash(&state);
    sell_with_cash(&state);
    jobs();

    let reprint =
        receipt_commands::receipt_reprint(&state, devices(), &first.invoice_no).expect("reprinted");

    assert_eq!(
        reprint.printable,
        Some(Printable::Receipt {
            sale_id: first.sale_id
        })
    );
    assert_eq!(jobs().len(), 1);
    assert!(matches!(
        receipt_commands::receipt_reprint(&state, devices(), "T1-999999"),
        Err(AppError::NotFound { .. })
    ));
}

#[test]
fn pay_10_a_cashier_reprints_the_last_receipt_but_not_older_ones() {
    let (_folder, state) = shop_selling();
    let done = sell_with_cash(&state);
    add_cashier(&state);
    app::session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");

    assert!(receipt_commands::receipt_reprint_last(&state, devices()).is_ok());
    assert!(matches!(
        receipt_commands::receipt_reprint(&state, devices(), &done.invoice_no),
        Err(AppError::PermissionDenied { .. })
    ));
}

#[test]
fn hw_04_cash_in_and_no_sale_open_the_drawer() {
    let (_folder, state) = shop_selling();
    let input = CashMoveInput {
        kind: CashMoveKind::CashIn,
        amount: Money(50_000),
        reason: "Change".into(),
    };

    let moved = shift_commands::shift_cash_move(&state, devices(), &input, None).expect("in");
    let opened =
        shift_commands::shift_no_sale(&state, devices(), "Change a note", None).expect("no sale");

    assert!(moved.printed && opened.printed);
    let printed = jobs();
    assert_eq!(printed.len(), 2);
    assert!(
        printed
            .iter()
            .all(|job| job.drawer_opened && job.pictures.is_empty())
    );
}

#[test]
fn hw_08_a_drawer_that_fails_is_reported_and_the_cash_move_stays_saved() {
    let (_folder, state) = shop_selling();
    broken_printer(&state);
    let input = CashMoveInput {
        kind: CashMoveKind::CashOut,
        amount: Money(1_000),
        reason: "Tea".into(),
    };

    let moved = shift_commands::shift_cash_move(&state, devices(), &input, None).expect("out");

    assert!(!moved.printed);
    assert!(moved.printable.is_none());
    assert_eq!(audit_count(&state, "shift.cash_move"), 1);
}

#[test]
fn shf_05_x_report_prints_the_own_open_shift() {
    let (_folder, state) = shop_selling();
    sell_with_cash(&state);
    jobs();

    let printed = shift_commands::shift_x_report(&state, devices()).expect("x report");

    assert!(printed.printed);
    let job = jobs();
    assert_eq!(job.len(), 1);
    assert!(!job[0].drawer_opened);
    check_golden("x_report_80mm", &job[0].pictures[0]);
}

#[test]
fn shf_05_x_report_refuses_someone_elses_shift_and_no_shift() {
    let (_folder, state) = shop_selling();
    add_user(
        &state,
        "karim",
        auth::ADMIN_ROLE_ID,
        Some("karim-pass-1"),
        None,
    );
    app::session::logout(&state).expect("out");
    sign_in(&state, "karim", "karim-pass-1");
    assert_eq!(
        code_of(shift_commands::shift_x_report(&state, devices())),
        "shift_not_yours"
    );
    shift_commands::shift_force_close(&state, devices(), &[], None, None).expect("closed");
    assert_eq!(
        code_of(shift_commands::shift_x_report(&state, devices())),
        "shift_not_open"
    );
}

#[test]
fn shf_06_closing_prints_the_z_report() {
    let (_folder, state) = shop_selling();
    sell_with_cash(&state);
    jobs();
    let counts = [
        CountLine {
            denomination: Money(100_000),
            count: 1,
        },
        CountLine {
            denomination: Money(10_000),
            count: 1,
        },
    ];

    let closed = shift_commands::shift_close(&state, devices(), &counts, None).expect("closed");

    let report = closed.report.expect("printed after closing");
    assert!(report.printed);
    assert_eq!(
        report.printable,
        Some(Printable::ShiftReport {
            shift_id: closed.shift_id
        })
    );
    let job = jobs();
    assert_eq!(job.len(), 1);
    check_golden("z_report_80mm", &job[0].pictures[0]);
}

#[test]
fn shf_07_force_close_prints_the_z_report_and_a_failed_one_can_be_retried() {
    let (_folder, state) = shop_selling();
    broken_printer(&state);

    let closed =
        shift_commands::shift_force_close(&state, devices(), &[], None, None).expect("closed");

    let report = closed.report.expect("tried to print");
    assert!(!report.printed);
    preview_printer(&state);
    let printable = report.printable.expect("printable");
    assert!(
        receipt_commands::print_retry(&state, devices(), &printable)
            .expect("retried")
            .printed
    );
    assert_eq!(jobs().len(), 1);
}

#[test]
fn hw_02_test_print_goes_to_the_saved_printer() {
    let (folder, state) = shop_selling();
    let file = folder.path().join("printer.bin");
    save(
        &state,
        SettingsChange::Hardware(HardwareSettings {
            receipt_printer: Some(PrinterConnection::File {
                path: file.to_string_lossy().into_owned(),
            }),
            ..HardwareSettings::default_value()
        }),
    );

    app::commands::hardware::hardware_test_print(&state, devices(), settings::PaperWidth::Mm58)
        .expect("printed");

    let bytes = std::fs::read(&file).expect("the printer got bytes");
    assert!(bytes.starts_with(&[0x1b, b'@']), "starts with ESC @");
    assert!(jobs().is_empty(), "nothing went to the Preview printer");
}

#[test]
fn hw_05_a_scale_in_the_settings_is_read_instead_of_the_fake_one() {
    let (_folder, state) = shop_selling();
    save(
        &state,
        SettingsChange::Hardware(HardwareSettings {
            scale: Some(ScaleSettings {
                port: "/dev/no-such-scale".into(),
                baud: 9600,
                profile: "cas".into(),
            }),
            ..HardwareSettings::default_value()
        }),
    );
    let potato = product(&state, "POTATO-1");

    let view = till::checkout_read_scale(&state, devices(), &potato).expect("read");

    assert!(view.has_scale);
    assert_eq!(view.qty, None);
    assert_eq!(view.problem.as_deref(), Some("not_connected"));
}
