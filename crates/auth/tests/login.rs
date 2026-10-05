//! Logging in and the lockout (USR-01, USR-02).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod common;

use auth::{LoginOutcome, SecretKind};
use common::{START, add_admin, add_cashier, add_user, cashier_role, secret, test_db};
use pos_core::{Clock, FixedClock, Id, Timestamp};
use rusqlite::Connection;

/// Tries a login in its own transaction and commits, like an `app` command does.
fn try_login(
    connection: &mut Connection,
    user: &Id,
    attempt: &str,
    now: Timestamp,
) -> LoginOutcome {
    let transaction = connection.transaction().unwrap();
    let outcome = auth::verify_login(&transaction, user, secret(attempt), now).unwrap();
    transaction.commit().unwrap();
    outcome
}

fn logged_in(outcome: &LoginOutcome) -> bool {
    matches!(outcome, LoginOutcome::LoggedIn(_))
}

#[test]
fn usr_01_correct_pin_logs_the_cashier_in() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");

    let outcome = try_login(&mut connection, &rina, "1234", START);

    let LoginOutcome::LoggedIn(session) = outcome else {
        panic!("expected a login, got {outcome:?}");
    };
    assert_eq!(session.id, rina);
    assert_eq!(session.username, "rina");
    assert_eq!(session.role_name, "Cashier");
    assert!(!session.is_admin);
    assert!(
        session
            .permissions
            .contains(pos_core::Permission::CheckoutSell)
    );
    assert!(!session.permissions.contains(pos_core::Permission::SaleVoid));
}

#[test]
fn usr_01_correct_password_logs_the_admin_in() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "long password");

    let outcome = try_login(&mut connection, &owner, "long password", START);

    let LoginOutcome::LoggedIn(session) = outcome else {
        panic!("expected a login, got {outcome:?}");
    };
    assert!(session.is_admin);
    assert_eq!(session.max_discount, pos_core::Bps(10000));
}

#[test]
fn usr_01_a_user_with_both_may_use_either() {
    let mut connection = test_db();
    let both = add_user(
        &mut connection,
        "both",
        cashier_role(),
        Some("password1"),
        Some("4321"),
    );

    assert!(logged_in(&try_login(&mut connection, &both, "4321", START)));
    assert!(logged_in(&try_login(
        &mut connection,
        &both,
        "password1",
        START
    )));
}

#[test]
fn usr_01_wrong_pin_is_refused_and_says_how_many_tries_are_left() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");

    let outcome = try_login(&mut connection, &rina, "9999", START);

    assert_eq!(outcome, LoginOutcome::WrongSecret { attempts_left: 4 });
}

#[test]
fn usr_01_login_tiles_show_active_users_only() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let owner = add_admin(&mut connection, "owner", "long password");
    let gone = add_cashier(&mut connection, "gone", "5555");
    let transaction = connection.transaction().unwrap();
    let disable = auth::UserUpdate {
        display_name: None,
        role_id: None,
        active: Some(false),
    };
    auth::update_user(&transaction, &gone, disable, START).unwrap();
    transaction.commit().unwrap();

    let tiles = auth::login_tiles(&connection).unwrap();

    let ids: Vec<Id> = tiles.iter().map(|tile| tile.id.clone()).collect();
    assert_eq!(ids, vec![owner.clone(), rina.clone()]); // sorted by display name
    let rina_tile = tiles.iter().find(|tile| tile.id == rina).unwrap();
    assert!(rina_tile.has_pin);
    assert!(!rina_tile.has_password);
}

#[test]
fn usr_01_a_disabled_user_cannot_log_in() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let _owner = add_admin(&mut connection, "owner", "long password");
    let transaction = connection.transaction().unwrap();
    let disable = auth::UserUpdate {
        display_name: None,
        role_id: None,
        active: Some(false),
    };
    auth::update_user(&transaction, &rina, disable, START).unwrap();

    let result = auth::verify_login(&transaction, &rina, secret("1234"), START);

    assert_eq!(common::error_code(result), "user_inactive");
}

#[test]
fn usr_02_five_failures_lock_the_user_for_60_seconds() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let clock = FixedClock::new(START, 0);

    for attempts_left in [4, 3, 2, 1] {
        let outcome = try_login(&mut connection, &rina, "0000", clock.now());
        assert_eq!(outcome, LoginOutcome::WrongSecret { attempts_left });
    }
    let fifth = try_login(&mut connection, &rina, "0000", clock.now());
    let until = Timestamp(START.0 + 60_000);
    assert_eq!(fifth, LoginOutcome::LockedNow { until });

    // Even the right PIN is refused while the lock lasts.
    clock.advance_ms(59_999);
    let during = try_login(&mut connection, &rina, "1234", clock.now());
    assert_eq!(during, LoginOutcome::StillLocked { until });
}

#[test]
fn usr_02_the_lock_ends_after_60_seconds() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let clock = FixedClock::new(START, 0);
    for _ in 0..5 {
        try_login(&mut connection, &rina, "0000", clock.now());
    }

    clock.advance_ms(60_000);

    assert!(logged_in(&try_login(
        &mut connection,
        &rina,
        "1234",
        clock.now()
    )));
}

#[test]
fn usr_02_after_the_lock_the_user_gets_five_new_tries() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let clock = FixedClock::new(START, 0);
    for _ in 0..5 {
        try_login(&mut connection, &rina, "0000", clock.now());
    }
    clock.advance_ms(60_000);

    let outcome = try_login(&mut connection, &rina, "0000", clock.now());

    assert_eq!(outcome, LoginOutcome::WrongSecret { attempts_left: 4 });
}

#[test]
fn usr_02_success_resets_the_failure_counter() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    for _ in 0..4 {
        try_login(&mut connection, &rina, "0000", START);
    }

    assert!(logged_in(&try_login(&mut connection, &rina, "1234", START)));

    // Four more failures don't lock, because the counter started again.
    for _ in 0..4 {
        let outcome = try_login(&mut connection, &rina, "0000", START);
        assert!(matches!(outcome, LoginOutcome::WrongSecret { .. }));
    }
}

#[test]
fn usr_02_other_users_can_still_log_in() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let sami = add_cashier(&mut connection, "sami", "5678");
    for _ in 0..5 {
        try_login(&mut connection, &rina, "0000", START);
    }

    assert!(logged_in(&try_login(&mut connection, &sami, "5678", START)));
}

#[test]
fn usr_02_a_new_pin_set_by_an_admin_clears_the_lock() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    for _ in 0..5 {
        try_login(&mut connection, &rina, "0000", START);
    }
    let transaction = connection.transaction().unwrap();
    auth::set_secret(&transaction, &rina, SecretKind::Pin, secret("2468"), START).unwrap();
    transaction.commit().unwrap();

    assert!(logged_in(&try_login(&mut connection, &rina, "2468", START)));
}
