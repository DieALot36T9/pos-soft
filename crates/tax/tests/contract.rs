//! Contract tests: the public API and views behave as the module page says
//! (`docs/05 Modules/Module - tax.md`). Other modules and `app` rely on these.

mod support;

use pos_core::{Bps, Money};
use support::{NOW, component, rate, test_db};
use tax::{TaxMode, calc_line, split_components};

#[test]
fn migrations_belong_to_this_module() {
    assert_eq!(tax::migrations().module, "tax");
}

#[test]
fn migrations_are_numbered_from_one_without_gaps() {
    let versions: Vec<u32> = tax::migrations()
        .migrations
        .iter()
        .map(|m| m.version)
        .collect();
    let expected: Vec<u32> = (1..=versions.len() as u32).collect();
    assert_eq!(versions, expected);
}

#[test]
fn every_table_and_view_starts_with_tax() {
    let db = test_db();
    let mut statement = db
        .prepare("SELECT name FROM sqlite_schema WHERE type IN ('table', 'view', 'index', 'trigger') AND name NOT LIKE 'sqlite_%'")
        .unwrap();
    let names: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|name| name.unwrap())
        .collect();
    assert!(!names.is_empty());
    for name in names {
        assert!(name.starts_with("tax_"), "{name} is not prefixed with tax_");
    }
}

#[test]
fn view_tax_v_rates_has_the_published_columns() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let standard = tax::create_rate(&tx, rate("Standard 15%", 1500), NOW).unwrap();
    let old = tax::create_rate(&tx, rate("Old 10%", 1000), NOW).unwrap();
    tax::deactivate(&tx, &old, NOW).unwrap();
    tx.commit().unwrap();

    let mut statement = db
        .prepare("SELECT id, name, rate_bps, active FROM tax_v_rates ORDER BY name")
        .unwrap();
    let columns: Vec<String> = statement
        .column_names()
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(columns, vec!["id", "name", "rate_bps", "active"]);
    let rows: Vec<(String, String, i64, i64)> = statement
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .map(|row| row.unwrap())
        .collect();
    assert_eq!(
        rows,
        vec![
            (old.to_string(), "Old 10%".to_string(), 1000, 0),
            (standard.to_string(), "Standard 15%".to_string(), 1500, 1),
        ]
    );
}

#[test]
fn saved_rate_feeds_the_calculation() {
    // How `app` uses this module at checkout: load the rate, calculate, then split.
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let mut gst = rate("GST 18%", 1800);
    gst.components = vec![component("CGST", 900), component("SGST", 900)];
    let id = tax::create_rate(&tx, gst, NOW).unwrap();

    let saved = tax::get(&tx, &id).unwrap();
    let line = calc_line(Money(10000), saved.rate_bps, TaxMode::Exclusive).unwrap();
    assert_eq!(line.tax, Money(1800));
    let parts = split_components(line.tax, &saved).unwrap();
    assert_eq!(
        parts,
        vec![
            ("CGST".to_string(), Money(900)),
            ("SGST".to_string(), Money(900))
        ]
    );
    assert_eq!(saved.rate_bps, Bps(1800));
}

#[test]
fn types_the_ui_uses_are_sent_as_plain_json() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let mut gst = rate("GST 18%", 1800);
    gst.components = vec![component("CGST", 900), component("SGST", 900)];
    let id = tax::create_rate(&tx, gst, NOW).unwrap();
    let json = serde_json::to_string(&tax::get(&tx, &id).unwrap()).unwrap();
    assert_eq!(
        json,
        format!(
            r#"{{"id":"{id}","name":"GST 18%","rate_bps":1800,"active":true,"sort":0,"components":[{{"name":"CGST","rate_bps":900}},{{"name":"SGST","rate_bps":900}}]}}"#
        )
    );
}
