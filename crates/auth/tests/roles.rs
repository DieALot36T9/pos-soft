//! Roles and permissions (USR-04, Roles and Permissions).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod common;

use auth::NewRole;
use common::{START, add_admin, add_cashier, add_user, admin_role, cashier_role, test_db};
use pos_core::{Bps, Id, Permission};
use rusqlite::Connection;

fn supervisor() -> NewRole {
    NewRole {
        name: "Supervisor".to_string(),
        permissions: vec![Permission::CheckoutSell, Permission::SaleVoid],
        max_discount: Bps(1000),
    }
}

fn create_role(connection: &mut Connection, role: NewRole) -> Result<Id, auth::AuthError> {
    let transaction = connection.transaction().unwrap();
    let result = auth::create_role(&transaction, role, START);
    transaction.commit().unwrap();
    result
}

#[test]
fn usr_04_admin_has_every_permission_including_new_ones() {
    let mut connection = test_db();
    let owner = add_admin(&mut connection, "owner", "long password");

    let permissions = auth::permissions_of(&connection, &owner).unwrap();

    // Permission::ALL lists every enum variant, so a variant added later is checked too.
    // Nothing is stored for Admin, so a new variant needs no migration to reach it.
    for permission in Permission::ALL {
        assert!(
            permissions.contains(permission),
            "Admin is missing {permission:?}"
        );
    }
    let stored: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM auth_role_permissions WHERE role_id = ?1",
            [auth::ADMIN_ROLE_ID],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, 0);
}

#[test]
fn usr_04_cashier_gets_the_default_c_permissions() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");

    let permissions = auth::permissions_of(&connection, &rina).unwrap();

    let expected = [
        Permission::CheckoutSell,
        Permission::CheckoutCancelCart,
        Permission::CheckoutHoldResume,
        Permission::CheckoutManualWeight,
        Permission::DiscountApply,
        Permission::SellOnCredit,
        Permission::ReceiptReprintLast,
        Permission::ShiftOpenCloseOwn,
        Permission::CustomerViewCreate,
        Permission::CustomerReceivePayment,
        Permission::StockView,
        Permission::StockCountEnter,
        Permission::LabelsPrint,
        Permission::BackupRun,
    ];
    let got: Vec<Permission> = permissions.iter().collect();
    let mut want = expected.to_vec();
    want.sort();
    assert_eq!(got, want);
}

#[test]
fn usr_04_custom_role_has_exactly_its_ticked_permissions() {
    let mut connection = test_db();
    let role = create_role(&mut connection, supervisor()).unwrap();
    let mina = add_user(&mut connection, "mina", role, None, Some("1111"));

    let permissions = auth::permissions_of(&connection, &mina).unwrap();

    assert_eq!(
        permissions.iter().collect::<Vec<_>>(),
        vec![Permission::CheckoutSell, Permission::SaleVoid]
    );
}

#[test]
fn usr_04_role_discount_limit_reaches_the_session() {
    let mut connection = test_db();
    let role = create_role(&mut connection, supervisor()).unwrap();
    let mina = add_user(&mut connection, "mina", role, None, Some("1111"));
    let transaction = connection.transaction().unwrap();

    let outcome = auth::verify_login(&transaction, &mina, common::secret("1111"), START).unwrap();

    let auth::LoginOutcome::LoggedIn(session) = outcome else {
        panic!("expected a login");
    };
    assert_eq!(session.max_discount, Bps(1000));
    assert_eq!(session.role_name, "Supervisor");
}

#[test]
fn usr_04_role_permissions_can_be_changed() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let transaction = connection.transaction().unwrap();

    auth::set_role_permissions(
        &transaction,
        &cashier_role(),
        &[Permission::StockView],
        Bps(500),
    )
    .unwrap();

    let permissions = auth::permissions_of(&transaction, &rina).unwrap();
    assert_eq!(
        permissions.iter().collect::<Vec<_>>(),
        vec![Permission::StockView]
    );
}

#[test]
fn usr_04_the_admin_role_cannot_be_edited() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();

    let result = auth::set_role_permissions(&transaction, &admin_role(), &[], Bps(0));

    assert_eq!(common::error_code(result), "admin_role_fixed");
}

#[test]
fn usr_04_role_names_are_unique_and_needed() {
    let mut connection = test_db();
    create_role(&mut connection, supervisor()).unwrap();

    let again = create_role(&mut connection, supervisor());
    assert_eq!(common::error_code(again), "role_name_in_use");

    let mut nameless = supervisor();
    nameless.name = "  ".to_string();
    let result = create_role(&mut connection, nameless);
    assert_eq!(common::error_code(result), "role_name_missing");
}

#[test]
fn usr_04_discount_limit_is_at_most_100_percent() {
    let mut connection = test_db();
    let mut greedy = supervisor();
    greedy.max_discount = Bps(10001);

    let result = create_role(&mut connection, greedy);

    assert_eq!(common::error_code(result), "max_discount_too_high");
}

#[test]
fn usr_04_unknown_role_is_not_found() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    let result = auth::set_role_permissions(&transaction, &Id::new(), &[], Bps(0));
    assert_eq!(common::error_code(result), "not_found");
}

#[test]
fn usr_06_a_disabled_user_has_no_permissions() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let transaction = connection.transaction().unwrap();
    let disable = auth::UserUpdate {
        display_name: None,
        role_id: None,
        active: Some(false),
    };
    auth::update_user(&transaction, &rina, disable, START).unwrap();

    assert!(
        auth::permissions_of(&transaction, &rina)
            .unwrap()
            .is_empty()
    );
}
