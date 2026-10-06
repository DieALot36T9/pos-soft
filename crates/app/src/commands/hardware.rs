//! Printer commands: "Test print" on the saved receipt printer, the Preview printer's
//! printouts, and the system print dialog (rules HW-02, HW-07, HW-10).

use std::sync::Arc;

use pos_core::{AppError, Permission};
use settings::{HardwareSettings, PaperWidth};
use tauri::State;

use super::on_worker;
use crate::devices::Devices;
use crate::preview::{self, PreviewPrintout};
use crate::session;
use crate::state::AppState;
use crate::workflows::test_print;

/// Prints the test receipt on the saved receipt printer (the Preview printer until one
/// is chosen). Needs `SettingsManage`, except during first-run setup.
pub fn hardware_test_print(
    state: &AppState,
    devices: &Devices,
    paper: PaperWidth,
) -> Result<(), AppError> {
    session::require_or_setup(state, Permission::SettingsManage)?;
    let hardware: HardwareSettings = state.db.read(|conn| Ok(settings::get(conn)?))?;
    let printer = devices.receipt_printer(&hardware);
    test_print::run(printer.as_ref(), paper, &devices.fonts)
}

pub mod cmd {
    use std::sync::Arc;

    use pos_core::AppError;
    use settings::PaperWidth;
    use tauri::State;

    use crate::commands::on_worker;
    use crate::devices::Devices;
    use crate::state::AppState;

    #[tauri::command]
    pub async fn hardware_test_print(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        paper: PaperWidth,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("hardware_test_print", move || {
            super::hardware_test_print(&state, &devices, paper)
        })
        .await
    }
}

/// Hands the screen every Preview printer printout since the last call.
#[tauri::command]
pub async fn preview_take_printouts(
    devices: State<'_, Arc<Devices>>,
) -> Result<Vec<PreviewPrintout>, AppError> {
    let devices = Arc::clone(&devices);
    on_worker("preview_take_printouts", move || {
        preview::take_printouts(&devices)
    })
    .await
}

/// Opens the system print dialog for what the window shows. The screen shows only the
/// printout while printing, so any normal printer or "Save as PDF" works (ADR-013).
#[tauri::command]
pub fn print_window(window: tauri::WebviewWindow) -> Result<(), AppError> {
    window
        .print()
        .map_err(|error| AppError::internal(format!("the print dialog did not open: {error}")))
}
