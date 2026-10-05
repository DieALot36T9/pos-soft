//! All settings read at once, for workflows that need several of them.

use std::collections::BTreeMap;

use pos_core::{CurrencyFormat, RoundingRule};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::store::decode;
use crate::{
    BackupSettings, CheckoutSettings, CreditSettings, Denominations, HardwareSettings,
    InternalBarcode, InventorySettings, LanguageSettings, LoggingSettings, LoyaltySettings,
    ReceiptSettings, Result, ScaleRules, Setting, ShopProfile, TaxSettings, TillSettings,
};

/// Every setting, with defaults for the ones the shop hasn't saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SettingsSnapshot {
    pub language: LanguageSettings,
    pub shop_profile: ShopProfile,
    pub currency: CurrencyFormat,
    pub rounding: RoundingRule,
    pub tax: TaxSettings,
    pub till: TillSettings,
    pub receipt: ReceiptSettings,
    pub hardware: HardwareSettings,
    pub scale_barcode_rules: ScaleRules,
    pub internal_barcode: InternalBarcode,
    pub backup: BackupSettings,
    pub logging: LoggingSettings,
    pub checkout: CheckoutSettings,
    pub inventory: InventorySettings,
    pub loyalty: LoyaltySettings,
    pub credit: CreditSettings,
    pub denominations: Denominations,
}

/// Reads every setting with one query.
pub fn snapshot(conn: &Connection) -> Result<SettingsSnapshot> {
    let mut statement = conn.prepare("SELECT key, value_json FROM settings_values")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut stored = BTreeMap::new();
    for row in rows {
        let (key, json) = row?;
        stored.insert(key, json);
    }
    Ok(SettingsSnapshot {
        language: pick(&stored)?,
        shop_profile: pick(&stored)?,
        currency: pick(&stored)?,
        rounding: pick(&stored)?,
        tax: pick(&stored)?,
        till: pick(&stored)?,
        receipt: pick(&stored)?,
        hardware: pick(&stored)?,
        scale_barcode_rules: pick(&stored)?,
        internal_barcode: pick(&stored)?,
        backup: pick(&stored)?,
        logging: pick(&stored)?,
        checkout: pick(&stored)?,
        inventory: pick(&stored)?,
        loyalty: pick(&stored)?,
        credit: pick(&stored)?,
        denominations: pick(&stored)?,
    })
}

/// One setting from the stored rows, or its default when it isn't there.
fn pick<S: Setting>(stored: &BTreeMap<String, String>) -> Result<S> {
    match stored.get(S::KEY) {
        Some(json) => decode(S::KEY, json),
        None => Ok(S::default_value()),
    }
}
