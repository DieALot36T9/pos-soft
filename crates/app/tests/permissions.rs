//! Every command with a permission, called for real: signed out it is refused,
//! and a cashier is refused exactly where the Cashier role lacks the permission
//! (Roles and Permissions, Security Model). A new command with a permission must
//! be added to `calls` below, or `every_guarded_command_is_called_here` fails.

mod common;

use app::commands::{
    COMMANDS, audit as audit_commands, backup as backup_commands, settings as settings_commands,
    tax as tax_commands, users as user_commands,
};
use app::session;
use app::state::AppState;
use app::workflows::settings_change::SettingsChange;
use app::workflows::users::{RoleInput, UserInput};
use audit::{AuditFilter, AuditPage};
use common::*;
use pos_core::{AppError, Bps, Id, Permission};
use settings::{CreditSettings, Setting};

type Call = fn(&AppState, &str) -> Result<(), AppError>;

fn ok<T>(result: Result<T, AppError>) -> Result<(), AppError> {
    result.map(|_| ())
}

/// One real call for each command that needs a permission. `file` is a backup file.
fn calls() -> Vec<(&'static str, Call)> {
    vec![
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
    ]
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
const TAKES_APPROVAL: &[&str] = &["backup_restore"];

fn shop() -> (tempfile::TempDir, AppState, String) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    add_cashier(&state);
    let file = backup_commands::backup_now(&state).expect("backed up").file;
    (folder, state, file)
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
