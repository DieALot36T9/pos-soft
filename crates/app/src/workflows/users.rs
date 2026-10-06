//! Users and roles screens, changing your own PIN or password, and the recovery
//! code (USR-04, USR-06, USR-07, USR-09). Each workflow is one transaction with its
//! audit event, so a refused change leaves nothing behind.

use auth::{
    LoginOutcome, NewRole, NewUser, RoleSummary, SecretKind, SessionUser, UserSummary, UserUpdate,
};
use pos_core::{AppError, Bps, Id, Permission, Secret};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::audit_actions::{
    RECOVERY_USED, ROLE_CHANGE, ROLE_CREATE, USER_CHANGE, USER_CREATE, USER_LOCKOUT,
    USER_OWN_SECRET_CHANGE,
};
use crate::audit_log;
use crate::session::SecretCheck;
use crate::state::AppState;

/// The "Add user" / "Change user" form. `id` is empty for a new user.
/// A PIN or password left empty stays as it is (for a new user: not set).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct UserInput {
    pub id: Option<Id>,
    /// Only used for a new user: usernames can't change.
    pub username: String,
    pub display_name: String,
    pub role_id: Id,
    /// False disables the user. Users are never deleted, because they have history.
    pub active: bool,
    #[ts(type = "string | null")]
    pub pin: Option<Secret<String>>,
    #[ts(type = "string | null")]
    pub password: Option<Secret<String>>,
}

/// The "Add role" / "Change role" form. `id` is empty for a new role.
/// The name of an existing role doesn't change.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct RoleInput {
    pub id: Option<Id>,
    pub name: String,
    pub permissions: Vec<Permission>,
    /// The biggest discount without an admin. Example: Bps(1000) is 10%.
    pub max_discount: Bps,
}

/// The roles screen: every role, and every permission there is, for the checkboxes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RolesView {
    pub roles: Vec<RoleSummary>,
    pub all_permissions: Vec<Permission>,
}

/// What the recovery form gives back: the new recovery code, shown once (USR-09).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RecoveryDone {
    pub recovery_code: String,
}

/// Every user, active or not.
pub fn list_users(state: &AppState) -> Result<Vec<UserSummary>, AppError> {
    state.db.read(|conn| Ok(auth::list_users(conn)?))
}

/// Every role with the permission checkboxes.
pub fn list_roles(state: &AppState) -> Result<RolesView, AppError> {
    let roles = state.db.read(|conn| Ok(auth::list_roles(conn)?))?;
    Ok(RolesView {
        roles,
        all_permissions: Permission::ALL.to_vec(),
    })
}

/// Adds or changes a user, resets their PIN or password if one is typed, and
/// audits it (USR-06). The last active admin can't be disabled or demoted.
pub fn save_user(state: &AppState, by: &SessionUser, input: UserInput) -> Result<Id, AppError> {
    let now = state.clock.now();
    let clock = state.clock.as_ref();
    let pin = typed(input.pin);
    let password = typed(input.password);
    let pin_reset = pin.is_some();
    let password_reset = password.is_some();
    state.db.write(|tx| {
        let (id, action) = match &input.id {
            None => {
                let new_user = NewUser {
                    username: input.username.clone(),
                    display_name: input.display_name.clone(),
                    role_id: input.role_id.clone(),
                    password,
                    pin,
                };
                let id = auth::create_user(tx, new_user, now)?;
                if !input.active {
                    let off = UserUpdate {
                        active: Some(false),
                        ..UserUpdate::default()
                    };
                    auth::update_user(tx, &id, off, now)?;
                }
                (id, USER_CREATE)
            }
            Some(id) => {
                let change = UserUpdate {
                    display_name: Some(input.display_name.clone()),
                    role_id: Some(input.role_id.clone()),
                    active: Some(input.active),
                };
                auth::update_user(tx, id, change, now)?;
                if let Some(pin) = pin {
                    auth::set_secret(tx, id, SecretKind::Pin, pin, now)?;
                }
                if let Some(password) = password {
                    auth::set_secret(tx, id, SecretKind::Password, password, now)?;
                }
                (id.clone(), USER_CHANGE)
            }
        };
        let details = serde_json::json!({
            "user_id": id.as_str(),
            "display_name": input.display_name.trim(),
            "role_id": input.role_id.as_str(),
            "active": input.active,
            "pin_reset": pin_reset,
            "password_reset": password_reset,
        });
        audit_log::record(tx, clock, action, details, Some(by.id.clone()))?;
        Ok(id)
    })
}

/// Adds a custom role, or changes a role's permissions and discount limit, and
/// audits it (USR-04). The Admin role can't be changed.
pub fn save_role(state: &AppState, by: &SessionUser, input: RoleInput) -> Result<Id, AppError> {
    let now = state.clock.now();
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let (id, action) = match &input.id {
            None => {
                let role = NewRole {
                    name: input.name.clone(),
                    permissions: input.permissions.clone(),
                    max_discount: input.max_discount,
                };
                (auth::create_role(tx, role, now)?, ROLE_CREATE)
            }
            Some(id) => {
                auth::set_role_permissions(tx, id, &input.permissions, input.max_discount)?;
                (id.clone(), ROLE_CHANGE)
            }
        };
        let permissions: Vec<&str> = input.permissions.iter().map(|p| p.as_str()).collect();
        let details = serde_json::json!({
            "role_id": id.as_str(),
            "name": input.name.trim(),
            "permissions": permissions,
            "max_discount_bps": input.max_discount.0,
        });
        audit_log::record(tx, clock, action, details, Some(by.id.clone()))?;
        Ok(id)
    })
}

/// A user changes their own PIN or password after typing the current one (USR-07).
/// Wrong tries count toward the lockout, like a login.
pub fn change_own_secret(
    state: &AppState,
    user: &SessionUser,
    kind: SecretKind,
    current: String,
    new_secret: String,
) -> Result<SecretCheck, AppError> {
    let now = state.clock.now();
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let outcome = auth::verify_login(tx, &user.id, Secret::new(current), now)?;
        Ok(match outcome {
            LoginOutcome::LoggedIn(_) => {
                auth::set_secret(tx, &user.id, kind, Secret::new(new_secret), now)?;
                let details = serde_json::json!({ "kind": kind_name(kind) });
                audit_log::record(
                    tx,
                    clock,
                    USER_OWN_SECRET_CHANGE,
                    details,
                    Some(user.id.clone()),
                )?;
                SecretCheck::Accepted
            }
            LoginOutcome::WrongSecret { attempts_left } => {
                SecretCheck::WrongSecret { attempts_left }
            }
            LoginOutcome::LockedNow { until } => {
                let details = serde_json::json!({ "during": "change_own_secret" });
                audit_log::record(tx, clock, USER_LOCKOUT, details, Some(user.id.clone()))?;
                SecretCheck::Locked { until_ms: until.0 }
            }
            LoginOutcome::StillLocked { until } => SecretCheck::Locked { until_ms: until.0 },
        })
    })
}

/// "Forgot admin password": the recovery code sets a new password for one active
/// admin, and a new code replaces the old one (USR-09). Audited.
pub fn recover_admin(
    state: &AppState,
    code: String,
    username: &str,
    new_password: String,
    new_password_again: &str,
) -> Result<RecoveryDone, AppError> {
    if new_password != new_password_again {
        return Err(field_error("password_again", "passwords_differ"));
    }
    let wanted = username.trim().to_lowercase();
    let users = list_users(state)?;
    let Some(admin) = users
        .iter()
        .find(|user| user.username.to_lowercase() == wanted)
    else {
        return Err(field_error("username", "recovery_user_not_admin"));
    };
    let now = state.clock.now();
    let clock = state.clock.as_ref();
    let new_code = state.db.write(|tx| {
        let new_code = auth::reset_with_recovery(
            tx,
            Secret::new(code),
            &admin.id,
            Secret::new(new_password),
            now,
        )?;
        audit_log::record(
            tx,
            clock,
            RECOVERY_USED,
            serde_json::json!({}),
            Some(admin.id.clone()),
        )?;
        Ok(new_code)
    })?;
    Ok(RecoveryDone {
        recovery_code: new_code.expose().clone(),
    })
}

/// An empty box means "not typed".
fn typed(secret: Option<Secret<String>>) -> Option<Secret<String>> {
    secret.filter(|each| !each.expose().is_empty())
}

fn kind_name(kind: SecretKind) -> &'static str {
    match kind {
        SecretKind::Pin => "pin",
        SecretKind::Password => "password",
    }
}

fn field_error(field: &str, code: &str) -> AppError {
    AppError::Validation {
        field: Some(field.into()),
        code: code.into(),
        params: Default::default(),
    }
}
