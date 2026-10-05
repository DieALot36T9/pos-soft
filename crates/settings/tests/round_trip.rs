//! Every typed setting saves and reads back unchanged (rule SET-13).

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

use pos_core::{
    CurrencyFormat, Grouping, Id, Money, RoundingAppliesTo, RoundingMode, RoundingRule, TaxMode,
    Timestamp,
};
use rusqlite::Connection;
use settings::{
    BackupSettings, CheckoutSettings, CostMethod, CreditSettings, Denominations, DrawerPin,
    ExpiredPolicy, HardwareSettings, InternalBarcode, InventorySettings, LanguageSettings,
    LoggingSettings, LoyaltySettings, PaperWidth, PrinterConnection, ReceiptSettings, ScaleKind,
    ScaleRule, ScaleRules, ScaleSettings, Setting, ShopProfile, TaxSettings, TillSettings,
};

/// An in-memory database built only by this module's own migrations (rule R6).
fn test_db() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    for migration in settings::migrations().migrations {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
}

/// Saves a value, reads it back and checks nothing changed.
fn round_trip<S: Setting + PartialEq + std::fmt::Debug>(connection: &mut Connection, value: S) {
    let transaction = connection.transaction().unwrap();
    settings::set(&transaction, &value, None, Timestamp(1)).unwrap();
    transaction.commit().unwrap();
    assert_eq!(settings::get::<S>(connection).unwrap(), value, "{}", S::KEY);
}

#[test]
fn set_13_shop_and_money_settings_survive_a_round_trip() {
    let mut connection = test_db();
    round_trip(&mut connection, LanguageSettings::default());
    round_trip(
        &mut connection,
        ShopProfile {
            name: "Rahim Store".to_string(),
            address_lines: vec!["12 Lake Road".to_string(), "Dhaka".to_string()],
            phone: "+8801711123456".to_string(),
            email: "shop@example.com".to_string(),
            tax_reg_label: "BIN".to_string(),
            tax_reg_no: "000123456-0101".to_string(),
            logo_path: Some("logo.png".to_string()),
            receipt_footer: vec!["Thank you".to_string()],
        },
    );
    round_trip(
        &mut connection,
        CurrencyFormat {
            code: "USD".to_string(),
            symbol: "$".to_string(),
            symbol_before: true,
            minor_digits: 2,
            decimal_sep: ".".to_string(),
            group_sep: ",".to_string(),
            grouping: Grouping::Western,
        },
    );
    round_trip(
        &mut connection,
        RoundingRule {
            increment_minor: Money(100),
            mode: RoundingMode::Nearest,
            applies_to: RoundingAppliesTo::CashOnly,
        },
    );
    round_trip(
        &mut connection,
        TaxSettings {
            label: "GST".to_string(),
            mode: TaxMode::Exclusive,
            default_rate_id: Some(Id::new()),
        },
    );
    round_trip(
        &mut connection,
        TillSettings {
            till_id: Some(Id::new()),
            till_code: "T2".to_string(),
            business_day_cutoff_hour: 3,
        },
    );
    round_trip(
        &mut connection,
        ReceiptSettings {
            paper_width: PaperWidth::Mm58,
            show_logo: false,
            copies: 2,
            header_lines: vec!["Open 8am to 10pm".to_string()],
            ..ReceiptSettings::default()
        },
    );
}

#[test]
fn set_13_device_and_feature_settings_survive_a_round_trip() {
    let mut connection = test_db();
    round_trip(
        &mut connection,
        HardwareSettings {
            receipt_printer: Some(PrinterConnection::Tcp {
                host: "192.168.1.50".to_string(),
                port: 9100,
            }),
            drawer_pin: DrawerPin::Pin5,
            scale: Some(ScaleSettings {
                port: "COM3".to_string(),
                baud: 9600,
                profile: "cas".to_string(),
            }),
            label_printer: Some(PrinterConnection::WindowsSpooler {
                printer_name: "TSC TE200".to_string(),
            }),
        },
    );
    round_trip(
        &mut connection,
        ScaleRules(vec![ScaleRule {
            name: "Weight".to_string(),
            pattern: "22.....{NNDDD}".to_string(),
            kind: ScaleKind::Weight,
        }]),
    );
    round_trip(
        &mut connection,
        InternalBarcode {
            prefix: "290".to_string(),
        },
    );
    round_trip(
        &mut connection,
        BackupSettings {
            primary_dir: "C:\\Shop\\backups".to_string(),
            secondary_dir: Some("E:\\backups".to_string()),
            interval_min: 30,
            keep_last: 20,
        },
    );
    round_trip(&mut connection, LoggingSettings { detailed: true });
    round_trip(
        &mut connection,
        CheckoutSettings {
            merge_scans: false,
            idle_lock_min: 10,
            blind_close: false,
            sounds: false,
        },
    );
    round_trip(
        &mut connection,
        InventorySettings {
            allow_negative: false,
            near_expiry_days: 14,
            expired_policy: ExpiredPolicy::Block,
            cost_method: CostMethod::LastCost,
        },
    );
    round_trip(
        &mut connection,
        LoyaltySettings {
            enabled: true,
            spend_unit_minor: Money(5_000),
            points_per_unit: 2,
            value_per_point_minor: Money(50),
            min_redeem: 10,
        },
    );
    round_trip(
        &mut connection,
        CreditSettings {
            enabled: true,
            default_limit_minor: Money(500_000),
        },
    );
    round_trip(
        &mut connection,
        Denominations(vec![Money(100_000), Money(50_000), Money(100)]),
    );
}
