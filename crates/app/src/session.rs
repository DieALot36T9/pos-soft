//! Signing in and the permission check every command makes (Security Model).
//! Until the login screens arrive (C-M3), an admin signs in with username and password.

use auth::{LoginOutcome, SessionUser};
use pos_core::{AppError, Permission, Secret};
use serde::Serialize;
use ts_rs::TS;

use crate::state::AppState;

/// Who is signed in, as the screens see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SessionView {
    pub username: String,
    pub display_name: String,
    pub is_admin: bool,
    pub permissions: Vec<Permission>,
}

impl SessionView {
    fn of(user: &SessionUser) -> SessionView {
        SessionView {
            username: user.username.clone(),
            display_name: user.display_name.clone(),
            is_admin: user.is_admin,
            permissions: user.permissions.iter().collect(),
        }
    }
}

/// What a sign-in attempt gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum SignInResult {
    SignedIn {
        session: SessionView,
    },
    /// Wrong username or password. The username isn't confirmed, so it can't be guessed.
    Refused {
        attempts_left: Option<u32>,
    },
    /// Too many wrong tries: wait until this time (milliseconds since 1970).
    Locked {
        until_ms: i64,
    },
}

/// Checks a username and password (or PIN) with `auth` and signs the user in.
pub fn sign_in(state: &AppState, username: &str, secret: String) -> Result<SignInResult, AppError> {
    let now = state.clock.now();
    let tiles = state.db.read(|conn| Ok(auth::login_tiles(conn)?))?;
    let wanted = username.trim().to_lowercase();
    let Some(tile) = tiles
        .iter()
        .find(|tile| tile.username.to_lowercase() == wanted)
    else {
        return Ok(SignInResult::Refused {
            attempts_left: None,
        });
    };
    let outcome = state
        .db
        .write(|tx| Ok(auth::verify_login(tx, &tile.id, Secret::new(secret), now)?))?;
    Ok(match outcome {
        LoginOutcome::LoggedIn(user) => {
            let view = SessionView::of(&user);
            *state.session()? = Some(user);
            SignInResult::SignedIn { session: view }
        }
        LoginOutcome::WrongSecret { attempts_left } => SignInResult::Refused {
            attempts_left: Some(attempts_left),
        },
        LoginOutcome::LockedNow { until } | LoginOutcome::StillLocked { until } => {
            SignInResult::Locked { until_ms: until.0 }
        }
    })
}

/// Signs the current user out.
pub fn sign_out(state: &AppState) -> Result<(), AppError> {
    *state.session()? = None;
    Ok(())
}

/// The signed-in user, for the screens.
pub fn current(state: &AppState) -> Result<Option<SessionView>, AppError> {
    Ok(state.session()?.as_ref().map(SessionView::of))
}

/// The signed-in user, if they have `permission`. Otherwise `PermissionDenied`.
pub fn require(state: &AppState, permission: Permission) -> Result<SessionUser, AppError> {
    match state.session()?.as_ref() {
        Some(user) if user.permissions.contains(permission) => Ok(user.clone()),
        _ => Err(AppError::PermissionDenied { permission }),
    }
}

/// During first-run setup nobody can be signed in yet, so the setup screens may
/// change settings without a session. Once setup is complete, `permission` is needed.
/// Returns the signed-in user, or `None` during setup.
pub fn require_or_setup(
    state: &AppState,
    permission: Permission,
) -> Result<Option<SessionUser>, AppError> {
    if setup_is_open(state)? {
        return Ok(state.session()?.clone());
    }
    require(state, permission).map(Some)
}

/// Refuses once setup is complete: the setup steps can't be run again.
pub fn require_setup_open(state: &AppState) -> Result<(), AppError> {
    if setup_is_open(state)? {
        Ok(())
    } else {
        Err(AppError::Conflict {
            code: "setup_already_done".into(),
            params: Default::default(),
        })
    }
}

fn setup_is_open(state: &AppState) -> Result<bool, AppError> {
    let status = state.db.read(|conn| Ok(settings::setup_status(conn)?))?;
    Ok(!status.is_complete)
}
