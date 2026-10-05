//! Tax rate screens (rules TAX-02, TAX-03, TAX-10). They need `TaxManage`, except
//! while setup is open (the wizard's tax step adds the first rates).

use pos_core::{AppError, Id, Permission};
use tax::RateWithComponents;

use crate::session;
use crate::state::AppState;
use crate::workflows::tax_rates::{self, RateInput};

pub fn tax_rate_list(
    state: &AppState,
    include_inactive: bool,
) -> Result<Vec<RateWithComponents>, AppError> {
    session::require_or_setup(state, Permission::TaxManage)?;
    tax_rates::list(state, include_inactive)
}

pub fn tax_rate_save(state: &AppState, input: RateInput) -> Result<Id, AppError> {
    let user = session::require_or_setup(state, Permission::TaxManage)?;
    tax_rates::save(state, input, user.as_ref().map(|user| &user.id))
}

pub fn tax_rate_deactivate(state: &AppState, id: &Id) -> Result<(), AppError> {
    let user = session::require_or_setup(state, Permission::TaxManage)?;
    tax_rates::deactivate(state, id, user.as_ref().map(|user| &user.id))
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::{AppError, Id};
    use tauri::State;
    use tax::RateWithComponents;

    use crate::commands::on_worker;
    use crate::state::AppState;
    use crate::workflows::tax_rates::RateInput;

    #[tauri::command]
    pub async fn tax_rate_list(
        state: State<'_, Arc<AppState>>,
        include_inactive: bool,
    ) -> Result<Vec<RateWithComponents>, AppError> {
        let state = Arc::clone(&state);
        on_worker("tax_rate_list", move || {
            super::tax_rate_list(&state, include_inactive)
        })
        .await
    }

    #[tauri::command]
    pub async fn tax_rate_save(
        state: State<'_, Arc<AppState>>,
        input: RateInput,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("tax_rate_save", move || super::tax_rate_save(&state, input)).await
    }

    #[tauri::command]
    pub async fn tax_rate_deactivate(
        state: State<'_, Arc<AppState>>,
        id: Id,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("tax_rate_deactivate", move || {
            super::tax_rate_deactivate(&state, &id)
        })
        .await
    }
}
