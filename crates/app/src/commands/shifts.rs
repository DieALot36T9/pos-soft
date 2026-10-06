//! The shift screens (Req - Shifts and Cash). Opening and closing one's own shift
//! needs `ShiftOpenCloseOwn`. Cash in and out, the no-sale drawer and force close take
//! their permission or an admin's approval token (USR-05). The X report of one's own
//! shift needs `ShiftOpenCloseOwn` (SHF-05).

use pos_core::{AppError, Money, Permission};

use crate::devices::Devices;
use crate::session;
use crate::state::AppState;
use crate::workflows::receipts::{self, PrintResult};
use crate::workflows::shifts::{
    self, CashMoveInput, CountLine, ShiftClosed, ShiftState, ShiftView,
};

/// The till's open shift and how cash is counted. Anyone signed in.
pub fn shift_current(state: &AppState) -> Result<ShiftState, AppError> {
    let user = session::require_signed_in(state)?;
    shifts::state_for(state, &user.id)
}

/// Adds up a count by denomination for the screen. Anyone signed in.
pub fn shift_count_total(state: &AppState, counts: &[CountLine]) -> Result<Money, AppError> {
    session::require_signed_in(state)?;
    shifts::count_total(counts)
}

pub fn shift_open(
    state: &AppState,
    float: Money,
    counts: &[CountLine],
) -> Result<ShiftView, AppError> {
    let user = session::require(state, Permission::ShiftOpenCloseOwn)?;
    shifts::open(state, &user.id, float, counts)
}

pub fn shift_cash_move(
    state: &AppState,
    devices: &Devices,
    input: &CashMoveInput,
    approval: Option<&str>,
) -> Result<PrintResult, AppError> {
    let allowed = session::require_or_approval(state, Permission::ShiftCashInOut, approval)?;
    shifts::cash_move_and_open(
        state,
        devices,
        &allowed.user.id,
        allowed.approved_by.as_ref(),
        input,
    )
}

pub fn shift_no_sale(
    state: &AppState,
    devices: &Devices,
    reason: &str,
    approval: Option<&str>,
) -> Result<PrintResult, AppError> {
    let allowed = session::require_or_approval(state, Permission::ShiftNoSaleDrawer, approval)?;
    shifts::no_sale_and_open(
        state,
        devices,
        &allowed.user.id,
        allowed.approved_by.as_ref(),
        reason,
    )
}

pub fn shift_close(
    state: &AppState,
    devices: &Devices,
    counts: &[CountLine],
    note: Option<&str>,
) -> Result<ShiftClosed, AppError> {
    let user = session::require(state, Permission::ShiftOpenCloseOwn)?;
    shifts::close_and_print(state, devices, &user.id, counts, note)
}

pub fn shift_force_close(
    state: &AppState,
    devices: &Devices,
    counts: &[CountLine],
    note: Option<&str>,
    approval: Option<&str>,
) -> Result<ShiftClosed, AppError> {
    let allowed = session::require_or_approval(state, Permission::ShiftForceClose, approval)?;
    shifts::force_close_and_print(
        state,
        devices,
        &allowed.user.id,
        allowed.approved_by.as_ref(),
        counts,
        note,
    )
}

/// Prints the X report of the user's own open shift (SHF-05).
pub fn shift_x_report(state: &AppState, devices: &Devices) -> Result<PrintResult, AppError> {
    let user = session::require(state, Permission::ShiftOpenCloseOwn)?;
    receipts::x_report(state, devices, &user.id)
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::{AppError, Money};
    use tauri::State;

    use crate::commands::on_worker;
    use crate::devices::Devices;
    use crate::state::AppState;
    use crate::workflows::receipts::PrintResult;
    use crate::workflows::shifts::{CashMoveInput, CountLine, ShiftClosed, ShiftState, ShiftView};

    #[tauri::command]
    pub async fn shift_current(state: State<'_, Arc<AppState>>) -> Result<ShiftState, AppError> {
        let state = Arc::clone(&state);
        on_worker("shift_current", move || super::shift_current(&state)).await
    }

    #[tauri::command]
    pub async fn shift_count_total(
        state: State<'_, Arc<AppState>>,
        counts: Vec<CountLine>,
    ) -> Result<Money, AppError> {
        let state = Arc::clone(&state);
        on_worker("shift_count_total", move || {
            super::shift_count_total(&state, &counts)
        })
        .await
    }

    #[tauri::command]
    pub async fn shift_open(
        state: State<'_, Arc<AppState>>,
        float: Money,
        counts: Vec<CountLine>,
    ) -> Result<ShiftView, AppError> {
        let state = Arc::clone(&state);
        on_worker("shift_open", move || {
            super::shift_open(&state, float, &counts)
        })
        .await
    }

    #[tauri::command]
    pub async fn shift_cash_move(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        input: CashMoveInput,
        approval: Option<String>,
    ) -> Result<PrintResult, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("shift_cash_move", move || {
            super::shift_cash_move(&state, &devices, &input, approval.as_deref())
        })
        .await
    }

    #[tauri::command]
    pub async fn shift_no_sale(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        reason: String,
        approval: Option<String>,
    ) -> Result<PrintResult, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("shift_no_sale", move || {
            super::shift_no_sale(&state, &devices, &reason, approval.as_deref())
        })
        .await
    }

    #[tauri::command]
    pub async fn shift_close(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        counts: Vec<CountLine>,
        note: Option<String>,
    ) -> Result<ShiftClosed, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("shift_close", move || {
            super::shift_close(&state, &devices, &counts, note.as_deref())
        })
        .await
    }

    #[tauri::command]
    pub async fn shift_force_close(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        counts: Vec<CountLine>,
        note: Option<String>,
        approval: Option<String>,
    ) -> Result<ShiftClosed, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("shift_force_close", move || {
            super::shift_force_close(
                &state,
                &devices,
                &counts,
                note.as_deref(),
                approval.as_deref(),
            )
        })
        .await
    }

    #[tauri::command]
    pub async fn shift_x_report(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
    ) -> Result<PrintResult, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("shift_x_report", move || {
            super::shift_x_report(&state, &devices)
        })
        .await
    }
}
