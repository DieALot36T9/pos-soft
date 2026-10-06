//! Login screen, lock screen, admin approval, your own PIN or password, and the
//! recovery code (USR-01 to USR-03, USR-05, USR-07, USR-09).

use auth::{SecretKind, UserTile};
use pos_core::{AppError, Id, Permission};

use crate::commands::startup::refuse_when_damaged;
use crate::session::{self, ApprovalResult, LoginResult, SecretCheck, SessionView};
use crate::state::AppState;
use crate::workflows::users::{self, RecoveryDone};

/// The users on the login screen. Needs no login, but setup must be done.
pub fn login_tiles(state: &AppState) -> Result<Vec<UserTile>, AppError> {
    ready(state)?;
    session::login_tiles(state)
}

/// Logs a user in with their PIN or password. Needs no login.
pub fn login(state: &AppState, user_id: &Id, secret: String) -> Result<LoginResult, AppError> {
    ready(state)?;
    session::login(state, user_id, secret)
}

/// Logs out. Needs no permission.
pub fn logout(state: &AppState) -> Result<(), AppError> {
    session::logout(state)
}

/// Locks the till. Needs someone signed in.
pub fn lock(state: &AppState) -> Result<SessionView, AppError> {
    session::lock_till(state)
}

/// Unlocks with the locked user's own PIN or password.
pub fn unlock(state: &AppState, secret: String) -> Result<SecretCheck, AppError> {
    session::unlock(state, secret)
}

/// An admin unlocks the till for the locked user.
pub fn unlock_by_admin(
    state: &AppState,
    username: &str,
    password: String,
) -> Result<SecretCheck, AppError> {
    session::unlock_by_admin(state, username, password)
}

/// An admin approves one action. Needs someone signed in.
pub fn request_override(
    state: &AppState,
    permission: Permission,
    username: &str,
    password: String,
) -> Result<ApprovalResult, AppError> {
    session::request_approval(state, permission, username, password)
}

/// Changes your own PIN or password. Needs someone signed in.
pub fn change_own_secret(
    state: &AppState,
    kind: SecretKind,
    current: String,
    new_secret: String,
) -> Result<SecretCheck, AppError> {
    let user = session::require_signed_in(state)?;
    users::change_own_secret(state, &user, kind, current, new_secret)
}

/// "Forgot admin password". Needs no login, but setup must be done.
pub fn recover_admin(
    state: &AppState,
    code: String,
    username: &str,
    password: String,
    password_again: &str,
) -> Result<RecoveryDone, AppError> {
    ready(state)?;
    users::recover_admin(state, code, username, password, password_again)
}

/// Logging in needs a working database and a finished setup.
fn ready(state: &AppState) -> Result<(), AppError> {
    refuse_when_damaged(state)?;
    session::require_setup_done(state)
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use auth::{SecretKind, UserTile};
    use pos_core::{AppError, Id, Permission};
    use tauri::State;

    use crate::commands::on_worker;
    use crate::session::{ApprovalResult, LoginResult, SecretCheck, SessionView};
    use crate::state::AppState;
    use crate::workflows::users::RecoveryDone;

    #[tauri::command]
    pub async fn login_tiles(state: State<'_, Arc<AppState>>) -> Result<Vec<UserTile>, AppError> {
        let state = Arc::clone(&state);
        on_worker("login_tiles", move || super::login_tiles(&state)).await
    }

    #[tauri::command]
    pub async fn login(
        state: State<'_, Arc<AppState>>,
        user_id: Id,
        secret: String,
    ) -> Result<LoginResult, AppError> {
        let state = Arc::clone(&state);
        on_worker("login", move || super::login(&state, &user_id, secret)).await
    }

    #[tauri::command]
    pub async fn logout(state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("logout", move || super::logout(&state)).await
    }

    #[tauri::command]
    pub fn lock(state: State<'_, Arc<AppState>>) -> Result<SessionView, AppError> {
        super::lock(&state)
    }

    #[tauri::command]
    pub async fn unlock(
        state: State<'_, Arc<AppState>>,
        secret: String,
    ) -> Result<SecretCheck, AppError> {
        let state = Arc::clone(&state);
        on_worker("unlock", move || super::unlock(&state, secret)).await
    }

    #[tauri::command]
    pub async fn unlock_by_admin(
        state: State<'_, Arc<AppState>>,
        username: String,
        password: String,
    ) -> Result<SecretCheck, AppError> {
        let state = Arc::clone(&state);
        on_worker("unlock_by_admin", move || {
            super::unlock_by_admin(&state, &username, password)
        })
        .await
    }

    #[tauri::command]
    pub async fn request_override(
        state: State<'_, Arc<AppState>>,
        permission: Permission,
        username: String,
        password: String,
    ) -> Result<ApprovalResult, AppError> {
        let state = Arc::clone(&state);
        on_worker("request_override", move || {
            super::request_override(&state, permission, &username, password)
        })
        .await
    }

    #[tauri::command]
    pub async fn change_own_secret(
        state: State<'_, Arc<AppState>>,
        kind: SecretKind,
        current: String,
        new_secret: String,
    ) -> Result<SecretCheck, AppError> {
        let state = Arc::clone(&state);
        on_worker("change_own_secret", move || {
            super::change_own_secret(&state, kind, current, new_secret)
        })
        .await
    }

    #[tauri::command]
    pub async fn recover_admin(
        state: State<'_, Arc<AppState>>,
        code: String,
        username: String,
        password: String,
        password_again: String,
    ) -> Result<RecoveryDone, AppError> {
        let state = Arc::clone(&state);
        on_worker("recover_admin", move || {
            super::recover_admin(&state, code, &username, password, &password_again)
        })
        .await
    }
}
