//! Contract tests: the public API and tables behave as the module page says.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

mod common;

use auth::{ADMIN_ROLE_ID, CASHIER_ROLE_ID, LoginOutcome, PermissionSet, SessionUser, UserTile};
use common::{START, add_cashier, secret, test_db};
use pos_core::{Id, Permission};

#[test]
fn migrations_belong_to_this_module() {
    let migrations = auth::migrations();
    assert_eq!(migrations.module, "auth");
    assert_eq!(migrations.migrations[0].version, 1);
    assert_eq!(migrations.migrations[0].name, "init");
}

#[test]
fn migrations_create_only_auth_tables() {
    let connection = test_db();
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type IN ('table', 'view') ORDER BY name")
        .unwrap();
    let names: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        names,
        vec![
            "auth_recovery",
            "auth_role_permissions",
            "auth_roles",
            "auth_users"
        ]
    );
}

#[test]
fn system_roles_have_fixed_valid_ids() {
    let connection = test_db();
    for (id, name) in [(ADMIN_ROLE_ID, "Admin"), (CASHIER_ROLE_ID, "Cashier")] {
        Id::parse(id).unwrap();
        let (stored, is_system): (String, i64) = connection
            .query_row(
                "SELECT name, is_system FROM auth_roles WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(stored, name);
        assert_eq!(is_system, 1);
    }
}

#[test]
fn stored_permission_text_is_the_enum_in_snake_case() {
    let connection = test_db();
    let mut statement = connection
        .prepare("SELECT permission FROM auth_role_permissions")
        .unwrap();
    let texts: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for text in texts {
        assert!(
            Permission::parse(&text).is_some(),
            "unknown permission text {text}"
        );
    }
}

#[test]
fn unknown_permission_text_in_the_database_is_ignored() {
    // A database written by a newer version may hold permissions this version doesn't know.
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    connection
        .execute(
            "INSERT INTO auth_role_permissions (role_id, permission) VALUES (?1, 'from_the_future')",
            [CASHIER_ROLE_ID],
        )
        .unwrap();

    let permissions = auth::permissions_of(&connection, &rina).unwrap();

    assert!(permissions.contains(Permission::CheckoutSell));
}

#[test]
fn session_user_json_never_holds_a_secret() {
    let mut connection = test_db();
    let rina = add_cashier(&mut connection, "rina", "1234");
    let transaction = connection.transaction().unwrap();
    let LoginOutcome::LoggedIn(session) =
        auth::verify_login(&transaction, &rina, secret("1234"), START).unwrap()
    else {
        panic!("expected a login");
    };

    let json = serde_json::to_string(&session).unwrap();

    assert!(!json.contains("hash"));
    assert!(!json.contains("1234"));
    assert!(json.contains(r#""permissions":["checkout_sell","#));
    let _: &SessionUser = &session;
}

#[test]
fn login_tiles_json_shape() {
    let mut connection = test_db();
    add_cashier(&mut connection, "rina", "1234");

    let tiles: Vec<UserTile> = auth::login_tiles(&connection).unwrap();
    let json = serde_json::to_value(&tiles[0]).unwrap();

    let keys: Vec<&String> = json.as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        vec!["display_name", "has_password", "has_pin", "id", "username"]
    );
}

#[test]
fn permission_set_answers_contains() {
    let set = PermissionSet::from_list(&[Permission::StockView]);
    assert!(set.contains(Permission::StockView));
    assert!(!set.contains(Permission::StockAdjust));
    assert_eq!(set.len(), 1);
}
