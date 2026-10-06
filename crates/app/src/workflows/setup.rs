//! First-run setup (rules SET-01 to SET-12): each step is saved in its own
//! transaction together with "step done", so setup can stop and resume (SET-02).

use auth::{ADMIN_ROLE_ID, NewUser};
use pos_core::{AppError, CurrencyFormat, Id, RoundingRule, Secret, TaxMode};
use rusqlite::Transaction;
use serde::{Deserialize, Serialize};
use settings::{
    BackupSettings, Denominations, DrawerPin, HardwareSettings, PaperWidth, PrinterConnection,
    ReceiptSettings, SetupStatus, SetupStep, ShopProfile, TaxSettings, TillSettings,
};
use ts_rs::TS;

use crate::session::{self, SessionView};
use crate::state::AppState;
use crate::workflows::folders::check_backup_folders;

/// Step 2: money format and cash rounding.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct MoneyStep {
    pub currency: CurrencyFormat,
    pub rounding: RoundingRule,
}

/// Step 3: the tax label and mode. The rates are added first with the tax rate commands.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct TaxStep {
    pub label: String,
    pub mode: TaxMode,
    /// `None` takes the first active rate.
    pub default_rate_id: Option<Id>,
}

/// Step 4: the first admin (SET-08).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct AdminStep {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub password_again: String,
}

/// What the admin step gives back: the recovery code, shown once (USR-09).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AdminCreated {
    pub recovery_code: String,
    pub session: SessionView,
}

/// Step 5: this till (SET-09).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct TillStep {
    pub till_code: String,
    pub business_day_cutoff_hour: u8,
}

/// Step 6: the receipt printer (SET-10). Can be skipped.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct HardwareStep {
    pub receipt_printer: Option<PrinterConnection>,
    pub paper_width: PaperWidth,
    pub drawer_pin: DrawerPin,
}

/// Where setup stands, for the wizard.
pub fn status(state: &AppState) -> Result<SetupStatus, AppError> {
    state.db.read(|conn| Ok(settings::setup_status(conn)?))
}

/// Step 1: the shop's name, address and receipt footer (SET-04).
pub fn save_shop(state: &AppState, shop: &ShopProfile) -> Result<SetupStatus, AppError> {
    save_step(state, SetupStep::Shop, |tx, now| {
        settings::set(tx, shop, None, now)?;
        Ok(())
    })
}

/// Step 2: currency, rounding, and the notes and coins for that currency (SET-05, SET-06, SET-16).
pub fn save_money(state: &AppState, step: &MoneyStep) -> Result<SetupStatus, AppError> {
    save_step(state, SetupStep::Currency, |tx, now| {
        settings::set(tx, &step.currency, None, now)?;
        settings::set(tx, &step.rounding, None, now)?;
        let notes =
            settings::suggested_denominations(&step.currency.code, step.currency.minor_digits);
        let saved: Denominations = settings::get(tx)?;
        if saved.0.is_empty() {
            settings::set(tx, &notes, None, now)?;
        }
        Ok(())
    })
}

/// Step 3: needs at least one active rate; the default must be one of them (SET-07).
pub fn save_tax(state: &AppState, step: &TaxStep) -> Result<SetupStatus, AppError> {
    save_step(state, SetupStep::Tax, |tx, now| {
        let rates = tax::list(tx, false)?;
        let Some(first) = rates.first() else {
            return Err(field_error("rates", "tax_rate_missing"));
        };
        let default_rate_id = match &step.default_rate_id {
            Some(wanted) if rates.iter().any(|rate| &rate.id == wanted) => wanted.clone(),
            Some(_) => return Err(field_error("default_rate_id", "tax_default_rate_unknown")),
            None => first.id.clone(),
        };
        let tax_settings = TaxSettings {
            label: step.label.clone(),
            mode: step.mode,
            default_rate_id: Some(default_rate_id),
        };
        settings::set(tx, &tax_settings, None, now)?;
        Ok(())
    })
}

/// Step 4: creates the first admin with a password, makes the recovery code and
/// signs the admin in (SET-08, USR-09). Only once.
pub fn save_admin(state: &AppState, step: AdminStep) -> Result<AdminCreated, AppError> {
    session::require_setup_open(state)?;
    if step.password != step.password_again {
        return Err(field_error("password_again", "passwords_differ"));
    }
    let now = state.clock.now();
    let admin_role = Id::parse(ADMIN_ROLE_ID)?;
    let (admin, recovery_code) = state.db.write(|tx| {
        if settings::setup_status(tx)?
            .completed
            .contains(&SetupStep::Admin)
        {
            return Err(AppError::Conflict {
                code: "admin_already_created".into(),
                params: Default::default(),
            });
        }
        let user = NewUser {
            username: step.username.clone(),
            display_name: step.display_name.clone(),
            role_id: admin_role,
            password: Some(Secret::new(step.password.clone())),
            pin: None,
        };
        let admin = auth::create_user(tx, user, now)?;
        let code = auth::create_recovery_code(tx, now)?;
        settings::complete_step(tx, SetupStep::Admin, now)?;
        Ok((admin, code))
    })?;
    Ok(AdminCreated {
        recovery_code: recovery_code.expose().clone(),
        session: session::start_session(state, &admin)?,
    })
}

/// Step 5: till code and business-day cutoff. The till gets its id here, once.
pub fn save_till(state: &AppState, step: &TillStep) -> Result<SetupStatus, AppError> {
    save_step(state, SetupStep::Till, |tx, now| {
        let saved: TillSettings = settings::get(tx)?;
        let till = TillSettings {
            till_id: Some(saved.till_id.unwrap_or_default()),
            till_code: step.till_code.trim().to_uppercase(),
            business_day_cutoff_hour: step.business_day_cutoff_hour,
        };
        settings::set(tx, &till, None, now)?;
        Ok(())
    })
}

/// Step 6: printer, paper width and drawer pin (SET-10).
pub fn save_hardware(state: &AppState, step: &HardwareStep) -> Result<SetupStatus, AppError> {
    save_step(state, SetupStep::Hardware, |tx, now| {
        let mut hardware: HardwareSettings = settings::get(tx)?;
        hardware.receipt_printer = step.receipt_printer.clone();
        hardware.drawer_pin = step.drawer_pin;
        settings::set(tx, &hardware, None, now)?;
        let mut receipt: ReceiptSettings = settings::get(tx)?;
        receipt.paper_width = step.paper_width;
        settings::set(tx, &receipt, None, now)?;
        Ok(())
    })
}

/// Step 7: backup folders (they must be writable), interval and copies kept (SET-11).
pub fn save_backup(state: &AppState, step: &BackupSettings) -> Result<SetupStatus, AppError> {
    check_backup_folders(step)?;
    save_step(state, SetupStep::Backup, |tx, now| {
        settings::set(tx, step, None, now)?;
        Ok(())
    })
}

/// Step 8: "Add products later" (SET-12). Import and demo data come in a later task.
pub fn finish(state: &AppState) -> Result<SetupStatus, AppError> {
    save_step(state, SetupStep::Products, |_, _| Ok(()))
}

/// Runs one step's saving and marks the step done, in one transaction.
fn save_step(
    state: &AppState,
    step: SetupStep,
    save: impl FnOnce(&Transaction, pos_core::Timestamp) -> Result<(), AppError>,
) -> Result<SetupStatus, AppError> {
    session::require_setup_open(state)?;
    let now = state.clock.now();
    state.db.write(|tx| {
        save(tx, now)?;
        settings::complete_step(tx, step, now)?;
        Ok(settings::setup_status(tx)?)
    })
}

pub(crate) fn field_error(field: &str, code: &str) -> AppError {
    AppError::Validation {
        field: Some(field.into()),
        code: code.into(),
        params: Default::default(),
    }
}
