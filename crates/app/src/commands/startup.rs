//! Startup, who is signed in, and the recovery screen (rules SET-01, BAK-06).

use std::path::Path;

use pos_core::AppError;
use serde::Serialize;
use settings::{CheckoutSettings, SetupStatus, SetupStep};
use ts_rs::TS;

use crate::session::{self, SessionView};
use crate::state::{AppState, DbHealth, lock};
use crate::workflows::backup::{self, BackupRow};

/// What the screens need to know first: which screen to open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct StartupView {
    /// True when the database failed its check: only the recovery screen works (BAK-06).
    pub damaged: bool,
    pub setup: SetupStatus,
    pub session: Option<SessionView>,
    /// The last backup problem, as an error code for the warning banner (BAK-02, BAK-03).
    pub backup_warning: Option<String>,
    /// Lock the till after this many minutes without a key or mouse press (USR-03).
    pub idle_lock_min: u32,
}

/// Which screen to show. Needs no permission.
pub fn startup_status(state: &AppState) -> Result<StartupView, AppError> {
    let damaged = state.health == DbHealth::Damaged;
    let (setup, checkout) = if damaged {
        let nothing = SetupStatus {
            completed: Vec::new(),
            next_step: Some(SetupStep::Shop),
            is_complete: false,
        };
        (nothing, CheckoutSettings::default())
    } else {
        let checkout: CheckoutSettings = state.db.read(|conn| Ok(settings::get(conn)?))?;
        (crate::workflows::setup::status(state)?, checkout)
    };
    Ok(StartupView {
        damaged,
        setup,
        session: session::current(state)?,
        backup_warning: lock(&state.backup_warning)?.clone(),
        idle_lock_min: checkout.idle_lock_min,
    })
}

/// The backups the recovery screen offers, newest first. Only while the database is damaged.
pub fn recovery_backups(state: &AppState) -> Result<Vec<BackupRow>, AppError> {
    require_damaged(state)?;
    backup::list_backups(&state.paths.backups())
}

/// Asks for a backup to be restored at the next start. Only while the database is damaged.
pub fn recovery_restore(state: &AppState, file: &str) -> Result<(), AppError> {
    require_damaged(state)?;
    backup::request_restore(state, Path::new(file), None, None)
}

fn require_damaged(state: &AppState) -> Result<(), AppError> {
    if state.health == DbHealth::Damaged {
        Ok(())
    } else {
        Err(conflict("database_not_damaged"))
    }
}

pub(crate) fn refuse_when_damaged(state: &AppState) -> Result<(), AppError> {
    if state.health == DbHealth::Damaged {
        Err(conflict("database_damaged"))
    } else {
        Ok(())
    }
}

pub(crate) fn conflict(code: &str) -> AppError {
    AppError::Conflict {
        code: code.into(),
        params: Default::default(),
    }
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::AppError;
    use tauri::{AppHandle, State};

    use super::StartupView;
    use crate::commands::on_worker;
    use crate::session::SessionView;
    use crate::state::AppState;
    use crate::workflows::backup::BackupRow;

    #[tauri::command]
    pub async fn startup_status(state: State<'_, Arc<AppState>>) -> Result<StartupView, AppError> {
        let state = Arc::clone(&state);
        on_worker("startup_status", move || super::startup_status(&state)).await
    }

    #[tauri::command]
    pub async fn session_current(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Option<SessionView>, AppError> {
        let state = Arc::clone(&state);
        on_worker("session_current", move || crate::session::current(&state)).await
    }

    #[tauri::command]
    pub async fn recovery_backups(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<BackupRow>, AppError> {
        let state = Arc::clone(&state);
        on_worker("recovery_backups", move || super::recovery_backups(&state)).await
    }

    #[tauri::command]
    pub async fn recovery_restore(
        app: AppHandle,
        state: State<'_, Arc<AppState>>,
        file: String,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("recovery_restore", move || {
            super::recovery_restore(&state, &file)
        })
        .await?;
        app.restart()
    }
}
