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
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use pos_core::{Money, Timestamp};
    use crate::{TillSettings, CreditSettings, SettingsError, set};

    fn test_db() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        for migration in crate::migrations().migrations {
            connection.execute_batch(migration.sql).unwrap();
        }
        connection
    }

    #[test]
    fn snapshot_reads_all_defaults_when_empty() {
        let connection = test_db();
        let empty = snapshot(&connection).unwrap();
        assert_eq!(empty.till, TillSettings::default());
        assert_eq!(empty.scale_barcode_rules, ScaleRules::default());
    }

    #[test]
    fn snapshot_reads_overridden_settings() {
        let mut connection = test_db();

        let till = TillSettings {
            till_code: "T9".to_string(),
            ..TillSettings::default()
        };
        let transaction = connection.transaction().unwrap();
        set(&transaction, &till, None, Timestamp(1)).unwrap();
        transaction.commit().unwrap();

        let credit = CreditSettings {
            enabled: true,
            default_limit_minor: Money(100),
        };
        let transaction = connection.transaction().unwrap();
        set(&transaction, &credit, None, Timestamp(2)).unwrap();
        transaction.commit().unwrap();

        let snap = snapshot(&connection).unwrap();
        assert_eq!(snap.till, till);
        assert_eq!(snap.credit, credit);
        assert_eq!(snap.inventory, InventorySettings::default());
    }

    #[test]
    fn snapshot_fails_on_unreadable_setting() {
        let connection = test_db();
        connection
            .execute(
                "INSERT INTO settings_values (key, value_json, updated_at) VALUES ('till', '[1, 2]', 0)",
                [],
            )
            .unwrap();
        let error = snapshot(&connection).unwrap_err();
        match error {
            SettingsError::Unreadable { key, .. } => assert_eq!(key, "till"),
            other => panic!("expected Unreadable, got {:?}", other),
        }
    }
}
