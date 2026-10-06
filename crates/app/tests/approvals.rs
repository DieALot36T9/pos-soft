//! Admin approval: one action, one permission, 60 seconds, used once (USR-05).

mod common;

use app::audit_actions::{APPROVAL_GRANTED, BACKUP_RESTORE};
use app::commands::{auth as auth_commands, backup as backup_commands};
use app::session::{self, ApprovalResult};
use common::*;
use pos_core::{AppError, Permission};

/// A shop with a backup file, and the cashier rina signed in.
fn shop_with_cashier(
    folder: &std::path::Path,
) -> (
    app::state::AppState,
    std::sync::Arc<pos_core::FixedClock>,
    String,
) {
    let (state, clock) = start_with_clock(folder);
    finish_setup(&state);
    add_cashier(&state);
    let file = backup_commands::backup_now(&state).expect("backed up").file;
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");
    (state, clock, file)
}

fn approve(state: &app::state::AppState) -> String {
    let result = auth_commands::request_override(
        state,
        Permission::BackupRestore,
        "owner",
        ADMIN_PASSWORD.into(),
    )
    .expect("runs");
    match result {
        ApprovalResult::Approved { token } => token,
        other => panic!("expected an approval, got {other:?}"),
    }
}

fn needs_approval(result: Result<(), AppError>) {
    assert!(
        matches!(
            result,
            Err(AppError::OverrideRequired {
                permission: Permission::BackupRestore
            })
        ),
        "{result:?}"
    );
}

#[test]
fn usr_05_an_admin_approves_one_restore_and_the_audit_names_both_users() {
    let folder = tempfile::tempdir().expect("folder");
    let (state, _clock, file) = shop_with_cashier(folder.path());
    let rina = user_id(&state, "rina");
    let owner = user_id(&state, "owner");

    needs_approval(backup_commands::backup_restore(&state, &file, None));
    let wrong = auth_commands::request_override(
        &state,
        Permission::BackupRestore,
        "owner",
        "not-the-password".into(),
    )
    .expect("runs");
    assert_eq!(wrong, ApprovalResult::WrongSecret { attempts_left: 4 });

    let token = approve(&state);
    assert_eq!(token.len(), 32);
    let (user, approved_by, details) = last_audit(&state, APPROVAL_GRANTED);
    assert_eq!(user.as_deref(), Some(rina.as_str()));
    assert_eq!(approved_by.as_deref(), Some(owner.as_str()));
    assert!(details.contains("backup_restore"), "{details}");

    backup_commands::backup_restore(&state, &file, Some(&token)).expect("restore asked");
    // The same token can't be used twice.
    needs_approval(backup_commands::backup_restore(&state, &file, Some(&token)));

    // After the restart, the restored database records who asked and who approved.
    drop(state);
    let state = start(folder.path());
    let (user, approved_by, _) = last_audit(&state, BACKUP_RESTORE);
    assert_eq!(user.as_deref(), Some(rina.as_str()));
    assert_eq!(approved_by.as_deref(), Some(owner.as_str()));
}

#[test]
fn usr_05_an_approval_runs_out_after_60_seconds() {
    let folder = tempfile::tempdir().expect("folder");
    let (state, clock, file) = shop_with_cashier(folder.path());

    let token = approve(&state);
    clock.advance_ms(60_000);

    needs_approval(backup_commands::backup_restore(&state, &file, Some(&token)));
}

#[test]
fn usr_05_an_approval_is_for_one_permission_only() {
    let folder = tempfile::tempdir().expect("folder");
    let (state, _clock, file) = shop_with_cashier(folder.path());

    let token = approve(&state);
    let other = session::require_or_approval(&state, Permission::SaleVoid, Some(&token));
    assert!(matches!(
        other,
        Err(AppError::OverrideRequired {
            permission: Permission::SaleVoid
        })
    ));
    // Trying it for the wrong action used it up.
    needs_approval(backup_commands::backup_restore(&state, &file, Some(&token)));
}

#[test]
fn usr_05_an_approval_ends_with_the_session() {
    let folder = tempfile::tempdir().expect("folder");
    let (state, _clock, file) = shop_with_cashier(folder.path());

    let token = approve(&state);
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");

    needs_approval(backup_commands::backup_restore(&state, &file, Some(&token)));
}

#[test]
fn usr_05_only_a_user_with_the_permission_can_approve() {
    let folder = tempfile::tempdir().expect("folder");
    let (state, _clock, _file) = shop_with_cashier(folder.path());
    add_user(
        &state,
        "karim",
        auth::CASHIER_ROLE_ID,
        Some("karim-password"),
        None,
    );

    let result = auth_commands::request_override(
        &state,
        Permission::BackupRestore,
        "karim",
        "karim-password".into(),
    )
    .expect("runs");
    assert_eq!(result, ApprovalResult::NotAllowed);
    assert!(matches!(
        auth_commands::request_override(
            &state,
            Permission::BackupRestore,
            "nobody",
            "whatever-1".into()
        ),
        Err(AppError::Validation { code, .. }) if code == "override_user_unknown"
    ));
}

#[test]
fn usr_05_approvals_need_someone_signed_in() {
    let folder = tempfile::tempdir().expect("folder");
    let (state, _clock, _file) = shop_with_cashier(folder.path());
    session::logout(&state).expect("out");

    assert!(matches!(
        auth_commands::request_override(
            &state,
            Permission::BackupRestore,
            "owner",
            ADMIN_PASSWORD.into()
        ),
        Err(AppError::Conflict { code, .. }) if code == "not_signed_in"
    ));
}

#[test]
fn usr_05_an_admin_needs_no_approval() {
    let folder = tempfile::tempdir().expect("folder");
    let (state, _clock, file) = shop_with_cashier(folder.path());
    session::logout(&state).expect("out");
    sign_in(&state, "owner", ADMIN_PASSWORD);

    backup_commands::backup_restore(&state, &file, None).expect("restore asked");
}
