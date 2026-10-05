//! The setup wizard's commands (rules SET-01 to SET-12). They work only while setup
//! isn't complete, and need no sign-in: nobody exists yet to sign in.

use std::path::Path;

use pos_core::AppError;
use settings::{BackupSettings, SetupStatus, ShopProfile};

use crate::commands::startup::refuse_when_damaged;
use crate::session;
use crate::state::AppState;
use crate::workflows::backup;
use crate::workflows::setup::{
    self, AdminCreated, AdminStep, HardwareStep, MoneyStep, TaxStep, TillStep,
};

pub fn setup_status(state: &AppState) -> Result<SetupStatus, AppError> {
    refuse_when_damaged(state)?;
    setup::status(state)
}

pub fn setup_save_shop(state: &AppState, shop: &ShopProfile) -> Result<SetupStatus, AppError> {
    setup::save_shop(state, shop)
}

pub fn setup_save_money(state: &AppState, step: &MoneyStep) -> Result<SetupStatus, AppError> {
    setup::save_money(state, step)
}

pub fn setup_save_tax(state: &AppState, step: &TaxStep) -> Result<SetupStatus, AppError> {
    setup::save_tax(state, step)
}

pub fn setup_save_admin(state: &AppState, step: AdminStep) -> Result<AdminCreated, AppError> {
    setup::save_admin(state, step)
}

pub fn setup_save_till(state: &AppState, step: &TillStep) -> Result<SetupStatus, AppError> {
    setup::save_till(state, step)
}

pub fn setup_save_hardware(state: &AppState, step: &HardwareStep) -> Result<SetupStatus, AppError> {
    setup::save_hardware(state, step)
}

pub fn setup_save_backup(state: &AppState, step: &BackupSettings) -> Result<SetupStatus, AppError> {
    setup::save_backup(state, step)
}

pub fn setup_finish(state: &AppState) -> Result<SetupStatus, AppError> {
    setup::finish(state)
}

/// "Restore from a backup" on the welcome screen: moving to a new PC (Setup Wizard).
pub fn setup_restore_backup(state: &AppState, file: &str) -> Result<(), AppError> {
    session::require_setup_open(state)?;
    backup::request_restore(state, Path::new(file), None)
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::AppError;
    use settings::{BackupSettings, SetupStatus, ShopProfile};
    use tauri::{AppHandle, State};

    use crate::commands::on_worker;
    use crate::state::AppState;
    use crate::workflows::setup::{
        AdminCreated, AdminStep, HardwareStep, MoneyStep, TaxStep, TillStep,
    };

    type AppStateRef<'a> = State<'a, Arc<AppState>>;

    #[tauri::command]
    pub async fn setup_status(state: AppStateRef<'_>) -> Result<SetupStatus, AppError> {
        let state = Arc::clone(&state);
        on_worker("setup_status", move || super::setup_status(&state)).await
    }

    #[tauri::command]
    pub async fn setup_save_shop(
        state: AppStateRef<'_>,
        shop: ShopProfile,
    ) -> Result<SetupStatus, AppError> {
        let state = Arc::clone(&state);
        on_worker("setup_save_shop", move || {
            super::setup_save_shop(&state, &shop)
        })
        .await
    }

    #[tauri::command]
    pub async fn setup_save_money(
        state: AppStateRef<'_>,
        step: MoneyStep,
    ) -> Result<SetupStatus, AppError> {
        let state = Arc::clone(&state);
        on_worker("setup_save_money", move || {
            super::setup_save_money(&state, &step)
        })
        .await
    }

    #[tauri::command]
    pub async fn setup_save_tax(
        state: AppStateRef<'_>,
        step: TaxStep,
    ) -> Result<SetupStatus, AppError> {
        let state = Arc::clone(&state);
        on_worker("setup_save_tax", move || {
            super::setup_save_tax(&state, &step)
        })
        .await
    }

    #[tauri::command]
    pub async fn setup_save_admin(
        state: AppStateRef<'_>,
        step: AdminStep,
    ) -> Result<AdminCreated, AppError> {
        let state = Arc::clone(&state);
        on_worker("setup_save_admin", move || {
            super::setup_save_admin(&state, step)
        })
        .await
    }

    #[tauri::command]
    pub async fn setup_save_till(
        state: AppStateRef<'_>,
        step: TillStep,
    ) -> Result<SetupStatus, AppError> {
        let state = Arc::clone(&state);
        on_worker("setup_save_till", move || {
            super::setup_save_till(&state, &step)
        })
        .await
    }

    #[tauri::command]
    pub async fn setup_save_hardware(
        state: AppStateRef<'_>,
        step: HardwareStep,
    ) -> Result<SetupStatus, AppError> {
        let state = Arc::clone(&state);
        on_worker("setup_save_hardware", move || {
            super::setup_save_hardware(&state, &step)
        })
        .await
    }

    #[tauri::command]
    pub async fn setup_save_backup(
        state: AppStateRef<'_>,
        step: BackupSettings,
    ) -> Result<SetupStatus, AppError> {
        let state = Arc::clone(&state);
        on_worker("setup_save_backup", move || {
            super::setup_save_backup(&state, &step)
        })
        .await
    }

    #[tauri::command]
    pub async fn setup_finish(state: AppStateRef<'_>) -> Result<SetupStatus, AppError> {
        let state = Arc::clone(&state);
        on_worker("setup_finish", move || super::setup_finish(&state)).await
    }

    #[tauri::command]
    pub async fn setup_restore_backup(
        app: AppHandle,
        state: AppStateRef<'_>,
        file: String,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("setup_restore_backup", move || {
            super::setup_restore_backup(&state, &file)
        })
        .await?;
        app.restart()
    }
}
