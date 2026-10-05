//! Managing users, and the "always one active admin" rule (USR-06, USR-07).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod common;

use auth::AuthError;
use auth::{LoginOutcome, NewUser, SecretKind, UserUpdate};
use common::{START, add_admin, add_cashier, admin_role, cashier_role, secret, test_db};
use pos_core::Id;
use rusqlite::Connection;

fn new_user(username: &str, pin: Option<&str>, password: Option<&str>) -> NewUser {
    NewUser {
        username: username.to_string(),
        display_name: "Rina Akter".to_string(),
        role_id: cashier_role(),
        password: password.map(secret),
        pin: pin.map(secret),
    }
}

fn create(connection: &mut Connection, user: NewUser) -> Result<Id, AuthError> {
    let transaction = connection.transaction().unwrap();
    let result = auth::create_user(&transaction, user, START);
    transaction.commit().unwrap();
    result
}

fn update(connection: &mut Connection, user: &Id, change: UserUpdate) -> Result<(), AuthError> {
    let transaction = connection.transaction().unwrap();
    let result = auth::update_user(&transaction, user, change, START);
    transaction.commit().unwrap();
    result
}

fn disable() -> UserUpdate {
    UserUpdate {
        display_name: None,
        role_id: None,
        active: Some(false),
    }
}

fn make_cashier() -> UserUpdate {
    UserUpdate {
        display_name: None,
        role_id: Some(cashier_role()),
        active: None,
    }
}

fn validation_code(result: Result<Id, AuthError>) -> String {
    common::error_code(result)
}

fn conflict_code(result: Result<(), AuthError>) -> String {
    common::error_code(result)
}

#[test]
fn usr_06_creates_a_user_with_a_pin() {
    let mut connection = test_db();
    let user = create(&mut connection, new_user("rina", Some("1234"), None)).unwrap();

    let tiles = auth::login_tiles(&connection).unwrap();
    assert_eq!(tiles.len(), 1);
    assert_eq!(tiles[0].id, user);
    assert_eq!(tiles[0].display_name, "Rina Akter");
}

#[test]
fn usr_06_secrets_are_stored_only_as_argon2id_hashes() {
    let mut connection = test_db();
    create(
        &mut connection,
        new_user("rina", Some("1234"), Some("password1")),
    )
    .unwrap();

    let (pin_hash, password_hash): (String, String) = connection
        .query_row(
            "SELECT pin_hash, password_hash FROM auth_users",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    // OWASP settings: argon2id, 19 MiB memory, 2 iterations, 1 lane.
    assert!(pin_hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
    assert!(password_hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
    assert!(!pin_hash.contains("1234"));
}

#[test]
fn usr_06_needs_a_pin_or_a_password() {
    let mut connection = test_db();
    let result = create(&mut connection, new_user("rina", None, None));
    assert_eq!(validation_code(result), "secret_missing");
}

#[test]
fn usr_06_pin_must_be_4_to_8_digits() {
    let mut connection = test_db();
    for bad_pin in ["123", "123456789", "12a4", ""] {
        let result = create(&mut connection, new_user("rina", Some(bad_pin), None));
        assert_eq!(validation_code(result), "pin_format", "PIN {bad_pin:?}");
    }
    create(&mut connection, new_user("eight", Some("12345678"), None)).unwrap();
}

#[test]
fn usr_06_password_must_be_at_least_8_characters() {
    let mut connection = test_db();
    let result = create(&mut connection, new_user("rina", None, Some("short12")));
    assert_eq!(validation_code(result), "password_too_short");
}

#[test]
fn usr_06_username_and_display_name_are_needed() {
    let mut connection = test_db();
    let result = create(&mut connection, new_user("  ", Some("1234"), None));
    assert_eq!(validation_code(result), "username_missing");

    let mut nameless = new_user("rina", Some("1234"), None);
    nameless.display_name = " ".to_string();
    assert_eq!(
        validation_code(create(&mut connection, nameless)),
        "display_name_missing"
    );
}

#[test]
fn usr_06_usernames_are_unique_ignoring_capitals() {
    let mut connection = test_db();
    create(&mut connection, new_user("rina", Some("1234"), None)).unwrap();

    let result = create(&mut connection, new_user("RINA", Some("5678"), None));

    assert_eq!(common::error_code(result), "username_in_use");
}

#[test]
fn usr_06_unknown_role_is_refused() {
    let mut connection = test_db();
    let mut user = new_user("rina", Some("1234"), None);
    user.role_id = Id::new();
    assert_eq!(
        common::error_code(create(&mut connection, user)),
        "not_found"
    );
}

#[test]
fn usr_06_the_last_active_admin_cannot_be_disabled() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "long password");

    assert_eq!(
        conflict_code(update(&mut connection, &owner, disable())),
        "last_active_admin"
    );
}

#[test]
fn usr_06_the_last_active_admin_cannot_be_demoted() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "long password");

    assert_eq!(
        conflict_code(update(&mut connection, &owner, make_cashier())),
        "last_active_admin"
    );
}

#[test]
fn usr_06_an_admin_can_be_disabled_while_another_admin_is_active() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "long password");
    let manager = add_admin(&mut connection, "manager", "other password");

    update(&mut connection, &owner, disable()).unwrap();

    // Now the manager is the last one.
    assert_eq!(
        conflict_code(update(&mut connection, &manager, disable())),
        "last_active_admin"
    );
}

#[test]
fn usr_06_a_disabled_admin_does_not_count() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "long password");
    let manager = add_admin(&mut connection, "manager", "other password");
    update(&mut connection, &owner, disable()).unwrap();

    assert_eq!(
        conflict_code(update(&mut connection, &manager, make_cashier())),
        "last_active_admin"
    );
}

#[test]
fn usr_06_cashiers_can_be_disabled_renamed_and_moved() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let change = UserUpdate {
        display_name: Some("Rina A.".to_string()),
        role_id: Some(admin_role()),
        active: None,
    };
    update(&mut connection, &rina, change).unwrap();

    let tiles = auth::login_tiles(&connection).unwrap();
    assert_eq!(tiles[0].display_name, "Rina A.");
    assert!(
        auth::permissions_of(&connection, &rina)
            .unwrap()
            .contains(pos_core::Permission::SaleVoid)
    );

    update(&mut connection, &rina, disable()).unwrap_err(); // now the last admin
}

#[test]
fn usr_06_update_of_unknown_user_is_not_found() {
    let mut connection = test_db();
    assert_eq!(
        common::error_code(update(&mut connection, &Id::new(), disable())),
        "not_found"
    );
}

#[test]
fn usr_06_an_admin_resets_another_users_password() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "long password");
    let transaction = connection.transaction().unwrap();

    auth::set_secret(
        &transaction,
        &owner,
        SecretKind::Password,
        secret("brand new pass"),
        START,
    )
    .unwrap();

    let old = auth::verify_login(&transaction, &owner, secret("long password"), START).unwrap();
    assert!(matches!(old, LoginOutcome::WrongSecret { .. }));
    let new = auth::verify_login(&transaction, &owner, secret("brand new pass"), START).unwrap();
    assert!(matches!(new, LoginOutcome::LoggedIn(_)));
}

#[test]
fn usr_07_a_cashier_adds_a_password_next_to_the_pin() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let transaction = connection.transaction().unwrap();

    auth::set_secret(
        &transaction,
        &rina,
        SecretKind::Password,
        secret("my password"),
        START,
    )
    .unwrap();

    let by_pin = auth::verify_login(&transaction, &rina, secret("1234"), START).unwrap();
    let by_password =
        auth::verify_login(&transaction, &rina, secret("my password"), START).unwrap();
    assert!(matches!(by_pin, LoginOutcome::LoggedIn(_)));
    assert!(matches!(by_password, LoginOutcome::LoggedIn(_)));
}

#[test]
fn usr_07_a_new_secret_is_checked_like_a_first_one() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let transaction = connection.transaction().unwrap();

    let result = auth::set_secret(&transaction, &rina, SecretKind::Pin, secret("12"), START);

    assert_eq!(common::error_code(result), "pin_format");
}
