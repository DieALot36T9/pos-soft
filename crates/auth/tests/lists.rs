//! The lists for the users and roles screens, and reading a signed-in user again
//! (USR-04, USR-06; used by `app` in C-M3).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod common;

use auth::{ADMIN_ROLE_ID, CASHIER_ROLE_ID, LoginOutcome, NewRole, UserUpdate};
use common::{START, add_admin, add_cashier, error_code, secret, test_db};
use pos_core::{Bps, Permission, Timestamp};

#[test]
fn usr_06_the_user_list_shows_disabled_users_and_their_role() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "long password");
    let rina = add_cashier(&mut connection, "rina", "1234");
    let transaction = connection.transaction().unwrap();
    let off = UserUpdate {
        active: Some(false),
        ..UserUpdate::default()
    };
    auth::update_user(&transaction, &rina, off, START).unwrap();
    transaction.commit().unwrap();

    let users = auth::list_users(&connection).unwrap();

    assert_eq!(users.len(), 2);
    let first = &users[0];
    assert_eq!(first.id, owner);
    assert_eq!(first.username, "owner");
    assert_eq!(first.role_name, "Admin");
    assert!(first.is_admin);
    assert!(first.active);
    assert!(first.has_password);
    assert!(!first.has_pin);
    let second = &users[1];
    assert_eq!(second.id, rina);
    assert_eq!(second.role_id.as_str(), CASHIER_ROLE_ID);
    assert!(!second.active);
    assert!(second.has_pin);
    assert_eq!(second.locked_until, None);
}

#[test]
fn usr_02_the_user_list_shows_a_lock() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let transaction = connection.transaction().unwrap();
    for _ in 0..5 {
        auth::verify_login(&transaction, &rina, secret("9999"), START).unwrap();
    }
    transaction.commit().unwrap();

    let users = auth::list_users(&connection).unwrap();

    assert_eq!(users[0].locked_until, Some(Timestamp(START.0 + 60_000)));
}

#[test]
fn usr_04_the_role_list_shows_permissions_and_limits() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    let role = NewRole {
        name: "Supervisor".into(),
        permissions: vec![Permission::SaleVoid],
        max_discount: Bps(1000),
    };
    let supervisor = auth::create_role(&transaction, role, START).unwrap();
    transaction.commit().unwrap();

    let roles = auth::list_roles(&connection).unwrap();

    let names: Vec<&str> = roles.iter().map(|role| role.name.as_str()).collect();
    assert_eq!(names, vec!["Admin", "Cashier", "Supervisor"]);
    let admin = &roles[0];
    assert_eq!(admin.id.as_str(), ADMIN_ROLE_ID);
    assert!(admin.is_system && admin.is_admin);
    assert_eq!(admin.permissions.len(), Permission::ALL.len());
    assert!(roles[1].is_system && !roles[1].is_admin);
    assert!(roles[1].permissions.contains(Permission::CheckoutSell));
    let custom = &roles[2];
    assert_eq!(custom.id, supervisor);
    assert!(!custom.is_system);
    assert_eq!(custom.max_discount, Bps(1000));
    assert_eq!(
        custom.permissions.iter().collect::<Vec<_>>(),
        vec![Permission::SaleVoid]
    );
}

#[test]
fn usr_06_a_signed_in_user_is_read_again_with_their_current_role() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let transaction = connection.transaction().unwrap();
    let LoginOutcome::LoggedIn(at_login) =
        auth::verify_login(&transaction, &rina, secret("1234"), START).unwrap()
    else {
        panic!("rina logs in");
    };
    let promote = UserUpdate {
        role_id: Some(pos_core::Id::parse(ADMIN_ROLE_ID).unwrap()),
        ..UserUpdate::default()
    };
    auth::update_user(&transaction, &rina, promote, START).unwrap();
    transaction.commit().unwrap();

    let now = auth::session_user(&connection, &rina).unwrap();

    assert!(!at_login.is_admin);
    assert!(now.is_admin);
    assert_eq!(now.role_name, "Admin");
    assert_eq!(now.permissions.len(), Permission::ALL.len());
}

#[test]
fn usr_06_a_disabled_user_cant_be_read_as_signed_in() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "long password");
    let rina = add_cashier(&mut connection, "rina", "1234");
    let transaction = connection.transaction().unwrap();
    let off = UserUpdate {
        active: Some(false),
        ..UserUpdate::default()
    };
    auth::update_user(&transaction, &rina, off, START).unwrap();
    transaction.commit().unwrap();

    assert!(auth::session_user(&connection, &owner).is_ok());
    assert_eq!(
        error_code(auth::session_user(&connection, &rina)),
        "user_inactive"
    );
}

#[test]
fn the_lists_are_sent_to_the_ui_in_snake_case() {
    let mut connection = test_db();
    add_cashier(&mut connection, "rina", "1234");
    let users = serde_json::to_value(auth::list_users(&connection).unwrap()).unwrap();
    let roles = serde_json::to_value(auth::list_roles(&connection).unwrap()).unwrap();
    for key in [
        "id",
        "username",
        "display_name",
        "role_id",
        "role_name",
        "is_admin",
        "active",
        "has_pin",
        "has_password",
        "locked_until",
    ] {
        assert!(users[0].get(key).is_some(), "user {key}");
    }
    for key in [
        "id",
        "name",
        "is_system",
        "is_admin",
        "max_discount",
        "permissions",
    ] {
        assert!(roles[0].get(key).is_some(), "role {key}");
    }
}
