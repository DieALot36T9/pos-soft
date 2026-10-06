//! Logging in and admin approvals, with the lockout (USR-01, USR-02, USR-05).
//!
//! A wrong secret is an *outcome*, not an error. The failure count must be saved,
//! so `app` commits the transaction for every outcome. Errors are only for calls
//! that change nothing (unknown or disabled user).

use pos_core::{Bps, Id, Permission, Secret, Timestamp};
use rusqlite::{Connection, Transaction};
use serde::Serialize;
use ts_rs::TS;

use crate::error::AuthError;
use crate::hashing::secret_matches;
use crate::roles::{ADMIN_ROLE_ID, PermissionSet, role_info};
use crate::rules::{LockState, MAX_FAILED_ATTEMPTS, after_failure, locked_until};
use crate::users::{UserRow, find_user_by_username, load_user, save_lock};

/// The logged-in user, kept in `app` memory for the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SessionUser {
    pub id: Id,
    pub username: String,
    pub display_name: String,
    pub role_id: Id,
    pub role_name: String,
    pub is_admin: bool,
    /// The biggest discount this user may give without an admin.
    pub max_discount: Bps,
    pub permissions: PermissionSet,
}

/// What happened when someone tried to log in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum LoginOutcome {
    LoggedIn(SessionUser),
    /// Wrong PIN or password. Example: 4 tries left after the first mistake.
    WrongSecret {
        attempts_left: u32,
    },
    /// This try was the fifth mistake: the user is locked until `until`. `app` audits it.
    LockedNow {
        until: Timestamp,
    },
    /// The user was already locked. Nothing was checked.
    StillLocked {
        until: Timestamp,
    },
}

/// What happened when an admin tried to approve one action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverrideOutcome {
    /// The action may run once. `app` makes the single-use token and audits `approved_by`.
    Approved {
        approved_by: Id,
    },
    /// The password was right, but this user may not do the action either.
    NotAllowed,
    WrongSecret {
        attempts_left: u32,
    },
    LockedNow {
        until: Timestamp,
    },
    StillLocked {
        until: Timestamp,
    },
}

/// The result of checking a secret, after the lockout rules ran.
enum Attempt {
    Matched,
    Wrong { attempts_left: u32 },
    LockedNow { until: Timestamp },
    StillLocked { until: Timestamp },
}

/// Checks a user's PIN or password (either one works) and counts failures:
/// 5 wrong tries in a row lock the user for 60 seconds (USR-01, USR-02).
pub fn verify_login(
    transaction: &Transaction,
    user: &Id,
    secret: Secret<String>,
    now: Timestamp,
) -> Result<LoginOutcome, AuthError> {
    let row = load_user(transaction, user)?;
    if !row.active {
        return Err(AuthError::UserInactive);
    }
    let hashes = [row.pin_hash.as_deref(), row.password_hash.as_deref()];
    let outcome = match attempt(transaction, &row, &hashes, &secret, now)? {
        Attempt::Matched => LoginOutcome::LoggedIn(session_user_of(transaction, row)?),
        Attempt::Wrong { attempts_left } => LoginOutcome::WrongSecret { attempts_left },
        Attempt::LockedNow { until } => LoginOutcome::LockedNow { until },
        Attempt::StillLocked { until } => LoginOutcome::StillLocked { until },
    };
    Ok(outcome)
}

/// Checks an admin approval: the approver's username and **password** (not PIN),
/// and that the approver has the needed permission (USR-05). Wrong passwords
/// count toward the approver's lockout, like a login.
pub fn verify_override(
    transaction: &Transaction,
    username: &str,
    secret: Secret<String>,
    needed: Permission,
    now: Timestamp,
) -> Result<OverrideOutcome, AuthError> {
    let Some(row) = find_user_by_username(transaction, username)? else {
        return Err(AuthError::OverrideUserUnknown);
    };
    if !row.active {
        return Err(AuthError::OverrideUserUnknown);
    }
    // Rule from the product owner: admin-only actions need the admin password.
    let hashes = [row.password_hash.as_deref()];
    let outcome = match attempt(transaction, &row, &hashes, &secret, now)? {
        Attempt::Matched => {
            let role = role_info(transaction, &row.role_id)?;
            if role.permissions.contains(needed) {
                OverrideOutcome::Approved {
                    approved_by: row.id,
                }
            } else {
                OverrideOutcome::NotAllowed
            }
        }
        Attempt::Wrong { attempts_left } => OverrideOutcome::WrongSecret { attempts_left },
        Attempt::LockedNow { until } => OverrideOutcome::LockedNow { until },
        Attempt::StillLocked { until } => OverrideOutcome::StillLocked { until },
    };
    Ok(outcome)
}

/// Checks the secret against any of the user's stored hashes and saves the new lock state.
fn attempt(
    transaction: &Transaction,
    row: &UserRow,
    hashes: &[Option<&str>],
    secret: &Secret<String>,
    now: Timestamp,
) -> Result<Attempt, AuthError> {
    if let Some(until) = locked_until(row.lock, now) {
        return Ok(Attempt::StillLocked { until });
    }
    let mut matched = false;
    for hash in hashes.iter().flatten() {
        if secret_matches(secret.expose(), hash)? {
            matched = true;
            break;
        }
    }
    if matched {
        save_lock(transaction, &row.id, LockState::CLEAR)?;
        return Ok(Attempt::Matched);
    }
    let lock = after_failure(row.lock, now);
    save_lock(transaction, &row.id, lock)?;
    match lock.locked_until {
        Some(until) => Ok(Attempt::LockedNow { until }),
        None => Ok(Attempt::Wrong {
            attempts_left: MAX_FAILED_ATTEMPTS - lock.failed_attempts,
        }),
    }
}

/// Reads a signed-in user again, with their role as it is now. `app` calls this
/// before each permission check, so a changed role or a disabled user counts at once.
pub fn session_user(connection: &Connection, user: &Id) -> Result<SessionUser, AuthError> {
    let row = load_user(connection, user)?;
    if !row.active {
        return Err(AuthError::UserInactive);
    }
    session_user_of(connection, row)
}

fn session_user_of(connection: &Connection, row: UserRow) -> Result<SessionUser, AuthError> {
    let role = role_info(connection, &row.role_id)?;
    let role_id = Id::parse(&row.role_id).map_err(|_| AuthError::RoleNotFound {
        role: row.role_id.clone(),
    })?;
    Ok(SessionUser {
        id: row.id,
        username: row.username,
        display_name: row.display_name,
        is_admin: row.role_id == ADMIN_ROLE_ID,
        role_id,
        role_name: role.name,
        max_discount: role.max_discount,
        permissions: role.permissions,
    })
}
