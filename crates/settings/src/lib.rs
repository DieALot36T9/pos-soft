//! Typed shop settings and setup progress.
//!
//! Page: `docs/05 Modules/Module - settings.md`.

mod currency;
mod devices;
mod error;
mod features;
mod setting;
mod setup;
mod shop;
mod snapshot;
mod store;

use pos_core::{Migration, ModuleMigrations};

pub use currency::{
    CurrencyChoice, Denominations, currency_choices, currency_format_for, suggested_denominations,
};
pub use devices::{
    BackupSettings, DrawerPin, HardwareSettings, InternalBarcode, LoggingSettings,
    PrinterConnection, ScaleKind, ScaleRule, ScaleRules, ScaleSettings,
};
pub use error::{Result, SettingsError};
pub use features::{
    CheckoutSettings, CostMethod, CreditSettings, ExpiredPolicy, InventorySettings, LoyaltySettings,
};
pub use setting::Setting;
pub use setup::{SetupStatus, SetupStep, complete_step, setup_status};
pub use shop::{
    LanguageSettings, PaperWidth, ReceiptSettings, ShopProfile, TaxMode, TaxSettings, TillSettings,
};
pub use snapshot::{SettingsSnapshot, snapshot};
pub use store::{get, set};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "settings",
        migrations: MIGRATIONS,
    }
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/001_init.sql"),
}];
