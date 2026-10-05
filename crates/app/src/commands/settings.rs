//! Settings screens and the helpers the setup wizard uses (rules SET-04 to SET-17, BAK-07).

use pos_core::{AppError, CurrencyFormat, Permission, format_money, parse_money};
use settings::{CurrencyChoice, SettingsSnapshot};

use crate::session;
use crate::state::AppState;
use crate::workflows::settings_change::{self, SettingsChange};

/// Every setting. Needs `SettingsManage` (or setup still open).
pub fn settings_get(state: &AppState) -> Result<SettingsSnapshot, AppError> {
    session::require_or_setup(state, Permission::SettingsManage)?;
    settings_change::read_all(state)
}

/// Saves one section, audited. Needs `SettingsManage` (or setup still open).
pub fn settings_save(
    state: &AppState,
    change: &SettingsChange,
) -> Result<SettingsSnapshot, AppError> {
    let user = session::require_or_setup(state, Permission::SettingsManage)?;
    settings_change::save(state, change, user.as_ref().map(|user| &user.id))
}

/// Detailed logging on or off, at once (BAK-07). Needs `LogsManage`.
pub fn logs_set_detailed(state: &AppState, detailed: bool) -> Result<(), AppError> {
    let user = session::require(state, Permission::LogsManage)?;
    settings_change::set_detailed_logging(state, detailed, Some(&user.id))
}

/// The ISO 4217 currency list for the currency picker. Needs no permission.
pub fn currency_choices() -> Vec<CurrencyChoice> {
    settings::currency_choices()
}

/// A ready-made format for a currency code. Needs no permission.
pub fn currency_format_for(code: &str) -> Result<CurrencyFormat, AppError> {
    settings::currency_format_for(code)
}

/// How 1234567.5 looks in this format, for the live preview (SET-05). Needs no permission.
/// A currency without decimals shows 1234567.
pub fn money_preview(format: &CurrencyFormat) -> Result<String, AppError> {
    let sample = if format.minor_digits == 0 {
        "1234567"
    } else {
        "1234567.5"
    };
    let amount = parse_money(sample, format.minor_digits)?;
    Ok(format_money(amount, format))
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::{AppError, CurrencyFormat};
    use settings::{CurrencyChoice, SettingsSnapshot};
    use tauri::State;

    use crate::commands::on_worker;
    use crate::state::AppState;
    use crate::workflows::settings_change::SettingsChange;

    #[tauri::command]
    pub async fn settings_get(
        state: State<'_, Arc<AppState>>,
    ) -> Result<SettingsSnapshot, AppError> {
        let state = Arc::clone(&state);
        on_worker("settings_get", move || super::settings_get(&state)).await
    }

    #[tauri::command]
    pub async fn settings_save(
        state: State<'_, Arc<AppState>>,
        change: SettingsChange,
    ) -> Result<SettingsSnapshot, AppError> {
        let state = Arc::clone(&state);
        on_worker("settings_save", move || {
            super::settings_save(&state, &change)
        })
        .await
    }

    #[tauri::command]
    pub async fn logs_set_detailed(
        state: State<'_, Arc<AppState>>,
        detailed: bool,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("logs_set_detailed", move || {
            super::logs_set_detailed(&state, detailed)
        })
        .await
    }

    #[tauri::command]
    pub fn currency_choices() -> Vec<CurrencyChoice> {
        super::currency_choices()
    }

    #[tauri::command]
    pub fn currency_format_for(code: String) -> Result<CurrencyFormat, AppError> {
        super::currency_format_for(&code)
    }

    #[tauri::command]
    pub fn money_preview(format: CurrencyFormat) -> Result<String, AppError> {
        super::money_preview(&format)
    }
}
