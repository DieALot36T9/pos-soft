//! The audit log screen and its Excel export (USR-08).

mod common;

use app::audit_actions::{self, SESSION_LOGIN, SESSION_LOGOUT};
use app::commands::audit as audit_commands;
use app::session;
use audit::{AuditFilter, AuditPage};
use common::*;
use pos_core::{AppError, Permission};

const PAGE: AuditPage = AuditPage {
    limit: 50,
    offset: 0,
};

/// Owner logs out, rina logs in and out, owner logs back in.
fn shop_with_some_logins(folder: &std::path::Path) -> app::state::AppState {
    let state = start(folder);
    finish_setup(&state);
    add_cashier(&state);
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");
    session::logout(&state).expect("out");
    sign_in(&state, "owner", ADMIN_PASSWORD);
    state
}

#[test]
fn usr_08_the_audit_log_needs_audit_view() {
    let folder = tempfile::tempdir().expect("folder");
    let state = shop_with_some_logins(folder.path());
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");

    let refused = audit_commands::audit_query(&state, &AuditFilter::default(), PAGE);
    assert!(matches!(
        refused,
        Err(AppError::PermissionDenied {
            permission: Permission::AuditView
        })
    ));
}

#[test]
fn usr_08_events_show_newest_first_with_names_and_filters() {
    let folder = tempfile::tempdir().expect("folder");
    let state = shop_with_some_logins(folder.path());

    let all = audit_commands::audit_query(&state, &AuditFilter::default(), PAGE).expect("rows");
    assert_eq!(all.rows[0].action, SESSION_LOGIN);
    assert!(
        all.people
            .iter()
            .any(|person| person.display_name == "rina name")
    );
    assert_eq!(all.actions.len(), audit_actions::ALL.len());

    let rina = user_id(&state, "rina");
    let filter = AuditFilter {
        user_id: Some(rina.clone()),
        ..AuditFilter::default()
    };
    let hers = audit_commands::audit_query(&state, &filter, PAGE).expect("rows");
    let actions: Vec<&str> = hers.rows.iter().map(|row| row.action.as_str()).collect();
    assert_eq!(actions, vec![SESSION_LOGOUT, SESSION_LOGIN]);

    let filter = AuditFilter {
        action: Some(SESSION_LOGOUT.into()),
        ..AuditFilter::default()
    };
    let logouts = audit_commands::audit_query(&state, &filter, PAGE).expect("rows");
    assert_eq!(logouts.rows.len(), 2);
}

#[test]
fn usr_08_the_export_writes_every_event_the_filter_finds() {
    let folder = tempfile::tempdir().expect("folder");
    let state = shop_with_some_logins(folder.path());
    let file = folder.path().join("audit.xlsx");
    let filter = AuditFilter {
        action: Some(SESSION_LOGIN.into()),
        ..AuditFilter::default()
    };

    let written =
        audit_commands::audit_export(&state, &filter, &file.to_string_lossy()).expect("saved");

    assert_eq!(written, 2);
    let table = io::read_file(&file, None).expect("readable");
    assert_eq!(table.headers[0], "Time");
    assert_eq!(table.headers[3], "User");
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0][3], "Shop Owner");
    assert_eq!(table.rows[1][3], "rina name");
    assert_eq!(table.rows[0][5], SESSION_LOGIN);
    assert!(
        table.rows[0][0].starts_with("2026-10-05 14:00"),
        "{:?}",
        table.rows[0]
    );
}
