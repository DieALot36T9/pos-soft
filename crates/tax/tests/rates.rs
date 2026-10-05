//! Saving and reading tax rates (TAX-02, TAX-03, TAX-07, TAX-10).

mod support;

use pos_core::{AppError, Bps, Id, Timestamp};
use support::{NOW, component, rate, test_db};
use tax::{RateComponent, RateUpdate, TaxError};

#[test]
fn tax_02_create_and_read_back_a_rate() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let id = tax::create_rate(&tx, rate("Reduced 7.5%", 750), NOW).unwrap();
    tx.commit().unwrap();

    let saved = tax::get(&db, &id).unwrap();
    assert_eq!(saved.id, id);
    assert_eq!(saved.name, "Reduced 7.5%");
    assert_eq!(saved.rate_bps, Bps(750));
    assert!(saved.active);
    assert!(saved.components.is_empty());
}

#[test]
fn tax_02_name_is_trimmed_and_required() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let id = tax::create_rate(&tx, rate("  Standard 15%  ", 1500), NOW).unwrap();
    assert_eq!(tax::get(&tx, &id).unwrap().name, "Standard 15%");

    let error = tax::create_rate(&tx, rate("   ", 1500), NOW).unwrap_err();
    assert!(matches!(error, TaxError::NameMissing));
}

#[test]
fn tax_02_rate_must_be_between_0_and_100_percent() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    assert!(tax::create_rate(&tx, rate("Exempt 0%", 0), NOW).is_ok());
    assert!(tax::create_rate(&tx, rate("All 100%", 10_000), NOW).is_ok());
    let error = tax::create_rate(&tx, rate("Too much", 10_001), NOW).unwrap_err();
    assert!(matches!(error, TaxError::RateOutOfRange));
}

#[test]
fn tax_02_two_rates_cannot_share_a_name() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    tax::create_rate(&tx, rate("Standard 15%", 1500), NOW).unwrap();
    // Names differing only in capitals would confuse the cashier too.
    let error = tax::create_rate(&tx, rate("standard 15%", 1500), NOW).unwrap_err();
    assert!(matches!(error, TaxError::NameTaken));
}

#[test]
fn tax_03_components_are_saved_in_order() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let mut gst = rate("GST 18%", 1800);
    gst.components = vec![component("CGST", 900), component("SGST", 900)];
    let id = tax::create_rate(&tx, gst, NOW).unwrap();

    let saved = tax::get(&tx, &id).unwrap();
    assert_eq!(
        saved.components,
        vec![
            RateComponent {
                name: "CGST".to_string(),
                rate_bps: Bps(900)
            },
            RateComponent {
                name: "SGST".to_string(),
                rate_bps: Bps(900)
            },
        ]
    );
}

#[test]
fn tax_03_components_must_add_up_to_the_rate() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let mut gst = rate("GST 18%", 1800);
    gst.components = vec![component("CGST", 900), component("SGST", 800)];
    let error = tax::create_rate(&tx, gst, NOW).unwrap_err();
    assert!(matches!(error, TaxError::ComponentsDoNotAddUp));
}

#[test]
fn tax_03_component_names_are_required() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let mut gst = rate("GST 18%", 1800);
    gst.components = vec![component("CGST", 900), component(" ", 900)];
    let error = tax::create_rate(&tx, gst, NOW).unwrap_err();
    assert!(matches!(error, TaxError::ComponentNameMissing));
}

#[test]
fn tax_07_update_changes_the_rate_and_replaces_components() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let mut gst = rate("GST 18%", 1800);
    gst.components = vec![component("CGST", 900), component("SGST", 900)];
    let id = tax::create_rate(&tx, gst, NOW).unwrap();

    let update = RateUpdate {
        name: "GST 12%".to_string(),
        rate_bps: Bps(1200),
        components: vec![component("CGST", 600), component("SGST", 600)],
        sort: 5,
    };
    tax::update_rate(&tx, &id, update, Timestamp(NOW.0 + 1)).unwrap();

    let saved = tax::get(&tx, &id).unwrap();
    assert_eq!(saved.name, "GST 12%");
    assert_eq!(saved.rate_bps, Bps(1200));
    assert_eq!(saved.sort, 5);
    assert_eq!(
        saved.components,
        vec![component("CGST", 600), component("SGST", 600)]
    );
}

#[test]
fn tax_07_update_checks_like_create() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let standard = tax::create_rate(&tx, rate("Standard 15%", 1500), NOW).unwrap();
    tax::create_rate(&tx, rate("Reduced 7.5%", 750), NOW).unwrap();

    let taken = RateUpdate {
        name: "Reduced 7.5%".to_string(),
        rate_bps: Bps(750),
        components: Vec::new(),
        sort: 0,
    };
    let error = tax::update_rate(&tx, &standard, taken, NOW).unwrap_err();
    assert!(matches!(error, TaxError::NameTaken));

    // Keeping its own name is fine.
    let same_name = RateUpdate {
        name: "Standard 15%".to_string(),
        rate_bps: Bps(1600),
        components: Vec::new(),
        sort: 0,
    };
    tax::update_rate(&tx, &standard, same_name, NOW).unwrap();
    assert_eq!(tax::get(&tx, &standard).unwrap().rate_bps, Bps(1600));
}

#[test]
fn tax_07_update_of_a_missing_rate_is_not_found() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let missing = Id::new();
    let update = RateUpdate {
        name: "Nothing".to_string(),
        rate_bps: Bps(0),
        components: Vec::new(),
        sort: 0,
    };
    let error = tax::update_rate(&tx, &missing, update, NOW).unwrap_err();
    assert!(matches!(error, TaxError::NotFound { .. }));
}

#[test]
fn tax_10_deactivated_rates_are_kept_but_left_out_of_the_active_list() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let standard = tax::create_rate(&tx, rate("Standard 15%", 1500), NOW).unwrap();
    let old = tax::create_rate(&tx, rate("Old 10%", 1000), NOW).unwrap();
    tax::deactivate(&tx, &old, NOW).unwrap();

    let active: Vec<Id> = tax::list(&tx, false)
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect();
    assert_eq!(active, vec![standard.clone()]);

    let everything = tax::list(&tx, true).unwrap();
    assert_eq!(everything.len(), 2);
    // Past sales still find the old rate by its id.
    assert!(!tax::get(&tx, &old).unwrap().active);
}

#[test]
fn tax_10_deactivating_a_missing_rate_is_not_found() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let error = tax::deactivate(&tx, &Id::new(), NOW).unwrap_err();
    assert!(matches!(error, TaxError::NotFound { .. }));
}

#[test]
fn tax_10_rates_cannot_be_deleted_in_the_database_either() {
    // There is no delete function, and the database refuses a hand-written DELETE too.
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let id = tax::create_rate(&tx, rate("Standard 15%", 1500), NOW).unwrap();
    let deleted = tx.execute("DELETE FROM tax_rates WHERE id = ?1", [id.as_str()]);
    assert!(deleted.is_err());
    assert!(tax::get(&tx, &id).is_ok());
}

#[test]
fn list_is_sorted_by_sort_then_name() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let mut zero = rate("Zero 0%", 0);
    zero.sort = 2;
    tax::create_rate(&tx, zero, NOW).unwrap();
    tax::create_rate(&tx, rate("Standard 15%", 1500), NOW).unwrap();
    tax::create_rate(&tx, rate("Reduced 7.5%", 750), NOW).unwrap();

    let names: Vec<String> = tax::list(&tx, true)
        .unwrap()
        .into_iter()
        .map(|r| r.name)
        .collect();
    assert_eq!(names, vec!["Reduced 7.5%", "Standard 15%", "Zero 0%"]);
}

#[test]
fn get_many_returns_the_rates_it_finds() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let standard = tax::create_rate(&tx, rate("Standard 15%", 1500), NOW).unwrap();
    let reduced = tax::create_rate(&tx, rate("Reduced 7.5%", 750), NOW).unwrap();
    let missing = Id::new();

    let found = tax::get_many(&tx, &[standard.clone(), reduced.clone(), missing.clone()]).unwrap();
    assert_eq!(found.len(), 2);
    assert_eq!(found[&standard].rate_bps, Bps(1500));
    assert_eq!(found[&reduced].rate_bps, Bps(750));
    assert!(!found.contains_key(&missing));
}

#[test]
fn errors_turn_into_app_errors_the_ui_understands() {
    assert_eq!(
        AppError::from(TaxError::NameMissing),
        AppError::Validation {
            field: Some("name".to_string()),
            code: "tax_rate_name_missing".to_string(),
            params: Default::default(),
        }
    );
    assert!(matches!(
        AppError::from(TaxError::NameTaken),
        AppError::Conflict { code, .. } if code == "tax_rate_name_taken"
    ));
    let id = Id::new();
    assert_eq!(
        AppError::from(TaxError::NotFound { id: id.clone() }),
        AppError::NotFound {
            entity: "tax_rate".to_string(),
            key: id.to_string()
        }
    );
}
