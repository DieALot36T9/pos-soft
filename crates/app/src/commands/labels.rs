//! Barcode labels and shelf tags (CAT-12, CAT-13). Everything needs `LabelsPrint`.

use pos_core::{AppError, Permission};

use crate::devices::Devices;
use crate::session;
use crate::state::AppState;
use crate::workflows::labels::{
    self, LabelPick, LabelPicked, LabelPicture, LabelRequest, LabelsPrinted,
};

pub fn labels_pick(state: &AppState, pick: &LabelPick) -> Result<LabelPicked, AppError> {
    session::require(state, Permission::LabelsPrint)?;
    labels::pick(state, pick)
}

pub fn labels_preview(
    state: &AppState,
    devices: &Devices,
    request: &LabelRequest,
) -> Result<Vec<LabelPicture>, AppError> {
    session::require(state, Permission::LabelsPrint)?;
    labels::preview(state, devices, request)
}

pub fn labels_print(
    state: &AppState,
    devices: &Devices,
    request: &LabelRequest,
) -> Result<LabelsPrinted, AppError> {
    session::require(state, Permission::LabelsPrint)?;
    labels::print(state, devices, request)
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::AppError;
    use tauri::State;

    use crate::commands::on_worker;
    use crate::devices::Devices;
    use crate::state::AppState;
    use crate::workflows::labels::{
        LabelPick, LabelPicked, LabelPicture, LabelRequest, LabelsPrinted,
    };

    #[tauri::command]
    pub async fn labels_pick(
        state: State<'_, Arc<AppState>>,
        pick: LabelPick,
    ) -> Result<LabelPicked, AppError> {
        let state = Arc::clone(&state);
        on_worker("labels_pick", move || super::labels_pick(&state, &pick)).await
    }

    #[tauri::command]
    pub async fn labels_preview(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        request: LabelRequest,
    ) -> Result<Vec<LabelPicture>, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("labels_preview", move || {
            super::labels_preview(&state, &devices, &request)
        })
        .await
    }

    #[tauri::command]
    pub async fn labels_print(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        request: LabelRequest,
    ) -> Result<LabelsPrinted, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("labels_print", move || {
            super::labels_print(&state, &devices, &request)
        })
        .await
    }
}
