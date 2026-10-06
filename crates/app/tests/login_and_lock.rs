//! Login tiles, PIN and password login, the lockout, the lock screen and logging
//! out (USR-01, USR-02, USR-03).

mod common;

use app::audit_actions::{SESSION_LOGIN, SESSION_LOGOUT, SESSION_UNLOCK_BY_ADMIN, USER_LOCKOUT};
use app::commands::{auth as auth_commands, backup as backup_commands};
use app::session::{self, LoginResult, SecretCheck};
use auth::UserUpdate;
use common::*;
use pos_core::{AppError, Permission};

fn conflict_code<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
    match result {
        Err(AppError::Conflict { code, .. }) => code,
        other => panic!("expected a conflict, got {other:?}"),
    }
}

#[test]
fn usr_01_the_tiles_show_active_users_and_a_cashier_logs_in_with_a_pin() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    add_cashier(&state);
    session::logout(&state).expect("out");

    let tiles = auth_commands::login_tiles(&state).expect("tiles");
    let names: Vec<&str> = tiles.iter().map(|tile| tile.username.as_str()).collect();
    assert_eq!(names, vec!["rina", "owner"]);
    assert!(tiles[0].has_pin && !tiles[0].has_password);

    let result = auth_commands::login(&state, &tiles[0].id, "1234".into()).expect("login");
    let LoginResult::LoggedIn { session: view } = result else {
        panic!("rina logs in: {result:?}");
    };
    assert_eq!(view.display_name, "rina name");
    assert_eq!(view.role_name, "Cashier");
    assert!(!view.is_admin && !view.locked);
    assert!(view.permissions.contains(&Permission::CheckoutSell));
    assert_eq!(session::current(&state).expect("current"), Some(view));
    let (user, _, _) = last_audit(&state, SESSION_LOGIN);
    assert_eq!(user.as_deref(), Some(tiles[0].id.as_str()));
}

#[test]
fn usr_01_logging_in_needs_a_finished_setup() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());

    assert_eq!(
        conflict_code(auth_commands::login_tiles(&state)),
        "setup_not_done"
    );
}

#[test]
fn usr_02_five_wrong_pins_lock_one_user_for_60_seconds() {
    let folder = tempfile::tempdir().expect("folder");
    let (state, clock) = start_with_clock(folder.path());
    finish_setup(&state);
    let rina = add_cashier(&state);
    session::logout(&state).expect("out");

    for left in [4, 3, 2, 1] {
        let result = session::login(&state, &rina, "9999".into()).expect("runs");
        assert_eq!(
            result,
            LoginResult::WrongSecret {
                attempts_left: left
            }
        );
    }
    let fifth = session::login(&state, &rina, "9999".into()).expect("runs");
    assert_eq!(
        fifth,
        LoginResult::Locked {
            until_ms: START_MS + 60_000
        }
    );
    assert_eq!(audit_count(&state, USER_LOCKOUT), 1);
    let (user, _, details) = last_audit(&state, USER_LOCKOUT);
    assert_eq!(user.as_deref(), Some(rina.as_str()));
    assert!(details.contains("login"), "{details}");

    // Even the right PIN waits for the lock to end. Other users can still log in.
    assert!(matches!(
        session::login(&state, &rina, "1234".into()).expect("runs"),
        LoginResult::Locked { .. }
    ));
    sign_in(&state, "owner", ADMIN_PASSWORD);
    session::logout(&state).expect("out");

    clock.advance_ms(60_000);
    sign_in(&state, "rina", "1234");
}

#[test]
fn usr_03_a_locked_till_keeps_the_user_but_refuses_every_command() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let rina = add_cashier(&state);

    let view = auth_commands::lock(&state).expect("locked");
    assert!(view.locked);
    assert_eq!(view.username, "owner");
    assert!(
        session::current(&state)
            .expect("current")
            .expect("signed in")
            .locked
    );
    assert_eq!(
        conflict_code(backup_commands::backup_list(&state)),
        "till_locked"
    );
    // Another user can't take over the locked till.
    assert_eq!(
        conflict_code(session::login(&state, &rina, "1234".into())),
        "till_locked"
    );

    let wrong = auth_commands::unlock(&state, "wrong-password".into()).expect("runs");
    assert_eq!(wrong, SecretCheck::WrongSecret { attempts_left: 4 });
    let right = auth_commands::unlock(&state, ADMIN_PASSWORD.into()).expect("runs");
    assert_eq!(right, SecretCheck::Accepted);
    assert!(backup_commands::backup_list(&state).is_ok());
    assert_eq!(
        conflict_code(auth_commands::unlock(&state, ADMIN_PASSWORD.into())),
        "till_not_locked"
    );
}

#[test]
fn usr_03_an_admin_unlocks_to_help_and_the_cashier_stays_signed_in() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let rina = add_cashier(&state);
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");
    auth_commands::lock(&state).expect("locked");

    // A cashier's PIN is not an admin password.
    let cashier = auth_commands::unlock_by_admin(&state, "rina", "1234".into()).expect("runs");
    assert!(matches!(cashier, SecretCheck::WrongSecret { .. }));
    let admin =
        auth_commands::unlock_by_admin(&state, "owner", ADMIN_PASSWORD.into()).expect("runs");
    assert_eq!(admin, SecretCheck::Accepted);

    let view = session::current(&state)
        .expect("current")
        .expect("signed in");
    assert_eq!(view.username, "rina");
    assert!(!view.locked);
    let (user, approved_by, _) = last_audit(&state, SESSION_UNLOCK_BY_ADMIN);
    assert_eq!(user.as_deref(), Some(rina.as_str()));
    assert_eq!(approved_by, Some(user_id(&state, "owner").to_string()));
}

#[test]
fn usr_03_switch_user_on_the_lock_screen_logs_out_and_is_audited() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    auth_commands::lock(&state).expect("locked");

    auth_commands::logout(&state).expect("out");

    assert_eq!(session::current(&state).expect("current"), None);
    assert_eq!(audit_count(&state, SESSION_LOGOUT), 1);
    assert!(matches!(
        backup_commands::backup_list(&state),
        Err(AppError::PermissionDenied { .. })
    ));
}

#[test]
fn usr_06_a_user_disabled_while_signed_in_loses_access_at_once() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let rina = add_cashier(&state);
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");
    assert!(backup_commands::backup_list(&state).is_ok());

    let now = state.clock.now();
    state
        .db
        .write(|tx| {
            let off = UserUpdate {
                active: Some(false),
                ..UserUpdate::default()
            };
            Ok(auth::update_user(tx, &rina, off, now)?)
        })
        .expect("disabled");

    assert!(matches!(
        backup_commands::backup_list(&state),
        Err(AppError::PermissionDenied { .. })
    ));
    assert_eq!(session::current(&state).expect("current"), None);
}

#[test]
fn usr_01_a_disabled_user_cant_log_in() {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    let rina = add_cashier(&state);
    let now = state.clock.now();
    state
        .db
        .write(|tx| {
            let off = UserUpdate {
                active: Some(false),
                ..UserUpdate::default()
            };
            Ok(auth::update_user(tx, &rina, off, now)?)
        })
        .expect("disabled");

    assert_eq!(
        conflict_code(session::login(&state, &rina, "1234".into())),
        "user_inactive"
    );
}
