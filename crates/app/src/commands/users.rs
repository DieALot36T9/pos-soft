//! The Users and Roles screens (USR-04, USR-06). Every command needs `UsersManage`.

use auth::UserSummary;
use pos_core::{AppError, Id, Permission};

use crate::session;
use crate::state::AppState;
use crate::workflows::users::{self, RoleInput, RolesView, UserInput};

/// Every user, active or not. Needs `UsersManage`.
pub fn user_list(state: &AppState) -> Result<Vec<UserSummary>, AppError> {
    session::require(state, Permission::UsersManage)?;
    users::list_users(state)
}

/// Adds or changes a user. Needs `UsersManage`.
pub fn user_save(state: &AppState, input: UserInput) -> Result<Id, AppError> {
    let by = session::require(state, Permission::UsersManage)?;
    users::save_user(state, &by, input)
}

/// Every role and the permission checkboxes. Needs `UsersManage`.
pub fn role_list(state: &AppState) -> Result<RolesView, AppError> {
    session::require(state, Permission::UsersManage)?;
    users::list_roles(state)
}

/// Adds or changes a role. Needs `UsersManage`.
pub fn role_save(state: &AppState, input: RoleInput) -> Result<Id, AppError> {
    let by = session::require(state, Permission::UsersManage)?;
    users::save_role(state, &by, input)
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use auth::UserSummary;
    use pos_core::{AppError, Id};
    use tauri::State;

    use crate::commands::on_worker;
    use crate::state::AppState;
    use crate::workflows::users::{RoleInput, RolesView, UserInput};

    #[tauri::command]
    pub async fn user_list(state: State<'_, Arc<AppState>>) -> Result<Vec<UserSummary>, AppError> {
        let state = Arc::clone(&state);
        on_worker("user_list", move || super::user_list(&state)).await
    }

    #[tauri::command]
    pub async fn user_save(
        state: State<'_, Arc<AppState>>,
        input: UserInput,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("user_save", move || super::user_save(&state, input)).await
    }

    #[tauri::command]
    pub async fn role_list(state: State<'_, Arc<AppState>>) -> Result<RolesView, AppError> {
        let state = Arc::clone(&state);
        on_worker("role_list", move || super::role_list(&state)).await
    }

    #[tauri::command]
    pub async fn role_save(
        state: State<'_, Arc<AppState>>,
        input: RoleInput,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("role_save", move || super::role_save(&state, input)).await
    }
}
