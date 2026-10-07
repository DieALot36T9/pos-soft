//! The Import / Export screen (Req - Import and Export). Imports need `DataImport`,
//! exports need `DataExport`. An import is audited `data.import` and takes a
//! `pre-import` backup first; an export is audited `data.export`.

use std::path::Path;

use pos_core::{AppError, Permission};

use crate::session;
use crate::state::AppState;
use crate::workflows::exports::{self, ExportRequest, FileFormat};
use crate::workflows::import_file::{
    self, ErrorLine, ImportDone, ImportEntity, ImportPreview, ImportProgress, ImportRequest,
};
use crate::workflows::products::Choice;

pub fn import_preview(
    state: &AppState,
    request: &ImportRequest,
) -> Result<ImportPreview, AppError> {
    let user = session::require(state, Permission::DataImport)?;
    import_file::preview(state, &user.id, request)
}

pub fn import_commit(state: &AppState, request: &ImportRequest) -> Result<ImportDone, AppError> {
    let user = session::require(state, Permission::DataImport)?;
    import_file::commit(state, &user.id, request)
}

pub fn import_progress(state: &AppState) -> Result<ImportProgress, AppError> {
    session::require(state, Permission::DataImport)?;
    Ok(state.import_job.progress())
}

/// Stops the running import at its next row. Nothing of it is saved (IO-05).
pub fn import_cancel(state: &AppState) -> Result<(), AppError> {
    session::require(state, Permission::DataImport)?;
    state.import_job.cancel();
    Ok(())
}

pub fn import_errors_save(
    state: &AppState,
    lines: &[ErrorLine],
    file: &str,
) -> Result<(), AppError> {
    session::require(state, Permission::DataImport)?;
    import_file::save_error_file(lines, Path::new(file))
}

pub fn template_download(
    state: &AppState,
    entity: ImportEntity,
    format: FileFormat,
    file: &str,
) -> Result<(), AppError> {
    session::require(state, Permission::DataImport)?;
    exports::template(state, entity, format, Path::new(file))
}

pub fn export_entity(state: &AppState, request: &ExportRequest) -> Result<u32, AppError> {
    let user = session::require(state, Permission::DataExport)?;
    exports::export(state, &user.id, request)
}

/// The price lists to import into or export from. `DataImport` or `DataExport`.
pub fn io_price_lists(state: &AppState) -> Result<Vec<Choice>, AppError> {
    if let Err(refused) = session::require(state, Permission::DataImport) {
        let AppError::PermissionDenied { .. } = refused else {
            return Err(refused);
        };
        session::require(state, Permission::DataExport).map_err(|_| refused)?;
    }
    state.db.read(|conn| {
        Ok(pricing::price_lists(conn)?
            .into_iter()
            .map(|list| Choice {
                id: list.id,
                name: list.name,
            })
            .collect())
    })
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::AppError;
    use tauri::State;

    use crate::commands::on_worker;
    use crate::state::AppState;
    use crate::workflows::exports::{ExportRequest, FileFormat};
    use crate::workflows::import_file::{
        ErrorLine, ImportDone, ImportEntity, ImportPreview, ImportProgress, ImportRequest,
    };
    use crate::workflows::products::Choice;

    #[tauri::command]
    pub async fn import_preview(
        state: State<'_, Arc<AppState>>,
        request: ImportRequest,
    ) -> Result<ImportPreview, AppError> {
        let state = Arc::clone(&state);
        on_worker("import_preview", move || {
            super::import_preview(&state, &request)
        })
        .await
    }

    #[tauri::command]
    pub async fn import_commit(
        state: State<'_, Arc<AppState>>,
        request: ImportRequest,
    ) -> Result<ImportDone, AppError> {
        let state = Arc::clone(&state);
        on_worker("import_commit", move || {
            super::import_commit(&state, &request)
        })
        .await
    }

    #[tauri::command]
    pub async fn import_progress(
        state: State<'_, Arc<AppState>>,
    ) -> Result<ImportProgress, AppError> {
        let state = Arc::clone(&state);
        on_worker("import_progress", move || super::import_progress(&state)).await
    }

    #[tauri::command]
    pub async fn import_cancel(state: State<'_, Arc<AppState>>) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("import_cancel", move || super::import_cancel(&state)).await
    }

    #[tauri::command]
    pub async fn import_errors_save(
        state: State<'_, Arc<AppState>>,
        lines: Vec<ErrorLine>,
        file: String,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("import_errors_save", move || {
            super::import_errors_save(&state, &lines, &file)
        })
        .await
    }

    #[tauri::command]
    pub async fn template_download(
        state: State<'_, Arc<AppState>>,
        entity: ImportEntity,
        format: FileFormat,
        file: String,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("template_download", move || {
            super::template_download(&state, entity, format, &file)
        })
        .await
    }

    #[tauri::command]
    pub async fn export_entity(
        state: State<'_, Arc<AppState>>,
        request: ExportRequest,
    ) -> Result<u32, AppError> {
        let state = Arc::clone(&state);
        on_worker("export_entity", move || {
            super::export_entity(&state, &request)
        })
        .await
    }

    #[tauri::command]
    pub async fn io_price_lists(state: State<'_, Arc<AppState>>) -> Result<Vec<Choice>, AppError> {
        let state = Arc::clone(&state);
        on_worker("io_price_lists", move || super::io_price_lists(&state)).await
    }
}
