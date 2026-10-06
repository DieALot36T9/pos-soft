//! Reprinting receipts and handling failed printouts (PAY-09, PAY-10, HW-07, HW-08).
//! Reprinting this till's last receipt needs `ReceiptReprintLast`; any other receipt
//! needs `ReceiptReprintAny`. "Retry", the system printer and hiding the banner work
//! only for a printout that failed, for anyone signed in.

use pos_core::{AppError, Permission};

use crate::devices::Devices;
use crate::session;
use crate::state::AppState;
use crate::workflows::receipts::{self, PrintResult, Printable};

pub fn receipt_reprint_last(state: &AppState, devices: &Devices) -> Result<PrintResult, AppError> {
    let user = session::require(state, Permission::ReceiptReprintLast)?;
    receipts::reprint_last(state, devices, &user.id)
}

pub fn receipt_reprint(
    state: &AppState,
    devices: &Devices,
    invoice_no: &str,
) -> Result<PrintResult, AppError> {
    let user = session::require(state, Permission::ReceiptReprintAny)?;
    receipts::reprint_invoice(state, devices, &user.id, invoice_no)
}

pub fn print_retry(
    state: &AppState,
    devices: &Devices,
    printable: &Printable,
) -> Result<PrintResult, AppError> {
    session::require_signed_in(state)?;
    receipts::retry(state, devices, printable)
}

pub fn print_system_pictures(
    state: &AppState,
    devices: &Devices,
    printable: &Printable,
) -> Result<Vec<String>, AppError> {
    session::require_signed_in(state)?;
    receipts::system_printer_pictures(state, devices, printable)
}

pub fn print_dismiss(
    state: &AppState,
    devices: &Devices,
    printable: &Printable,
) -> Result<(), AppError> {
    session::require_signed_in(state)?;
    receipts::dismiss(devices, printable)
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::AppError;
    use tauri::State;

    use crate::commands::on_worker;
    use crate::devices::Devices;
    use crate::state::AppState;
    use crate::workflows::receipts::{PrintResult, Printable};

    #[tauri::command]
    pub async fn receipt_reprint_last(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
    ) -> Result<PrintResult, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("receipt_reprint_last", move || {
            super::receipt_reprint_last(&state, &devices)
        })
        .await
    }

    #[tauri::command]
    pub async fn receipt_reprint(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        invoice_no: String,
    ) -> Result<PrintResult, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("receipt_reprint", move || {
            super::receipt_reprint(&state, &devices, &invoice_no)
        })
        .await
    }

    #[tauri::command]
    pub async fn print_retry(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        printable: Printable,
    ) -> Result<PrintResult, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("print_retry", move || {
            super::print_retry(&state, &devices, &printable)
        })
        .await
    }

    #[tauri::command]
    pub async fn print_system_pictures(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        printable: Printable,
    ) -> Result<Vec<String>, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("print_system_pictures", move || {
            super::print_system_pictures(&state, &devices, &printable)
        })
        .await
    }

    #[tauri::command]
    pub async fn print_dismiss(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        printable: Printable,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("print_dismiss", move || {
            super::print_dismiss(&state, &devices, &printable)
        })
        .await
    }
}
