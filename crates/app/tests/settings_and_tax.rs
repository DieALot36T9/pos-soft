//! Settings and tax rates after setup: permissions, audit, and the locks (SET-05, SET-13, TAX-10).

mod common;

use app::commands::{settings as settings_commands, tax as tax_commands};
use app::session;
use app::workflows::settings_change::SettingsChange;
use auth::{CASHIER_ROLE_ID, NewUser};
use common::*;
use pos_core::{AppError, CurrencyFormat, Id, Permission, Secret};
use settings::{CreditSettings, Setting};

fn add_password_cashier(state: &app::state::AppState) {
    let now = state.clock.now();
    state
        .db
        .write(|tx| {
            let cashier = NewUser {
                username: "rina".into(),
                display_name: "Rina".into(),
                role_id: Id::parse(CASHIER_ROLE_ID)?,
                password: Some(Secret::new("cashier-pass".into())),
                pin: None,
            };
            auth::create_user(tx, cashier, now)?;
            Ok(())
        })
        .expect("cashier added");
}

fn credit_change() -> SettingsChange {
    SettingsChange::Credit(CreditSettings {
        enabled: true,
        ..CreditSettings::default()
    })
}

#[test]
fn set_13_after_setup_settings_need_a_signed_in_admin() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    session::logout(&state).expect("signed out");

    let refused = settings_commands::settings_save(&state, &credit_change());
    assert!(matches!(
        refused,
        Err(AppError::PermissionDenied {
            permission: Permission::SettingsManage
        })
    ));

    add_password_cashier(&state);
    sign_in(&state, "rina", "cashier-pass");
    assert!(matches!(
        settings_commands::settings_save(&state, &credit_change()),
        Err(AppError::PermissionDenied { .. })
    ));
    assert!(matches!(
        tax_commands::tax_rate_save(&state, rate("Reduced 5%", 500)),
        Err(AppError::PermissionDenied {
            permission: Permission::TaxManage
        })
    ));

    sign_in(&state, "OWNER", ADMIN_PASSWORD);
    let saved = settings_commands::settings_save(&state, &credit_change()).expect("saved");
    assert!(saved.credit.enabled);
}

#[test]
fn set_13_each_settings_change_is_audited_with_old_and_new_values() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);

    settings_commands::settings_save(&state, &credit_change()).expect("saved");

    assert_eq!(audit_count(&state, "settings.change"), 1);
    let details: String = state
        .db
        .read(|conn| {
            Ok(conn
                .query_row(
                    "SELECT details_json FROM audit_events WHERE action = 'settings.change'",
                    [],
                    |row| row.get(0),
                )
                .expect("details"))
        })
        .expect("read");
    assert!(details.contains(r#""key":"credit""#), "{details}");
    assert!(details.contains(r#""old":{"#), "{details}");
}

#[test]
fn set_05_currency_decimals_lock_once_a_sale_exists() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let three_digits = |state: &app::state::AppState| {
        let mut currency = CurrencyFormat::default();
        currency.minor_digits = 3;
        let change = SettingsChange::Money {
            currency,
            rounding: money().rounding,
        };
        settings_commands::settings_save(state, &change)
    };
    // No sale yet: allowed.
    three_digits(&state).expect("no sale yet");

    // A saved return counts as a sale (the sales module's own rule).
    state
        .db
        .write(|tx| {
            tx.execute(
                "INSERT INTO sales_returns (id, return_no, till_id, shift_id, user_id, business_date,
                   at, net_minor, tax_minor, gross_minor, rounding_minor, total_minor, cost_minor)
                 VALUES ('r1', 'T1-R1', 't', 's', 'u', '2026-10-05', 0, 0, 0, 0, 0, 0, 0)",
                [],
            )
            .expect("return row");
            Ok(())
        })
        .expect("write");
    let mut back = CurrencyFormat::default();
    back.minor_digits = 2;
    let change = SettingsChange::Money {
        currency: back,
        rounding: money().rounding,
    };

    let refused = settings_commands::settings_save(&state, &change).expect_err("locked");

    assert!(matches!(
        refused,
        AppError::Validation { field: Some(field), code, .. }
            if field == "minor_digits" && code == "minor_digits_locked"
    ));
}

#[test]
fn tax_10_a_rate_is_deactivated_not_deleted_and_audited() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let id = tax_commands::tax_rate_save(&state, rate("Reduced 5%", 500)).expect("added");

    tax_commands::tax_rate_deactivate(&state, &id).expect("deactivated");

    let active = tax_commands::tax_rate_list(&state, false).expect("list");
    assert!(active.iter().all(|rate| rate.id != id));
    let all = tax_commands::tax_rate_list(&state, true).expect("list");
    assert!(all.iter().any(|rate| rate.id == id && !rate.active));
    assert_eq!(audit_count(&state, "tax_rate.change"), 2);
}

#[test]
fn set_05_money_preview_shows_the_format() {
    let text = settings_commands::money_preview(&CurrencyFormat::default()).expect("preview");
    assert_eq!(text, "৳12,34,567.50");
    let mut yen = CurrencyFormat::default();
    yen.minor_digits = 0;
    assert_eq!(
        settings_commands::money_preview(&yen).expect("preview"),
        "৳12,34,567"
    );
}

#[test]
fn bak_07_switching_detailed_logging_needs_logs_manage_and_is_saved() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);

    settings_commands::logs_set_detailed(&state, true).expect("switched");

    let saved: settings::LoggingSettings = state
        .db
        .read(|conn| Ok(settings::get(conn)?))
        .expect("read");
    assert!(saved.detailed);
    assert_eq!(audit_count(&state, "logging.switch"), 1);
    session::logout(&state).expect("out");
    assert!(matches!(
        settings_commands::logs_set_detailed(&state, false),
        Err(AppError::PermissionDenied {
            permission: Permission::LogsManage
        })
    ));
}
