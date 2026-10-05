//! The Backups screen (rules BAK-04, BAK-05).

use std::path::Path;

use pos_core::{AppError, Permission};

use crate::audit_actions::BACKUP_MANUAL;
use crate::audit_log;
use crate::session;
use crate::state::AppState;
use crate::workflows::backup::{self, BackupDone, BackupRow};

/// "Back up now". Needs `BackupRun`.
pub fn backup_now(state: &AppState) -> Result<BackupDone, AppError> {
    let user = session::require(state, Permission::BackupRun)?;
    let done = backup::make_backup(state, "manual")?;
    let clock = state.clock.as_ref();
    let details = serde_json::json!({ "file": done.file });
    state
        .db
        .write(|tx| audit_log::record(tx, clock, BACKUP_MANUAL, details, Some(user.id.clone())))?;
    Ok(done)
}

/// The backups in the main folder, newest first. Needs `BackupRun`.
pub fn backup_list(state: &AppState) -> Result<Vec<BackupRow>, AppError> {
    session::require(state, Permission::BackupRun)?;
    let options = backup::backup_settings(state)?;
    backup::list_backups(Path::new(&options.primary_dir))
}

/// Restores a backup at the next start (the command then restarts). Needs `BackupRestore`.
pub fn backup_restore(state: &AppState, file: &str) -> Result<(), AppError> {
    let user = session::require(state, Permission::BackupRestore)?;
    backup::request_restore(state, Path::new(file), Some(user.id))
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::AppError;
    use tauri::{AppHandle, State};

    use crate::commands::on_worker;
    use crate::state::AppState;
    use crate::workflows::backup::{BackupDone, BackupRow};

    #[tauri::command]
    pub async fn backup_now(state: State<'_, Arc<AppState>>) -> Result<BackupDone, AppError> {
        let state = Arc::clone(&state);
        on_worker("backup_now", move || super::backup_now(&state)).await
    }

    #[tauri::command]
    pub async fn backup_list(state: State<'_, Arc<AppState>>) -> Result<Vec<BackupRow>, AppError> {
        let state = Arc::clone(&state);
        on_worker("backup_list", move || super::backup_list(&state)).await
    }

    #[tauri::command]
    pub async fn backup_restore(
        app: AppHandle,
        state: State<'_, Arc<AppState>>,
        file: String,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("backup_restore", move || {
            super::backup_restore(&state, &file)
        })
        .await?;
        app.restart()
    }
}
