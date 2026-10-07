//! The Reports screen (Req - Reports). Every report needs `ReportsViewAll`, except
//! that a user with `ShiftOpenCloseOwn` may see the X and Z reports of their own
//! shifts (RPT-10). Reports only read; nothing here is audited.

use std::path::Path;

use pos_core::{AppError, Permission, ReportTable};

use crate::devices::Devices;
use auth::SessionUser;

use crate::session;
use crate::state::AppState;
use crate::workflows::products::Choice;
use crate::workflows::receipts::{self, PrintResult};
use crate::workflows::report_runs::{self, ReportChoices, ReportRequest, ShiftList};

pub fn report_choices(state: &AppState) -> Result<ReportChoices, AppError> {
    session::require(state, Permission::ReportsViewAll)?;
    report_runs::choices(state)
}

pub fn report_customer_search(state: &AppState, text: &str) -> Result<Vec<Choice>, AppError> {
    session::require(state, Permission::ReportsViewAll)?;
    report_runs::find_customers(state, text)
}

/// Every shift for `ReportsViewAll`; otherwise only the user's own (RPT-10).
pub fn report_shifts(state: &AppState) -> Result<ShiftList, AppError> {
    if sees_all(state)?.is_some() {
        return report_runs::shifts(state, None);
    }
    let user = session::require(state, Permission::ShiftOpenCloseOwn)?;
    report_runs::shifts(state, Some(&user.id))
}

pub fn report_run(state: &AppState, request: &ReportRequest) -> Result<ReportTable, AppError> {
    may_see(state, request)?;
    report_runs::run(state, request)
}

pub fn report_export_xlsx(
    state: &AppState,
    request: &ReportRequest,
    file: &str,
) -> Result<(), AppError> {
    may_see(state, request)?;
    report_runs::export_xlsx(state, request, Path::new(file))
}

pub fn report_print(
    state: &AppState,
    devices: &Devices,
    request: &ReportRequest,
) -> Result<PrintResult, AppError> {
    may_see(state, request)?;
    Ok(receipts::print_report(state, devices, request))
}

/// The user, if they may see every report. `None` when they lack `ReportsViewAll`.
/// A locked till is still refused.
fn sees_all(state: &AppState) -> Result<Option<SessionUser>, AppError> {
    match session::require(state, Permission::ReportsViewAll) {
        Ok(user) => Ok(Some(user)),
        Err(AppError::PermissionDenied { .. }) => Ok(None),
        Err(other) => Err(other),
    }
}

/// RPT-10: `ReportsViewAll`, or the X/Z report of the user's own shift.
fn may_see(state: &AppState, request: &ReportRequest) -> Result<(), AppError> {
    if sees_all(state)?.is_some() {
        return Ok(());
    }
    let refused = AppError::PermissionDenied {
        permission: Permission::ReportsViewAll,
    };
    let Ok(user) = session::require(state, Permission::ShiftOpenCloseOwn) else {
        return Err(refused);
    };
    if report_runs::is_own_shift(state, &user, request)? {
        Ok(())
    } else {
        Err(refused)
    }
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::{AppError, ReportTable};
    use tauri::State;

    use crate::commands::on_worker;
    use crate::devices::Devices;
    use crate::state::AppState;
    use crate::workflows::products::Choice;
    use crate::workflows::receipts::PrintResult;
    use crate::workflows::report_runs::{ReportChoices, ReportRequest, ShiftList};

    #[tauri::command]
    pub async fn report_choices(
        state: State<'_, Arc<AppState>>,
    ) -> Result<ReportChoices, AppError> {
        let state = Arc::clone(&state);
        on_worker("report_choices", move || super::report_choices(&state)).await
    }

    #[tauri::command]
    pub async fn report_customer_search(
        state: State<'_, Arc<AppState>>,
        text: String,
    ) -> Result<Vec<Choice>, AppError> {
        let state = Arc::clone(&state);
        on_worker("report_customer_search", move || {
            super::report_customer_search(&state, &text)
        })
        .await
    }

    #[tauri::command]
    pub async fn report_shifts(state: State<'_, Arc<AppState>>) -> Result<ShiftList, AppError> {
        let state = Arc::clone(&state);
        on_worker("report_shifts", move || super::report_shifts(&state)).await
    }

    #[tauri::command]
    pub async fn report_run(
        state: State<'_, Arc<AppState>>,
        request: ReportRequest,
    ) -> Result<ReportTable, AppError> {
        let state = Arc::clone(&state);
        on_worker("report_run", move || super::report_run(&state, &request)).await
    }

    #[tauri::command]
    pub async fn report_export_xlsx(
        state: State<'_, Arc<AppState>>,
        request: ReportRequest,
        file: String,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("report_export_xlsx", move || {
            super::report_export_xlsx(&state, &request, &file)
        })
        .await
    }

    #[tauri::command]
    pub async fn report_print(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        request: ReportRequest,
    ) -> Result<PrintResult, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("report_print", move || {
            super::report_print(&state, &devices, &request)
        })
        .await
    }
}
