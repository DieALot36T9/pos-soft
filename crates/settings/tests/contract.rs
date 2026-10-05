//! Contract tests: the public API behaves as `docs/05 Modules/Module - settings.md` says.
//! Each test names the requirement it covers. The module publishes no views.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

use pos_core::{AppError, CurrencyFormat, Grouping, Id, Money, Timestamp};
use rusqlite::Connection;
use settings::{
    CheckoutSettings, CreditSettings, InventorySettings, LoggingSettings, ScaleRules, Setting,
    SettingsError, SetupStep, TillSettings,
};

/// An in-memory database built only by this module's own migrations (rule R6).
fn test_db() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    for migration in settings::migrations().migrations {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
}

/// Saves a value in its own transaction and returns the old value.
fn save<S: Setting>(connection: &mut Connection, value: &S) -> Result<Option<S>, SettingsError> {
    let transaction = connection.transaction().unwrap();
    let old = settings::set(&transaction, value, None, Timestamp(1_000))?;
    transaction.commit().unwrap();
    Ok(old)
}

fn validation_code(error: SettingsError) -> String {
    match AppError::from(error) {
        AppError::Validation { code, .. } => code,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn migrations_belong_to_this_module() {
    let migrations = settings::migrations();
    assert_eq!(migrations.module, "settings");
    assert_eq!(migrations.migrations[0].version, 1);
}

#[test]
fn set_13_missing_keys_return_defaults() {
    let connection = test_db();
    assert_eq!(
        settings::get::<TillSettings>(&connection).unwrap(),
        TillSettings::default()
    );
    let till = settings::get::<TillSettings>(&connection).unwrap();
    assert_eq!(till.till_code, "T1");
    assert_eq!(till.business_day_cutoff_hour, 4);
    assert_eq!(
        settings::get::<CurrencyFormat>(&connection).unwrap(),
        CurrencyFormat::default_value()
    );
}

#[test]
fn set_13_set_returns_the_old_value_for_the_audit_log() {
    let mut connection = test_db();
    let first = LoggingSettings { detailed: true };
    assert_eq!(save(&mut connection, &first).unwrap(), None);
    let second = LoggingSettings { detailed: false };
    assert_eq!(save(&mut connection, &second).unwrap(), Some(first));
    assert_eq!(save(&mut connection, &second).unwrap(), Some(second));
}

#[test]
fn set_13_set_records_who_and_when() {
    let mut connection = test_db();
    let admin = Id::new();
    let transaction = connection.transaction().unwrap();
    settings::set(
        &transaction,
        &LoggingSettings { detailed: true },
        Some(&admin),
        Timestamp(42),
    )
    .unwrap();
    transaction.commit().unwrap();
    let (updated_at, updated_by): (i64, Option<String>) = connection
        .query_row(
            "SELECT updated_at, updated_by FROM settings_values WHERE key = 'logging'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(updated_at, 42);
    assert_eq!(updated_by.as_deref(), Some(admin.as_str()));
}

#[test]
fn set_09_invalid_values_are_refused_and_not_saved() {
    let mut connection = test_db();
    let bad_cutoff = TillSettings {
        business_day_cutoff_hour: 24,
        ..TillSettings::default()
    };
    let error = save(&mut connection, &bad_cutoff).unwrap_err();
    assert_eq!(validation_code(error), "cutoff_hour_out_of_range");
    assert_eq!(
        settings::get::<TillSettings>(&connection).unwrap(),
        TillSettings::default()
    );
}

#[test]
fn set_05_four_minor_digits_are_refused() {
    let mut connection = test_db();
    let currency = CurrencyFormat {
        minor_digits: 4,
        ..CurrencyFormat::default_value()
    };
    let error = save(&mut connection, &currency).unwrap_err();
    assert_eq!(validation_code(error), "minor_digits_out_of_range");
}

#[test]
fn a_stored_value_that_cannot_be_read_is_an_internal_error() {
    let connection = test_db();
    connection
        .execute(
            "INSERT INTO settings_values (key, value_json, updated_at) VALUES ('till', '[1, 2]', 0)",
            [],
        )
        .unwrap();
    let error = settings::get::<TillSettings>(&connection).unwrap_err();
    assert!(matches!(AppError::from(error), AppError::Internal { .. }));
}

#[test]
fn set_13_fields_missing_from_older_saves_get_their_defaults() {
    let connection = test_db();
    // A save from an older version that only knew about `merge_scans`.
    connection
        .execute(
            "INSERT INTO settings_values (key, value_json, updated_at)
             VALUES ('checkout', '{\"merge_scans\": false}', 0)",
            [],
        )
        .unwrap();
    let checkout = settings::get::<CheckoutSettings>(&connection).unwrap();
    assert!(!checkout.merge_scans);
    assert_eq!(checkout.idle_lock_min, 5);
    assert!(checkout.blind_close);
}

#[test]
fn set_02_setup_status_ordering_and_resume() {
    let mut connection = test_db();
    let status = settings::setup_status(&connection).unwrap();
    assert!(status.completed.is_empty());
    assert_eq!(status.next_step, Some(SetupStep::Shop));
    assert!(!status.is_complete);

    // The wizard was closed after step 3.
    let transaction = connection.transaction().unwrap();
    for step in [SetupStep::Shop, SetupStep::Currency, SetupStep::Tax] {
        settings::complete_step(&transaction, step, Timestamp(1)).unwrap();
    }
    transaction.commit().unwrap();

    let status = settings::setup_status(&connection).unwrap();
    assert_eq!(
        status.completed,
        vec![SetupStep::Shop, SetupStep::Currency, SetupStep::Tax]
    );
    assert_eq!(status.next_step, Some(SetupStep::Admin));
    assert!(!status.is_complete);
}

#[test]
fn set_02_completed_steps_are_listed_in_wizard_order() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    settings::complete_step(&transaction, SetupStep::Till, Timestamp(1)).unwrap();
    settings::complete_step(&transaction, SetupStep::Shop, Timestamp(2)).unwrap();
    settings::complete_step(&transaction, SetupStep::Shop, Timestamp(3)).unwrap();
    transaction.commit().unwrap();
    let status = settings::setup_status(&connection).unwrap();
    assert_eq!(status.completed, vec![SetupStep::Shop, SetupStep::Till]);
    assert_eq!(status.next_step, Some(SetupStep::Currency));
}

#[test]
fn set_10_setup_is_complete_without_the_hardware_step() {
    let mut connection = test_db();
    let transaction = connection.transaction().unwrap();
    for step in SetupStep::ALL {
        if step != SetupStep::Hardware {
            settings::complete_step(&transaction, step, Timestamp(1)).unwrap();
        }
    }
    transaction.commit().unwrap();
    let status = settings::setup_status(&connection).unwrap();
    assert!(status.is_complete);
    assert_eq!(status.next_step, Some(SetupStep::Hardware));

    let transaction = connection.transaction().unwrap();
    settings::complete_step(&transaction, SetupStep::Hardware, Timestamp(2)).unwrap();
    transaction.commit().unwrap();
    let status = settings::setup_status(&connection).unwrap();
    assert!(status.is_complete);
    assert_eq!(status.next_step, None);
}

#[test]
fn set_13_snapshot_holds_every_setting() {
    let mut connection = test_db();
    let empty = settings::snapshot(&connection).unwrap();
    assert_eq!(empty.till, TillSettings::default());
    assert_eq!(empty.scale_barcode_rules, ScaleRules::default());

    let till = TillSettings {
        till_code: "T9".to_string(),
        ..TillSettings::default()
    };
    save(&mut connection, &till).unwrap();
    let credit = CreditSettings {
        enabled: true,
        default_limit_minor: Money(100),
    };
    save(&mut connection, &credit).unwrap();

    let snapshot = settings::snapshot(&connection).unwrap();
    assert_eq!(snapshot.till, till);
    assert_eq!(snapshot.credit, credit);
    assert_eq!(snapshot.inventory, InventorySettings::default());
}

#[test]
fn set_05_currency_list_comes_from_iso_4217() {
    let choices = settings::currency_choices();
    let taka = choices.iter().find(|choice| choice.code == "BDT").unwrap();
    assert_eq!(taka.minor_digits, 2);
    assert!(choices.iter().all(|choice| choice.minor_digits <= 3));
    let yen = choices.iter().find(|choice| choice.code == "JPY").unwrap();
    assert_eq!(yen.minor_digits, 0);

    let format = settings::currency_format_for("INR").unwrap();
    assert_eq!(format.minor_digits, 2);
    assert_eq!(format.grouping, Grouping::Indian);
    assert!(settings::currency_format_for("XYZ").is_err());
}

#[test]
fn set_16_denominations_are_suggested_for_known_currencies() {
    let taka = settings::suggested_denominations("BDT", 2);
    assert_eq!(taka.0.first(), Some(&Money(100_000)));
    assert_eq!(taka.0.last(), Some(&Money(100)));
    // With no decimals, 0.50 coins can't be counted and are left out.
    let whole_dollars = settings::suggested_denominations("USD", 0);
    assert!(whole_dollars.0.iter().all(|value| value.0 >= 1));
    assert_eq!(whole_dollars.0.last(), Some(&Money(1)));
    assert!(settings::suggested_denominations("XYZ", 2).0.is_empty());
}
