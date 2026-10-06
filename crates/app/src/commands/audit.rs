//! The audit log screen (USR-08). Both commands need `AuditView`.

use std::path::Path;

use audit::{AuditFilter, AuditPage};
use pos_core::{AppError, Permission};

use crate::session;
use crate::state::AppState;
use crate::workflows::audit_view::{self, AuditList};

/// One page of events, newest first. Needs `AuditView`.
pub fn audit_query(
    state: &AppState,
    filter: &AuditFilter,
    page: AuditPage,
) -> Result<AuditList, AppError> {
    session::require(state, Permission::AuditView)?;
    audit_view::list(state, filter, page)
}

/// Saves every event the filter finds as an Excel file. Needs `AuditView`.
pub fn audit_export(state: &AppState, filter: &AuditFilter, file: &str) -> Result<u32, AppError> {
    session::require(state, Permission::AuditView)?;
    audit_view::export(state, filter, Path::new(file))
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use audit::{AuditFilter, AuditPage};
    use pos_core::AppError;
    use tauri::State;

    use crate::commands::on_worker;
    use crate::state::AppState;
    use crate::workflows::audit_view::AuditList;

    #[tauri::command]
    pub async fn audit_query(
        state: State<'_, Arc<AppState>>,
        filter: AuditFilter,
        page: AuditPage,
    ) -> Result<AuditList, AppError> {
        let state = Arc::clone(&state);
        on_worker("audit_query", move || {
            super::audit_query(&state, &filter, page)
        })
        .await
    }

    #[tauri::command]
    pub async fn audit_export(
        state: State<'_, Arc<AppState>>,
        filter: AuditFilter,
        file: String,
    ) -> Result<u32, AppError> {
        let state = Arc::clone(&state);
        on_worker("audit_export", move || {
            super::audit_export(&state, &filter, &file)
        })
        .await
    }
}
