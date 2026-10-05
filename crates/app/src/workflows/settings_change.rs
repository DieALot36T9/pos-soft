//! Changing settings after setup (rule SET-13): each change is checked, saved and
//! written to the audit log with its old and new values, in one transaction.

use pos_core::{AppError, Clock, CurrencyFormat, Id, RoundingRule};
use rusqlite::Transaction;
use serde::Deserialize;
use settings::{
    BackupSettings, CheckoutSettings, CreditSettings, Denominations, HardwareSettings,
    InventorySettings, LoggingSettings, LoyaltySettings, ReceiptSettings, Setting,
    SettingsSnapshot, ShopProfile, TaxSettings, TillSettings,
};
use ts_rs::TS;

use crate::audit_actions::{LOGGING_SWITCH, SETTINGS_CHANGE};
use crate::audit_log;
use crate::state::AppState;
use crate::workflows::folders::check_backup_folders;
use crate::workflows::setup::{TillStep, field_error};

/// One section of the Settings screen, as saved.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(tag = "section", content = "value", rename_all = "snake_case")]
#[ts(export)]
pub enum SettingsChange {
    Shop(ShopProfile),
    Money {
        currency: CurrencyFormat,
        rounding: RoundingRule,
    },
    Tax(TaxSettings),
    Till(TillStep),
    Receipt(ReceiptSettings),
    Hardware(HardwareSettings),
    Backup(BackupSettings),
    Checkout(CheckoutSettings),
    Inventory(InventorySettings),
    Loyalty(LoyaltySettings),
    Credit(CreditSettings),
    Denominations(Denominations),
}

/// Every setting at once, for the Settings screen.
pub fn read_all(state: &AppState) -> Result<SettingsSnapshot, AppError> {
    state.db.read(|conn| Ok(settings::snapshot(conn)?))
}

/// Saves one section. `by` is the signed-in user, or `None` during setup.
pub fn save(
    state: &AppState,
    change: &SettingsChange,
    by: Option<&Id>,
) -> Result<SettingsSnapshot, AppError> {
    if let SettingsChange::Backup(backup) = change {
        check_backup_folders(backup)?;
    }
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        match change {
            SettingsChange::Shop(value) => save_one(tx, clock, value, by)?,
            SettingsChange::Money { currency, rounding } => {
                check_decimals_unlocked(tx, currency)?;
                save_one(tx, clock, currency, by)?;
                save_one(tx, clock, rounding, by)?;
            }
            SettingsChange::Tax(value) => {
                if let Some(rate) = &value.default_rate_id {
                    tax::get(tx, rate)
                        .map_err(|_| field_error("default_rate_id", "tax_default_rate_unknown"))?;
                }
                save_one(tx, clock, value, by)?;
            }
            SettingsChange::Till(step) => {
                let saved: TillSettings = settings::get(tx)?;
                let till = TillSettings {
                    till_id: saved.till_id,
                    till_code: step.till_code.trim().to_uppercase(),
                    business_day_cutoff_hour: step.business_day_cutoff_hour,
                };
                save_one(tx, clock, &till, by)?;
            }
            SettingsChange::Receipt(value) => save_one(tx, clock, value, by)?,
            SettingsChange::Hardware(value) => save_one(tx, clock, value, by)?,
            SettingsChange::Backup(value) => save_one(tx, clock, value, by)?,
            SettingsChange::Checkout(value) => save_one(tx, clock, value, by)?,
            SettingsChange::Inventory(value) => save_one(tx, clock, value, by)?,
            SettingsChange::Loyalty(value) => save_one(tx, clock, value, by)?,
            SettingsChange::Credit(value) => save_one(tx, clock, value, by)?,
            SettingsChange::Denominations(value) => save_one(tx, clock, value, by)?,
        }
        Ok(settings::snapshot(tx)?)
    })
}

/// Switches detailed logging on or off: saved, audited, and applied at once (BAK-07).
pub fn set_detailed_logging(
    state: &AppState,
    detailed: bool,
    by: Option<&Id>,
) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        settings::set(tx, &LoggingSettings { detailed }, by, clock.now())?;
        audit_log::record(
            tx,
            clock,
            LOGGING_SWITCH,
            serde_json::json!({ "detailed": detailed }),
            by.cloned(),
        )
    })?;
    if let Some(switch) = &state.log {
        switch.set_detailed(detailed)?;
    }
    Ok(())
}

/// Rule SET-05: once a sale exists, the number of currency decimals can't change,
/// because every stored amount would change its meaning.
fn check_decimals_unlocked(tx: &Transaction, currency: &CurrencyFormat) -> Result<(), AppError> {
    let saved: CurrencyFormat = settings::get(tx)?;
    if saved.minor_digits != currency.minor_digits && sales::has_any_sale(tx)? {
        return Err(field_error("minor_digits", "minor_digits_locked"));
    }
    Ok(())
}

/// Saves one setting and audits its old and new values.
fn save_one<S: Setting>(
    tx: &Transaction,
    clock: &dyn Clock,
    value: &S,
    by: Option<&Id>,
) -> Result<(), AppError> {
    // The value in use before, which is the default when it was never saved.
    let old: S = settings::get(tx)?;
    settings::set(tx, value, by, clock.now())?;
    let details = serde_json::json!({
        "key": S::KEY,
        "old": serde_json::to_value(&old).unwrap_or_default(),
        "new": serde_json::to_value(value).unwrap_or_default(),
    });
    audit_log::record(tx, clock, SETTINGS_CHANGE, details, by.cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_13_a_change_names_its_section() {
        let change: SettingsChange = serde_json::from_str(
            r#"{"section":"credit","value":{"enabled":true,"default_limit_minor":500}}"#,
        )
        .expect("reads");
        assert!(matches!(
            change,
            SettingsChange::Credit(CreditSettings { enabled: true, .. })
        ));
    }
}
