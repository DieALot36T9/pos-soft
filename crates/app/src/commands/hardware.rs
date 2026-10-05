//! Printer commands that need no device: "Test print" on the Preview printer, the
//! Preview printer's printouts, and the system print dialog (rules HW-02, HW-07, HW-10).

use std::sync::Arc;

use pos_core::AppError;
use settings::PaperWidth;
use tauri::State;

use super::on_worker;
use crate::devices::Devices;
use crate::preview::{self, PreviewPrintout};
use crate::workflows::test_print;

/// Prints the test receipt on the receipt printer (for now always the Preview printer).
#[tauri::command]
pub async fn hardware_test_print(
    devices: State<'_, Arc<Devices>>,
    paper: PaperWidth,
) -> Result<(), AppError> {
    let devices = Arc::clone(&devices);
    on_worker(move || {
        let printer = devices.receipt_printer();
        test_print::run(printer.as_ref(), paper, &devices.fonts)
    })
    .await
}

/// Hands the screen every Preview printer printout since the last call.
#[tauri::command]
pub async fn preview_take_printouts(
    devices: State<'_, Arc<Devices>>,
) -> Result<Vec<PreviewPrintout>, AppError> {
    let devices = Arc::clone(&devices);
    on_worker(move || preview::take_printouts(&devices)).await
}

/// Opens the system print dialog for what the window shows. The screen shows only the
/// printout while printing, so any normal printer or "Save as PDF" works (ADR-013).
#[tauri::command]
pub fn print_window(window: tauri::WebviewWindow) -> Result<(), AppError> {
    window
        .print()
        .map_err(|error| AppError::internal(format!("the print dialog did not open: {error}")))
}
