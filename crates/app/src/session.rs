//! Logging in, the lock screen, admin approvals, and the permission check every
//! command makes (USR-01 to USR-05, Security Model).
//!
//! The session lives only in memory. Before each permission check the signed-in
//! user is read again from `auth`, so a changed role or a disabled user counts at once.

use auth::{LoginOutcome, OverrideOutcome, SessionUser, UserTile};
use pos_core::{AppError, Bps, Id, Permission, Secret, Timestamp};
use rand::RngExt;
use rusqlite::Transaction;
use serde::Serialize;
use ts_rs::TS;

use crate::audit_actions::{
    APPROVAL_GRANTED, SESSION_LOGIN, SESSION_LOGOUT, SESSION_UNLOCK_BY_ADMIN, USER_LOCKOUT,
};
use crate::audit_log;
use crate::state::{AppState, lock};

/// An admin approval is good for this long (Roles and Permissions: 60 seconds).
pub const APPROVAL_LIFETIME_MS: i64 = 60_000;

/// Unlocking for someone else needs an admin: a user who may manage users.
const ADMIN_UNLOCK_PERMISSION: Permission = Permission::UsersManage;

/// The signed-in user. `locked` is true while the lock screen shows (USR-03).
#[derive(Debug, Clone)]
pub struct Session {
    pub user: SessionUser,
    pub locked: bool,
}

/// One admin approval that hasn't been used yet (USR-05).
#[derive(Debug, Clone)]
pub struct Approval {
    token: String,
    permission: Permission,
    requested_by: Id,
    approved_by: Id,
    expires_at: Timestamp,
}

/// Who is signed in, as the screens see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SessionView {
    pub user_id: Id,
    pub username: String,
    pub display_name: String,
    pub role_name: String,
    pub is_admin: bool,
    /// The till is locked: only the lock screen shows.
    pub locked: bool,
    /// The biggest discount this user may give without an admin.
    pub max_discount: Bps,
    pub permissions: Vec<Permission>,
}

impl SessionView {
    fn of(session: &Session) -> SessionView {
        let user = &session.user;
        SessionView {
            user_id: user.id.clone(),
            username: user.username.clone(),
            display_name: user.display_name.clone(),
            role_name: user.role_name.clone(),
            is_admin: user.is_admin,
            locked: session.locked,
            max_discount: user.max_discount,
            permissions: user.permissions.iter().collect(),
        }
    }
}

/// What a login attempt gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum LoginResult {
    LoggedIn {
        session: SessionView,
    },
    /// Wrong PIN or password. Example: 4 tries left after the first mistake.
    WrongSecret {
        attempts_left: u32,
    },
    /// Too many wrong tries: wait until this time (milliseconds since 1970).
    Locked {
        until_ms: i64,
    },
}

/// What checking a PIN or password gave, when nothing else comes back
/// (unlocking, changing your own PIN or password).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum SecretCheck {
    Accepted,
    WrongSecret {
        attempts_left: u32,
    },
    Locked {
        until_ms: i64,
    },
    /// The password was right, but that user isn't allowed to do this.
    NotAllowed,
}

/// What asking an admin to approve one action gave (USR-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum ApprovalResult {
    /// Send `token` with the action. It works once, within 60 seconds.
    Approved {
        token: String,
    },
    /// The password was right, but that user may not do the action either.
    NotAllowed,
    WrongSecret {
        attempts_left: u32,
    },
    Locked {
        until_ms: i64,
    },
}

/// The user an action runs for, and the admin who approved it, if one had to.
#[derive(Debug, Clone)]
pub struct Allowed {
    pub user: SessionUser,
    pub approved_by: Option<Id>,
}

/// The tiles on the login screen: every active user.
pub fn login_tiles(state: &AppState) -> Result<Vec<UserTile>, AppError> {
    state.db.read(|conn| Ok(auth::login_tiles(conn)?))
}

/// Checks a user's PIN or password and signs them in (USR-01). A lockout is
/// audited (USR-02). While the till is locked, only the user who locked it can
/// get back in, through the lock screen (USR-03).
pub fn login(state: &AppState, user: &Id, secret: String) -> Result<LoginResult, AppError> {
    if let Some(session) = state.session()?.as_ref()
        && session.locked
        && session.user.id != *user
    {
        return Err(conflict("till_locked"));
    }
    let clock = state.clock.as_ref();
    let outcome = state.db.write(|tx| {
        let outcome = auth::verify_login(tx, user, Secret::new(secret), clock.now())?;
        match &outcome {
            LoginOutcome::LoggedIn(who) => audit_log::record(
                tx,
                clock,
                SESSION_LOGIN,
                serde_json::json!({}),
                Some(who.id.clone()),
            )?,
            LoginOutcome::LockedNow { .. } => record_lockout(tx, state, user.clone(), "login")?,
            LoginOutcome::WrongSecret { .. } | LoginOutcome::StillLocked { .. } => {}
        }
        Ok(outcome)
    })?;
    Ok(match outcome {
        LoginOutcome::LoggedIn(who) => {
            let session = Session {
                user: who,
                locked: false,
            };
            let view = SessionView::of(&session);
            *state.session()? = Some(session);
            lock(&state.approvals)?.clear();
            LoginResult::LoggedIn { session: view }
        }
        LoginOutcome::WrongSecret { attempts_left } => LoginResult::WrongSecret { attempts_left },
        LoginOutcome::LockedNow { until } | LoginOutcome::StillLocked { until } => {
            LoginResult::Locked { until_ms: until.0 }
        }
    })
}

/// Signs the current user out, and audits it. Also "Switch user" on the lock screen.
pub fn logout(state: &AppState) -> Result<(), AppError> {
    let Some(session) = state.session()?.take() else {
        return Ok(());
    };
    lock(&state.approvals)?.clear();
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        audit_log::record(
            tx,
            clock,
            SESSION_LOGOUT,
            serde_json::json!({}),
            Some(session.user.id),
        )
    })
}

/// Locks the till (F12 or the idle timer). The user stays signed in (USR-03).
pub fn lock_till(state: &AppState) -> Result<SessionView, AppError> {
    let mut guard = state.session()?;
    let Some(session) = guard.as_mut() else {
        return Err(conflict("not_signed_in"));
    };
    session.locked = true;
    let view = SessionView::of(session);
    drop(guard);
    lock(&state.approvals)?.clear();
    Ok(view)
}

/// The user who locked the till unlocks it with their own PIN or password (USR-03).
pub fn unlock(state: &AppState, secret: String) -> Result<SecretCheck, AppError> {
    let user = locked_user(state)?;
    let clock = state.clock.as_ref();
    let outcome = state.db.write(|tx| {
        let outcome = auth::verify_login(tx, &user, Secret::new(secret), clock.now())?;
        if let LoginOutcome::LockedNow { .. } = outcome {
            record_lockout(tx, state, user.clone(), "unlock")?;
        }
        Ok(outcome)
    })?;
    Ok(match outcome {
        LoginOutcome::LoggedIn(who) => {
            *state.session()? = Some(Session {
                user: who,
                locked: false,
            });
            SecretCheck::Accepted
        }
        LoginOutcome::WrongSecret { attempts_left } => SecretCheck::WrongSecret { attempts_left },
        LoginOutcome::LockedNow { until } | LoginOutcome::StillLocked { until } => {
            SecretCheck::Locked { until_ms: until.0 }
        }
    })
}

/// An admin unlocks the till to help the user who locked it (USR-03). The same
/// user stays signed in; the audit log names the admin.
pub fn unlock_by_admin(
    state: &AppState,
    username: &str,
    password: String,
) -> Result<SecretCheck, AppError> {
    let user = locked_user(state)?;
    let clock = state.clock.as_ref();
    let outcome = state.db.write(|tx| {
        let outcome = auth::verify_override(
            tx,
            username,
            Secret::new(password),
            ADMIN_UNLOCK_PERMISSION,
            clock.now(),
        )?;
        match &outcome {
            OverrideOutcome::Approved { approved_by } => audit_log::record_approved(
                tx,
                clock,
                SESSION_UNLOCK_BY_ADMIN,
                serde_json::json!({}),
                Some(user.clone()),
                Some(approved_by.clone()),
            )?,
            OverrideOutcome::LockedNow { .. } => {
                if let Some(admin) = user_id_by_username(tx, username)? {
                    record_lockout(tx, state, admin, "admin_unlock")?;
                }
            }
            _ => {}
        }
        Ok(outcome)
    })?;
    Ok(match outcome {
        OverrideOutcome::Approved { .. } => {
            if let Some(session) = state.session()?.as_mut() {
                session.locked = false;
            }
            SecretCheck::Accepted
        }
        OverrideOutcome::NotAllowed => SecretCheck::NotAllowed,
        OverrideOutcome::WrongSecret { attempts_left } => {
            SecretCheck::WrongSecret { attempts_left }
        }
        OverrideOutcome::LockedNow { until } | OverrideOutcome::StillLocked { until } => {
            SecretCheck::Locked { until_ms: until.0 }
        }
    })
}

/// An admin types their username and password to approve one action that the
/// signed-in user's role doesn't allow (USR-05). Gives a single-use token for
/// that one permission, good for 60 seconds. The approval is audited.
pub fn request_approval(
    state: &AppState,
    permission: Permission,
    username: &str,
    password: String,
) -> Result<ApprovalResult, AppError> {
    let user = require_signed_in(state)?;
    let clock = state.clock.as_ref();
    let outcome = state.db.write(|tx| {
        let outcome =
            auth::verify_override(tx, username, Secret::new(password), permission, clock.now())?;
        match &outcome {
            OverrideOutcome::Approved { approved_by } => audit_log::record_approved(
                tx,
                clock,
                APPROVAL_GRANTED,
                serde_json::json!({ "permission": permission.as_str() }),
                Some(user.id.clone()),
                Some(approved_by.clone()),
            )?,
            OverrideOutcome::LockedNow { .. } => {
                if let Some(admin) = user_id_by_username(tx, username)? {
                    record_lockout(tx, state, admin, "approval")?;
                }
            }
            _ => {}
        }
        Ok(outcome)
    })?;
    Ok(match outcome {
        OverrideOutcome::Approved { approved_by } => {
            let token = new_token();
            lock(&state.approvals)?.push(Approval {
                token: token.clone(),
                permission,
                requested_by: user.id,
                approved_by,
                expires_at: Timestamp(clock.now().0 + APPROVAL_LIFETIME_MS),
            });
            ApprovalResult::Approved { token }
        }
        OverrideOutcome::NotAllowed => ApprovalResult::NotAllowed,
        OverrideOutcome::WrongSecret { attempts_left } => {
            ApprovalResult::WrongSecret { attempts_left }
        }
        OverrideOutcome::LockedNow { until } | OverrideOutcome::StillLocked { until } => {
            ApprovalResult::Locked { until_ms: until.0 }
        }
    })
}

/// The signed-in user, read again from `auth`. Refuses when nobody is signed in
/// or the till is locked. A user disabled since login is signed out.
pub fn require_signed_in(state: &AppState) -> Result<SessionUser, AppError> {
    let mut guard = state.session()?;
    let Some(session) = guard.as_mut() else {
        return Err(conflict("not_signed_in"));
    };
    if session.locked {
        return Err(conflict("till_locked"));
    }
    match state
        .db
        .read(|conn| Ok(auth::session_user(conn, &session.user.id)?))
    {
        Ok(user) => {
            session.user = user.clone();
            Ok(user)
        }
        Err(AppError::Conflict { code, .. }) if code == "user_inactive" => {
            *guard = None;
            Err(conflict("not_signed_in"))
        }
        Err(other) => Err(other),
    }
}

/// The signed-in user, if they have `permission`. Otherwise `PermissionDenied`.
pub fn require(state: &AppState, permission: Permission) -> Result<SessionUser, AppError> {
    match require_signed_in(state) {
        Ok(user) if user.permissions.contains(permission) => Ok(user),
        Err(AppError::Conflict { code, params }) if code == "till_locked" => {
            Err(AppError::Conflict { code, params })
        }
        _ => Err(AppError::PermissionDenied { permission }),
    }
}

/// Like `require`, but an admin's approval token lets a user without the
/// permission do this one action (USR-05). The token is used up, even when it
/// doesn't fit. Without a fitting token the answer is `OverrideRequired`, and
/// the screen shows the approval dialog.
pub fn require_or_approval(
    state: &AppState,
    permission: Permission,
    approval: Option<&str>,
) -> Result<Allowed, AppError> {
    let user = match require_signed_in(state) {
        Ok(user) => user,
        Err(AppError::Conflict { code, params }) if code == "till_locked" => {
            return Err(AppError::Conflict { code, params });
        }
        Err(_) => return Err(AppError::PermissionDenied { permission }),
    };
    if user.permissions.contains(permission) {
        return Ok(Allowed {
            user,
            approved_by: None,
        });
    }
    let needed = AppError::OverrideRequired { permission };
    let Some(token) = approval else {
        return Err(needed);
    };
    let found = {
        let mut approvals = lock(&state.approvals)?;
        let place = approvals.iter().position(|each| each.token == token);
        place.map(|index| approvals.remove(index))
    };
    let now = state.clock.now();
    match found {
        Some(each)
            if each.permission == permission
                && each.requested_by == user.id
                && now < each.expires_at =>
        {
            Ok(Allowed {
                user,
                approved_by: Some(each.approved_by),
            })
        }
        _ => Err(needed),
    }
}

/// The signed-in user, for the screens. A user disabled since login is signed out.
pub fn current(state: &AppState) -> Result<Option<SessionView>, AppError> {
    let locked = match state.session()?.as_ref() {
        None => return Ok(None),
        Some(session) => session.locked,
    };
    if !locked {
        match require_signed_in(state) {
            Ok(_) => {}
            Err(AppError::Conflict { code, .. }) if code == "not_signed_in" => return Ok(None),
            Err(other) => return Err(other),
        }
    }
    Ok(state.session()?.as_ref().map(SessionView::of))
}

/// Signs a user in without a check. Only for the admin the setup wizard has just made.
pub(crate) fn start_session(state: &AppState, user: &Id) -> Result<SessionView, AppError> {
    let who = state.db.read(|conn| Ok(auth::session_user(conn, user)?))?;
    let session = Session {
        user: who,
        locked: false,
    };
    let view = SessionView::of(&session);
    *state.session()? = Some(session);
    Ok(view)
}

/// During first-run setup nobody can be signed in yet, so the setup screens may
/// change settings without a session. Once setup is complete, `permission` is needed.
/// Returns the signed-in user, or `None` during setup.
pub fn require_or_setup(
    state: &AppState,
    permission: Permission,
) -> Result<Option<SessionUser>, AppError> {
    if setup_is_open(state)? {
        return Ok(state
            .session()?
            .as_ref()
            .map(|session| session.user.clone()));
    }
    require(state, permission).map(Some)
}

/// Refuses once setup is complete: the setup steps can't be run again.
pub fn require_setup_open(state: &AppState) -> Result<(), AppError> {
    if setup_is_open(state)? {
        Ok(())
    } else {
        Err(conflict("setup_already_done"))
    }
}

/// Refuses until setup is complete: logging in needs the shop to exist.
pub fn require_setup_done(state: &AppState) -> Result<(), AppError> {
    if setup_is_open(state)? {
        Err(conflict("setup_not_done"))
    } else {
        Ok(())
    }
}

fn setup_is_open(state: &AppState) -> Result<bool, AppError> {
    let status = state.db.read(|conn| Ok(settings::setup_status(conn)?))?;
    Ok(!status.is_complete)
}

/// The user who locked the till. Refuses when the till isn't locked.
fn locked_user(state: &AppState) -> Result<Id, AppError> {
    match state.session()?.as_ref() {
        Some(session) if session.locked => Ok(session.user.id.clone()),
        _ => Err(conflict("till_not_locked")),
    }
}

/// Writes the lockout into the audit log (USR-02). `during` says what was tried.
fn record_lockout(
    tx: &Transaction,
    state: &AppState,
    user: Id,
    during: &str,
) -> Result<(), AppError> {
    audit_log::record(
        tx,
        state.clock.as_ref(),
        USER_LOCKOUT,
        serde_json::json!({ "during": during }),
        Some(user),
    )
}

/// The id of the active user with this username (capitals don't matter).
fn user_id_by_username(tx: &Transaction, username: &str) -> Result<Option<Id>, AppError> {
    let wanted = username.trim().to_lowercase();
    let tiles = auth::login_tiles(tx)?;
    Ok(tiles
        .into_iter()
        .find(|tile| tile.username.to_lowercase() == wanted)
        .map(|tile| tile.id))
}

/// A random 128-bit approval token, as 32 hex digits.
fn new_token() -> String {
    let number: u128 = rand::rng().random();
    format!("{number:032x}")
}

fn conflict(code: &str) -> AppError {
    AppError::Conflict {
        code: code.into(),
        params: Default::default(),
    }
}
