//! Payment methods on the Settings screen (SET-15). Needs `SettingsManage`.

use pos_core::{AppError, Id, Permission};

use crate::session;
use crate::state::AppState;
use crate::workflows::payment_methods::{self, PaymentMethodRow, PaymentMethodSave};

pub fn payment_method_list(state: &AppState) -> Result<Vec<PaymentMethodRow>, AppError> {
    session::require(state, Permission::SettingsManage)?;
    payment_methods::list(state)
}

pub fn payment_method_save(state: &AppState, input: &PaymentMethodSave) -> Result<Id, AppError> {
    let user = session::require(state, Permission::SettingsManage)?;
    payment_methods::save(state, &user.id, input)
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::{AppError, Id};
    use tauri::State;

    use crate::commands::on_worker;
    use crate::state::AppState;
    use crate::workflows::payment_methods::{PaymentMethodRow, PaymentMethodSave};

    #[tauri::command]
    pub async fn payment_method_list(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<PaymentMethodRow>, AppError> {
        let state = Arc::clone(&state);
        on_worker("payment_method_list", move || {
            super::payment_method_list(&state)
        })
        .await
    }

    #[tauri::command]
    pub async fn payment_method_save(
        state: State<'_, Arc<AppState>>,
        input: PaymentMethodSave,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("payment_method_save", move || {
            super::payment_method_save(&state, &input)
        })
        .await
    }
}
