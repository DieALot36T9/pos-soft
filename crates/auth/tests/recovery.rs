//! The recovery code for a forgotten admin password (USR-09).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod common;

use auth::LoginOutcome;
use common::{START, add_admin, add_cashier, secret, test_db};
use pos_core::Secret;

fn code_text(code: &Secret<String>) -> String {
    code.expose().clone()
}

#[test]
fn usr_09_recovery_code_is_shown_once_and_stored_as_a_hash() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();

    let code = auth::create_recovery_code(&transaction, START).unwrap();

    // Example: "7KQ2-M9XD-4RTW-PZ8C-H3NB": 20 characters in groups of 4.
    let text = code_text(&code);
    assert_eq!(text.len(), 24);
    assert_eq!(text.split('-').count(), 5);
    let stored: String = transaction
        .query_row("SELECT code_hash FROM auth_recovery", [], |row| row.get(0))
        .unwrap();
    assert!(stored.starts_with("$argon2id$"));
    assert!(!stored.contains(&text.replace('-', "")));
}

#[test]
fn usr_09_codes_are_different_each_time() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    let first = auth::create_recovery_code(&transaction, START).unwrap();
    let second = auth::create_recovery_code(&transaction, START).unwrap();
    assert_ne!(code_text(&first), code_text(&second));
}

#[test]
fn usr_09_correct_code_sets_a_new_admin_password_and_gives_a_new_code() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "forgotten password");
    let transaction = connection.transaction().unwrap();
    let code = auth::create_recovery_code(&transaction, START).unwrap();

    let new_code = auth::reset_with_recovery(
        &transaction,
        code.clone(),
        &owner,
        secret("fresh password"),
        START,
    )
    .unwrap();

    let login = auth::verify_login(&transaction, &owner, secret("fresh password"), START).unwrap();
    assert!(matches!(login, LoginOutcome::LoggedIn(_)));
    assert_ne!(code_text(&new_code), code_text(&code));
    // The old code no longer works; the new one does.
    let reused = auth::reset_with_recovery(
        &transaction,
        code,
        &owner,
        secret("another password"),
        START,
    );
    assert_eq!(common::error_code(reused), "recovery_code_wrong");
    auth::reset_with_recovery(
        &transaction,
        new_code,
        &owner,
        secret("another password"),
        START,
    )
    .unwrap();
}

#[test]
fn usr_09_code_is_accepted_in_small_letters_and_without_dashes() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "forgotten password");
    let transaction = connection.transaction().unwrap();
    let code = auth::create_recovery_code(&transaction, START).unwrap();
    let typed = code_text(&code).replace('-', " ").to_lowercase();

    auth::reset_with_recovery(
        &transaction,
        Secret::new(typed),
        &owner,
        secret("fresh password"),
        START,
    )
    .unwrap();
}

#[test]
fn usr_09_reset_also_clears_a_lock() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "forgotten password");
    let transaction = connection.transaction().unwrap();
    for _ in 0..5 {
        auth::verify_login(&transaction, &owner, secret("wrong guess"), START).unwrap();
    }
    let code = auth::create_recovery_code(&transaction, START).unwrap();

    auth::reset_with_recovery(&transaction, code, &owner, secret("fresh password"), START).unwrap();

    let login = auth::verify_login(&transaction, &owner, secret("fresh password"), START).unwrap();
    assert!(matches!(login, LoginOutcome::LoggedIn(_)));
}

#[test]
fn usr_09_wrong_code_changes_nothing() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "forgotten password");
    let transaction = connection.transaction().unwrap();
    auth::create_recovery_code(&transaction, START).unwrap();

    let result = auth::reset_with_recovery(
        &transaction,
        secret("AAAA-BBBB-CCCC-DDDD-EEEE"),
        &owner,
        secret("fresh password"),
        START,
    );

    assert_eq!(common::error_code(result), "recovery_code_wrong");
    let login =
        auth::verify_login(&transaction, &owner, secret("forgotten password"), START).unwrap();
    assert!(matches!(login, LoginOutcome::LoggedIn(_)));
}

#[test]
fn usr_09_without_a_saved_code_nothing_works() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "forgotten password");
    let transaction = connection.transaction().unwrap();

    let result = auth::reset_with_recovery(
        &transaction,
        secret("AAAA"),
        &owner,
        secret("fresh password"),
        START,
    );

    assert_eq!(common::error_code(result), "recovery_code_wrong");
}

#[test]
fn usr_09_only_an_admin_password_can_be_reset() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let transaction = connection.transaction().unwrap();
    let code = auth::create_recovery_code(&transaction, START).unwrap();

    let result =
        auth::reset_with_recovery(&transaction, code, &rina, secret("fresh password"), START);

    assert_eq!(common::error_code(result), "recovery_user_not_admin");
}

#[test]
fn usr_09_new_password_must_be_long_enough() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "forgotten password");
    let transaction = connection.transaction().unwrap();
    let code = auth::create_recovery_code(&transaction, START).unwrap();

    let result = auth::reset_with_recovery(&transaction, code, &owner, secret("short"), START);

    assert_eq!(common::error_code(result), "password_too_short");
}
