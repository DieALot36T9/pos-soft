//! Returns, exchanges and voids at the till (Req - Returns and Voids). Looking a sale up
//! and previewing a refund need `CheckoutSell`. Completing a return needs
//! `ReturnWithReceipt` (`ReturnWithoutReceipt` without one), and a void needs `SaleVoid`;
//! both take an admin's approval token instead (USR-05), and the audit event records
//! who approved it.

use pos_core::{AppError, Id, Permission};

use crate::devices::Devices;
use crate::session;
use crate::state::AppState;
use crate::workflows::returns::{
    self, ExchangeCashOut, ReturnDone, ReturnInput, ReturnPreview, ReturnSale, Returner,
};
use crate::workflows::voids::{self, VoidDone};

/// Finds a sale by the invoice number typed or scanned from the receipt (RET-01).
pub fn return_lookup(state: &AppState, invoice_no: &str) -> Result<ReturnSale, AppError> {
    session::require(state, Permission::CheckoutSell)?;
    returns::lookup(state, invoice_no)
}

/// What the return would refund, line by line and by method (RET-02, RET-07).
pub fn return_preview(state: &AppState, input: &ReturnInput) -> Result<ReturnPreview, AppError> {
    session::require(state, Permission::CheckoutSell)?;
    returns::preview(state, input)
}

/// Saves the return, then prints the return receipt and opens the drawer for cash.
pub fn return_complete(
    state: &AppState,
    devices: &Devices,
    input: &ReturnInput,
    approval: Option<&str>,
) -> Result<ReturnDone, AppError> {
    let allowed = session::require_or_approval(state, returns::permission_for(input), approval)?;
    let by = Returner {
        user: &allowed.user,
        approved_by: allowed.approved_by.as_ref(),
    };
    returns::complete_and_print(state, devices, &by, input)
}

/// Saves the return as exchange credit and starts the new sale with it (RET-05). Also
/// needs `CheckoutSell`, because it opens a sale.
pub fn exchange_start(
    state: &AppState,
    devices: &Devices,
    input: &ReturnInput,
    approval: Option<&str>,
) -> Result<ReturnDone, AppError> {
    let allowed = session::require_or_approval(state, returns::permission_for(input), approval)?;
    session::require(state, Permission::CheckoutSell)?;
    let by = Returner {
        user: &allowed.user,
        approved_by: allowed.approved_by.as_ref(),
    };
    returns::exchange_start(state, devices, &by, input)
}

/// Gives the exchange credit back in cash: the customer took nothing new (RET-05).
pub fn exchange_cash_out(state: &AppState, devices: &Devices) -> Result<ExchangeCashOut, AppError> {
    let user = session::require(state, Permission::CheckoutSell)?;
    returns::exchange_cash_out(state, devices, &user.id)
}

/// Voids a completed sale while its shift is open (RET-04), then prints the void slip.
pub fn sale_void(
    state: &AppState,
    devices: &Devices,
    sale_id: &Id,
    reason: &str,
    approval: Option<&str>,
) -> Result<VoidDone, AppError> {
    let allowed = session::require_or_approval(state, Permission::SaleVoid, approval)?;
    voids::void_and_print(
        state,
        devices,
        &allowed.user.id,
        allowed.approved_by.as_ref(),
        sale_id,
        reason,
    )
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::{AppError, Id};
    use tauri::State;

    use crate::commands::on_worker;
    use crate::devices::Devices;
    use crate::state::AppState;
    use crate::workflows::returns::{
        ExchangeCashOut, ReturnDone, ReturnInput, ReturnPreview, ReturnSale,
    };
    use crate::workflows::voids::VoidDone;

    #[tauri::command]
    pub async fn return_lookup(
        state: State<'_, Arc<AppState>>,
        invoice_no: String,
    ) -> Result<ReturnSale, AppError> {
        let state = Arc::clone(&state);
        on_worker("return_lookup", move || {
            super::return_lookup(&state, &invoice_no)
        })
        .await
    }

    #[tauri::command]
    pub async fn return_preview(
        state: State<'_, Arc<AppState>>,
        input: ReturnInput,
    ) -> Result<ReturnPreview, AppError> {
        let state = Arc::clone(&state);
        on_worker("return_preview", move || {
            super::return_preview(&state, &input)
        })
        .await
    }

    #[tauri::command]
    pub async fn return_complete(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        input: ReturnInput,
        approval: Option<String>,
    ) -> Result<ReturnDone, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("return_complete", move || {
            super::return_complete(&state, &devices, &input, approval.as_deref())
        })
        .await
    }

    #[tauri::command]
    pub async fn exchange_start(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        input: ReturnInput,
        approval: Option<String>,
    ) -> Result<ReturnDone, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("exchange_start", move || {
            super::exchange_start(&state, &devices, &input, approval.as_deref())
        })
        .await
    }

    #[tauri::command]
    pub async fn exchange_cash_out(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
    ) -> Result<ExchangeCashOut, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("exchange_cash_out", move || {
            super::exchange_cash_out(&state, &devices)
        })
        .await
    }

    #[tauri::command]
    pub async fn sale_void(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        sale_id: Id,
        reason: String,
        approval: Option<String>,
    ) -> Result<VoidDone, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("sale_void", move || {
            super::sale_void(&state, &devices, &sale_id, &reason, approval.as_deref())
        })
        .await
    }
}
