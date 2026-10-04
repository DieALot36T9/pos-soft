//! Helpers shared by the auth test files.
// Each test file uses only some of these helpers.
#![allow(dead_code)]
// Helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

use auth::{ADMIN_ROLE_ID, CASHIER_ROLE_ID, NewUser};
use pos_core::{Id, Secret, Timestamp};
use rusqlite::Connection;

/// A moment to start the test clock at: 14 Nov 2023, 22:13 UTC.
pub const START: Timestamp = Timestamp(1_700_000_000_000);

/// An in-memory database built only from this module's own migrations (Module Rules R6).
pub fn test_db() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .unwrap();
    for migration in auth::migrations().migrations {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
}

/// Wraps text as a secret, the way the UI sends it.
pub fn secret(text: &str) -> Secret<String> {
    Secret::new(text.to_string())
}

/// The fixed id of the Admin role.
pub fn admin_role() -> Id {
    Id::parse(ADMIN_ROLE_ID).unwrap()
}

/// The fixed id of the Cashier role.
pub fn cashier_role() -> Id {
    Id::parse(CASHIER_ROLE_ID).unwrap()
}

/// Adds a cashier who logs in with a PIN.
pub fn add_cashier(connection: &mut Connection, username: &str, pin: &str) -> Id {
    add_user(connection, username, cashier_role(), None, Some(pin))
}

/// Adds an admin who logs in with a password.
pub fn add_admin(connection: &mut Connection, username: &str, password: &str) -> Id {
    add_user(connection, username, admin_role(), Some(password), None)
}

/// Adds a user and commits.
pub fn add_user(
    connection: &mut Connection,
    username: &str,
    role_id: Id,
    password: Option<&str>,
    pin: Option<&str>,
) -> Id {
    let transaction = connection.transaction().unwrap();
    let user = auth::create_user(
        &transaction,
        NewUser {
            username: username.to_string(),
            display_name: format!("{username} display"),
            role_id,
            password: password.map(secret),
            pin: pin.map(secret),
        },
        START,
    )
    .unwrap();
    transaction.commit().unwrap();
    user
}

/// The code the UI would get for a refused call. `NotFound` gives "not_found".
pub fn error_code<T: std::fmt::Debug>(result: Result<T, auth::AuthError>) -> String {
    match result.map_err(pos_core::AppError::from) {
        Err(pos_core::AppError::Validation { code, .. }) => code,
        Err(pos_core::AppError::Conflict { code, .. }) => code,
        Err(pos_core::AppError::NotFound { .. }) => "not_found".to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
