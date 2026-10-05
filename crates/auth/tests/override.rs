//! Admin approval for one action (USR-05).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod common;

use auth::OverrideOutcome;
use common::{START, add_admin, add_cashier, add_user, secret, test_db};
use pos_core::Permission;

#[test]
fn usr_05_admin_password_approves_the_action() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "long password");
    let transaction = connection.transaction().unwrap();

    let outcome = auth::verify_override(
        &transaction,
        "Owner",
        secret("long password"),
        Permission::SaleVoid,
        START,
    )
    .unwrap();

    assert_eq!(outcome, OverrideOutcome::Approved { approved_by: owner });
}

#[test]
fn usr_05_fails_for_a_user_without_the_needed_permission() {
    let mut connection = test_db();
    add_user(
        &mut connection,
        "sami",
        common::cashier_role(),
        Some("cashier password"),
        None,
    );
    let transaction = connection.transaction().unwrap();

    let outcome = auth::verify_override(
        &transaction,
        "sami",
        secret("cashier password"),
        Permission::SaleVoid,
        START,
    )
    .unwrap();

    assert_eq!(outcome, OverrideOutcome::NotAllowed);
}

#[test]
fn usr_05_wrong_password_is_refused_and_counts_toward_the_lock() {
    let mut connection = test_db();
    add_admin(&mut connection, "owner", "long password");
    let transaction = connection.transaction().unwrap();

    for attempts_left in [4, 3, 2, 1] {
        let outcome = auth::verify_override(
            &transaction,
            "owner",
            secret("guess"),
            Permission::SaleVoid,
            START,
        )
        .unwrap();
        assert_eq!(outcome, OverrideOutcome::WrongSecret { attempts_left });
    }
    let fifth = auth::verify_override(
        &transaction,
        "owner",
        secret("guess"),
        Permission::SaleVoid,
        START,
    )
    .unwrap();
    assert!(matches!(fifth, OverrideOutcome::LockedNow { .. }));
    let locked = auth::verify_override(
        &transaction,
        "owner",
        secret("long password"),
        Permission::SaleVoid,
        START,
    )
    .unwrap();
    assert!(matches!(locked, OverrideOutcome::StillLocked { .. }));
}

#[test]
fn usr_05_needs_the_password_not_the_pin() {
    // Rule from the product owner: admin-only actions need the admin password.
    let mut connection = test_db();
    add_user(
        &mut connection,
        "boss",
        common::admin_role(),
        Some("long password"),
        Some("9999"),
    );
    let transaction = connection.transaction().unwrap();

    let outcome = auth::verify_override(
        &transaction,
        "boss",
        secret("9999"),
        Permission::SaleVoid,
        START,
    )
    .unwrap();

    assert!(matches!(outcome, OverrideOutcome::WrongSecret { .. }));
}

#[test]
fn usr_05_unknown_or_disabled_users_cannot_approve() {
    let mut connection = test_db();
    add_admin(&mut connection, "owner", "long password");
    let helper = add_user(
        &mut connection,
        "helper",
        common::admin_role(),
        Some("helper password"),
        None,
    );
    add_cashier(&mut connection, "rina", "1234");
    let transaction = connection.transaction().unwrap();
    let disable = auth::UserUpdate {
        display_name: None,
        role_id: None,
        active: Some(false),
    };
    auth::update_user(&transaction, &helper, disable, START).unwrap();

    let unknown = auth::verify_override(
        &transaction,
        "nobody",
        secret("x"),
        Permission::SaleVoid,
        START,
    );
    assert_eq!(common::error_code(unknown), "override_user_unknown");
    let disabled = auth::verify_override(
        &transaction,
        "helper",
        secret("helper password"),
        Permission::SaleVoid,
        START,
    );
    assert_eq!(common::error_code(disabled), "override_user_unknown");
}
