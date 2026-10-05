//! Users: creating, changing and disabling them, their PIN and password,
//! and the tiles on the login screen (USR-01, USR-06, USR-07).

use pos_core::{Id, Secret, Timestamp};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AuthError;
use crate::hashing::hash_secret;
use crate::roles::{ADMIN_ROLE_ID, role_info};
use crate::rules::{LockState, check_password, check_pin};

/// A new user, as filled in on the "Add user" form. At least one of PIN and password is needed.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct NewUser {
    /// The name typed for admin approvals. Example: "rina". Capitals don't matter.
    pub username: String,
    /// The name on the login tile. Example: "Rina Akter".
    pub display_name: String,
    pub role_id: Id,
    #[ts(type = "string | null")]
    pub password: Option<Secret<String>>,
    #[ts(type = "string | null")]
    pub pin: Option<Secret<String>>,
}

/// Changes to a user. Fields left empty stay as they are.
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[ts(export)]
pub struct UserUpdate {
    pub display_name: Option<String>,
    pub role_id: Option<Id>,
    /// `Some(false)` disables the user. Users are never deleted, because they have history.
    pub active: Option<bool>,
}

/// Which secret to set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SecretKind {
    Password,
    Pin,
}

/// One tile on the login screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct UserTile {
    pub id: Id,
    pub username: String,
    pub display_name: String,
    /// The user can log in with a PIN.
    pub has_pin: bool,
    /// The user can log in with a password.
    pub has_password: bool,
}

/// One row of `auth_users`, for use inside this module.
pub(crate) struct UserRow {
    pub id: Id,
    pub username: String,
    pub display_name: String,
    pub role_id: String,
    pub password_hash: Option<String>,
    pub pin_hash: Option<String>,
    pub active: bool,
    pub lock: LockState,
}

const USER_COLUMNS: &str = "id, username, display_name, role_id, password_hash, pin_hash,
                            active, failed_attempts, locked_until";

/// Adds a user. The PIN and password are stored only as argon2id hashes. Returns the new id.
pub fn create_user(
    transaction: &Transaction,
    user: NewUser,
    now: Timestamp,
) -> Result<Id, AuthError> {
    let username = user.username.trim();
    if username.is_empty() {
        return Err(AuthError::UsernameMissing);
    }
    let display_name = checked_display_name(&user.display_name)?;
    if user.pin.is_none() && user.password.is_none() {
        return Err(AuthError::SecretMissing);
    }
    let pin_hash = checked_hash(user.pin.as_ref(), SecretKind::Pin)?;
    let password_hash = checked_hash(user.password.as_ref(), SecretKind::Password)?;
    role_info(transaction, user.role_id.as_str())?;
    if find_user_by_username(transaction, username)?.is_some() {
        return Err(AuthError::UsernameInUse);
    }

    let id = Id::new();
    transaction.execute(
        "INSERT INTO auth_users (id, username, display_name, role_id, password_hash, pin_hash,
                                 created_at, updated_at)
         VALUES (:id, :username, :display_name, :role_id, :password_hash, :pin_hash, :now, :now)",
        named_params! {
            ":id": id.as_str(),
            ":username": username,
            ":display_name": display_name,
            ":role_id": user.role_id.as_str(),
            ":password_hash": password_hash,
            ":pin_hash": pin_hash,
            ":now": now.0,
        },
    )?;
    Ok(id)
}

/// Renames, moves to another role, or disables/enables a user.
/// Refuses anything that would leave no active admin (USR-06).
pub fn update_user(
    transaction: &Transaction,
    user: &Id,
    change: UserUpdate,
    now: Timestamp,
) -> Result<(), AuthError> {
    let row = load_user(transaction, user)?;
    let display_name = match &change.display_name {
        Some(name) => checked_display_name(name)?.to_string(),
        None => row.display_name.clone(),
    };
    let role_id = match &change.role_id {
        Some(role) => role.as_str().to_string(),
        None => row.role_id.clone(),
    };
    role_info(transaction, &role_id)?;
    let active = change.active.unwrap_or(row.active);

    let is_active_admin_now = row.active && row.role_id == ADMIN_ROLE_ID;
    let stays_active_admin = active && role_id == ADMIN_ROLE_ID;
    if is_active_admin_now && !stays_active_admin && other_active_admins(transaction, user)? == 0 {
        return Err(AuthError::LastActiveAdmin);
    }

    transaction.execute(
        "UPDATE auth_users
         SET display_name = :display_name, role_id = :role_id, active = :active, updated_at = :now
         WHERE id = :id",
        named_params! {
            ":display_name": display_name,
            ":role_id": role_id,
            ":active": active,
            ":now": now.0,
            ":id": user.as_str(),
        },
    )?;
    Ok(())
}

/// Sets a user's new PIN or password. Also ends any lockout, so an admin's reset
/// lets a locked user straight back in. For USR-07, `app` checks the current
/// secret with `verify_login` first.
pub fn set_secret(
    transaction: &Transaction,
    user: &Id,
    kind: SecretKind,
    secret: Secret<String>,
    now: Timestamp,
) -> Result<(), AuthError> {
    let hash = checked_hash(Some(&secret), kind)?;
    let sql = match kind {
        SecretKind::Password => {
            "UPDATE auth_users SET password_hash = :hash, failed_attempts = 0,
                                   locked_until = NULL, updated_at = :now
             WHERE id = :id"
        }
        SecretKind::Pin => {
            "UPDATE auth_users SET pin_hash = :hash, failed_attempts = 0,
                                   locked_until = NULL, updated_at = :now
             WHERE id = :id"
        }
    };
    let changed = transaction.execute(
        sql,
        named_params! { ":hash": hash, ":now": now.0, ":id": user.as_str() },
    )?;
    if changed == 0 {
        return Err(AuthError::UserNotFound {
            user: user.to_string(),
        });
    }
    Ok(())
}

/// The active users, as tiles for the login screen, sorted by display name.
pub fn login_tiles(connection: &Connection) -> Result<Vec<UserTile>, AuthError> {
    let mut statement = connection.prepare(&format!(
        "SELECT {USER_COLUMNS} FROM auth_users WHERE active = 1
         ORDER BY display_name COLLATE NOCASE, username"
    ))?;
    let rows = statement.query_map([], read_user_row)?;
    let mut tiles = Vec::new();
    for row in rows {
        let user = row?;
        tiles.push(UserTile {
            has_pin: user.pin_hash.is_some(),
            has_password: user.password_hash.is_some(),
            id: user.id,
            username: user.username,
            display_name: user.display_name,
        });
    }
    Ok(tiles)
}

/// Reads one user by id.
pub(crate) fn load_user(connection: &Connection, user: &Id) -> Result<UserRow, AuthError> {
    let found = connection
        .query_row(
            &format!("SELECT {USER_COLUMNS} FROM auth_users WHERE id = :id"),
            named_params! { ":id": user.as_str() },
            read_user_row,
        )
        .optional()?;
    found.ok_or_else(|| AuthError::UserNotFound {
        user: user.to_string(),
    })
}

/// Reads one user by username. Capitals don't matter: "Owner" finds "owner".
pub(crate) fn find_user_by_username(
    connection: &Connection,
    username: &str,
) -> Result<Option<UserRow>, AuthError> {
    let found = connection
        .query_row(
            &format!("SELECT {USER_COLUMNS} FROM auth_users WHERE username = :username"),
            named_params! { ":username": username.trim() },
            read_user_row,
        )
        .optional()?;
    Ok(found)
}

/// Saves the failure count and lock after a login or approval attempt.
pub(crate) fn save_lock(
    transaction: &Transaction,
    user: &Id,
    lock: LockState,
) -> Result<(), AuthError> {
    transaction.execute(
        "UPDATE auth_users SET failed_attempts = :failed, locked_until = :until WHERE id = :id",
        named_params! {
            ":failed": lock.failed_attempts,
            ":until": lock.locked_until.map(|until| until.0),
            ":id": user.as_str(),
        },
    )?;
    Ok(())
}

fn read_user_row(row: &Row) -> rusqlite::Result<UserRow> {
    let id_text: String = row.get("id")?;
    let id = Id::parse(&id_text).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(UserRow {
        id,
        username: row.get("username")?,
        display_name: row.get("display_name")?,
        role_id: row.get("role_id")?,
        password_hash: row.get("password_hash")?,
        pin_hash: row.get("pin_hash")?,
        active: row.get("active")?,
        lock: LockState {
            failed_attempts: row.get("failed_attempts")?,
            locked_until: row.get::<_, Option<i64>>("locked_until")?.map(Timestamp),
        },
    })
}

/// How many active admins there are besides this user.
fn other_active_admins(transaction: &Transaction, user: &Id) -> Result<i64, AuthError> {
    let count = transaction.query_row(
        "SELECT COUNT(*) FROM auth_users WHERE active = 1 AND role_id = :admin AND id <> :id",
        named_params! { ":admin": ADMIN_ROLE_ID, ":id": user.as_str() },
        |row| row.get(0),
    )?;
    Ok(count)
}

fn checked_display_name(name: &str) -> Result<&str, AuthError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AuthError::DisplayNameMissing);
    }
    Ok(trimmed)
}

/// Checks a secret's format and hashes it. `None` stays `None`.
fn checked_hash(
    secret: Option<&Secret<String>>,
    kind: SecretKind,
) -> Result<Option<String>, AuthError> {
    let Some(secret) = secret else {
        return Ok(None);
    };
    match kind {
        SecretKind::Pin => check_pin(secret.expose())?,
        SecretKind::Password => check_password(secret.expose())?,
    }
    Ok(Some(hash_secret(secret.expose())?))
}
