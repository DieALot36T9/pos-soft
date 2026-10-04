//! Products: saving, SKUs, barcodes, archive, delete and price history (Req - Catalog).

#![allow(clippy::unwrap_used)]

mod common;

use catalog::{
    CatalogError, NewProduct, PriceField, ProductFilter, ProductKind, ProductPage, ProductUpdate,
    UNCATEGORISED, UNIT_KG,
};
use common::{NOW, create, create_with_barcode, id, standard, test_db, user};
use pos_core::{AppError, Money, Timestamp};

#[test]
fn cat_01_blank_sku_is_generated_in_order() {
    let mut db = test_db();
    let first = create(&mut db, standard("Milk 1L", 9000));
    let second = create(&mut db, standard("Bread", 5000));

    assert_eq!(catalog::get(&db, &first).unwrap().sku, "P000001");
    assert_eq!(catalog::get(&db, &second).unwrap().sku, "P000002");
}

#[test]
fn cat_01_generated_sku_skips_one_typed_by_hand() {
    let mut db = test_db();
    let typed = NewProduct {
        sku: Some("P000001".to_string()),
        ..standard("Typed", 100)
    };
    create(&mut db, typed);
    let generated = create(&mut db, standard("Generated", 100));

    assert_eq!(catalog::get(&db, &generated).unwrap().sku, "P000002");
}

#[test]
fn cat_01_duplicate_sku_is_refused() {
    let mut db = test_db();
    let with_sku = |name: &str| NewProduct {
        sku: Some("MILK-1".to_string()),
        ..standard(name, 100)
    };
    create(&mut db, with_sku("Milk"));

    let tx = db.transaction().unwrap();
    let error = catalog::create_product(&tx, with_sku("Other"), NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Conflict {
            code: "sku_in_use",
            ..
        }
    ));
}

#[test]
fn cat_01_name_is_required_and_at_most_120_characters() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();

    let blank = catalog::create_product(&tx, standard("   ", 100), NOW).unwrap_err();
    assert!(matches!(
        blank,
        CatalogError::Invalid {
            field: "name",
            code: "name_missing"
        }
    ));

    let long_name = "a".repeat(121);
    let long = catalog::create_product(&tx, standard(&long_name, 100), NOW).unwrap_err();
    assert!(matches!(
        long,
        CatalogError::Invalid {
            code: "name_too_long",
            ..
        }
    ));
}

#[test]
fn cat_01_missing_category_means_uncategorised() {
    let mut db = test_db();
    let product_id = create(&mut db, standard("Milk", 100));
    assert_eq!(
        catalog::get(&db, &product_id).unwrap().category_id,
        id(UNCATEGORISED)
    );
}

#[test]
fn cat_01_weighed_product_needs_a_unit_with_decimals() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let weighed_by_piece = NewProduct {
        is_weighed: true,
        ..standard("Potato", 6000)
    };
    let error = catalog::create_product(&tx, weighed_by_piece, NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "weighed_unit_needs_decimals",
            ..
        }
    ));

    let weighed_by_kg = NewProduct {
        is_weighed: true,
        unit_id: id(UNIT_KG),
        ..standard("Potato", 6000)
    };
    assert!(catalog::create_product(&tx, weighed_by_kg, NOW).is_ok());
}

#[test]
fn cat_04_duplicate_barcode_names_the_product_that_has_it() {
    let mut db = test_db();
    let milk = create_with_barcode(&mut db, "Milk 1L", "8901234567890", 9000);
    let other = create(&mut db, standard("Juice", 12000));
    let milk_sku = catalog::get(&db, &milk).unwrap().sku;

    let tx = db.transaction().unwrap();
    let error = catalog::add_barcode(&tx, &other, "8901234567890", NOW).unwrap_err();
    assert_eq!(
        error,
        CatalogError::BarcodeInUse {
            code: "8901234567890".to_string(),
            product_name: "Milk 1L".to_string(),
            sku: milk_sku.clone(),
        }
    );

    // The UI turns this into "Already used by Milk 1L (SKU P000001)".
    let AppError::Conflict { code, params } = AppError::from(error) else {
        panic!("expected a conflict");
    };
    assert_eq!(code, "barcode_in_use");
    assert_eq!(params["product_name"], "Milk 1L");
    assert_eq!(params["sku"], milk_sku);
}

#[test]
fn cat_04_duplicate_barcode_is_refused_when_creating_a_product() {
    let mut db = test_db();
    create_with_barcode(&mut db, "Milk 1L", "8901234567890", 9000);
    let tx = db.transaction().unwrap();
    let copy = NewProduct {
        barcodes: vec!["8901234567890".to_string()],
        ..standard("Copy", 100)
    };
    let error = catalog::create_product(&tx, copy, NOW).unwrap_err();
    assert!(matches!(error, CatalogError::BarcodeInUse { .. }));
}

#[test]
fn cat_04_first_barcode_is_primary_and_removed_barcodes_are_free_again() {
    let mut db = test_db();
    let milk = create_with_barcode(&mut db, "Milk", "111", 100);
    let juice = create(&mut db, standard("Juice", 100));

    let tx = db.transaction().unwrap();
    catalog::add_barcode(&tx, &milk, "222", NOW).unwrap();
    assert_eq!(
        catalog::get(&tx, &milk).unwrap().barcodes,
        vec!["111", "222"]
    );

    catalog::remove_barcode(&tx, "111").unwrap();
    catalog::add_barcode(&tx, &juice, "111", NOW).unwrap();
    assert_eq!(catalog::get(&tx, &juice).unwrap().barcodes, vec!["111"]);
}

#[test]
fn cat_08_archive_hides_from_find_by_code_and_search() {
    let mut db = test_db();
    let milk = create_with_barcode(&mut db, "Milk 1L", "8901234567890", 9000);
    assert!(
        catalog::find_by_code(&db, "8901234567890")
            .unwrap()
            .is_some()
    );

    let tx = db.transaction().unwrap();
    catalog::archive_product(&tx, &milk, NOW).unwrap();
    tx.commit().unwrap();

    assert_eq!(catalog::find_by_code(&db, "8901234567890").unwrap(), None);
    assert!(catalog::search(&db, "milk", 10).unwrap().is_empty());
    // Still kept for history and reports.
    assert_eq!(catalog::get(&db, &milk).unwrap().archived_at, Some(NOW));
}

#[test]
fn cat_08_archived_products_are_listed_only_when_asked() {
    let mut db = test_db();
    let milk = create(&mut db, standard("Milk", 100));
    create(&mut db, standard("Bread", 100));
    let tx = db.transaction().unwrap();
    catalog::archive_product(&tx, &milk, NOW).unwrap();
    tx.commit().unwrap();

    let page = ProductPage {
        offset: 0,
        limit: 50,
    };
    let active = catalog::list(&db, &ProductFilter::default(), page).unwrap();
    assert_eq!(active.total, 1);
    assert_eq!(active.rows[0].name, "Bread");

    let everything = ProductFilter {
        include_archived: true,
        ..ProductFilter::default()
    };
    assert_eq!(catalog::list(&db, &everything, page).unwrap().total, 2);
}

#[test]
fn cat_08_delete_is_refused_for_a_used_product() {
    let mut db = test_db();
    let milk = create_with_barcode(&mut db, "Milk", "111", 100);
    let tx = db.transaction().unwrap();

    let error = catalog::delete_unused_product(&tx, &milk, true).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Conflict {
            code: "product_in_use",
            ..
        }
    ));

    catalog::delete_unused_product(&tx, &milk, false).unwrap();
    assert!(matches!(
        catalog::get(&tx, &milk),
        Err(CatalogError::NotFound { .. })
    ));
    assert_eq!(catalog::find_by_code(&tx, "111").unwrap(), None);
}

#[test]
fn cat_09_price_change_rows_are_written_only_when_prices_change() {
    let mut db = test_db();
    let milk = create(&mut db, standard("Milk", 9000));
    let tx = db.transaction().unwrap();

    // Same prices, new name: no history row.
    let mut edit = ProductUpdate::from_product(&catalog::get(&tx, &milk).unwrap());
    edit.name = "Milk 1L".to_string();
    let changes = catalog::update_product(&tx, &milk, edit.clone(), &user(), NOW).unwrap();
    assert_eq!(changes.sale_price, None);
    assert!(catalog::price_history(&tx, &milk).unwrap().is_empty());

    // New sale price: one row with the old and new value, time and user.
    edit.sale_price = Money(9500);
    let later = Timestamp(NOW.0 + 1000);
    let changes = catalog::update_product(&tx, &milk, edit, &user(), later).unwrap();
    assert_eq!(changes.sale_price.unwrap().new, Money(9500));
    assert_eq!(changes.cost_price, None);

    let history = catalog::price_history(&tx, &milk).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].field, PriceField::SalePrice);
    assert_eq!((history[0].old, history[0].new), (Money(9000), Money(9500)));
    assert_eq!(history[0].changed_at, later);
    assert_eq!(history[0].changed_by, Some(user()));
    assert_eq!(history[0].reason.as_deref(), Some("manual"));
}

#[test]
fn cat_09_set_cost_records_the_reason_and_skips_unchanged_cost() {
    let mut db = test_db();
    let milk = create(&mut db, standard("Milk", 9000));
    let tx = db.transaction().unwrap();

    catalog::set_cost(&tx, &milk, Money(7000), "grn", &user(), NOW).unwrap();
    catalog::set_cost(&tx, &milk, Money(7000), "grn", &user(), NOW).unwrap();

    let history = catalog::price_history(&tx, &milk).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].field, PriceField::CostPrice);
    assert_eq!((history[0].old, history[0].new), (Money(0), Money(7000)));
    assert_eq!(history[0].reason.as_deref(), Some("grn"));
    assert_eq!(catalog::get(&tx, &milk).unwrap().cost_price, Money(7000));
}

#[test]
fn cat_01_negative_prices_are_refused() {
    let mut db = test_db();
    let milk = create(&mut db, standard("Milk", 9000));
    let tx = db.transaction().unwrap();
    let error = catalog::set_cost(&tx, &milk, Money(-1), "manual", &user(), NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "price_negative",
            ..
        }
    ));
}

#[test]
fn cat_01_scale_item_code_is_unique_and_found_by_scale_lookup() {
    let mut db = test_db();
    let beef = NewProduct {
        is_weighed: true,
        unit_id: id(UNIT_KG),
        scale_item_code: Some("00123".to_string()),
        ..standard("Beef", 75000)
    };
    create(&mut db, beef.clone());

    let found = catalog::find_by_scale_code(&db, "00123").unwrap().unwrap();
    assert_eq!(found.name, "Beef");
    assert!(found.is_weighed);
    assert_eq!(found.unit_decimals, 3);

    let tx = db.transaction().unwrap();
    let copy = NewProduct {
        name: "Beef copy".to_string(),
        ..beef
    };
    let error = catalog::create_product(&tx, copy, NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Conflict {
            code: "scale_item_code_in_use",
            ..
        }
    ));
}

#[test]
fn cat_11_internal_numbers_count_up_inside_the_transaction() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    assert_eq!(catalog::next_internal_number(&tx).unwrap(), 1);
    assert_eq!(catalog::next_internal_number(&tx).unwrap(), 2);
    tx.commit().unwrap();

    let tx = db.transaction().unwrap();
    assert_eq!(catalog::next_internal_number(&tx).unwrap(), 3);
}

#[test]
fn chk_01_find_by_code_tries_barcode_then_sku() {
    let mut db = test_db();
    let milk = create_with_barcode(&mut db, "Milk 1L", "8901234567890", 9000);

    let by_barcode = catalog::find_by_code(&db, "8901234567890")
        .unwrap()
        .unwrap();
    assert_eq!(by_barcode.id, milk);
    assert_eq!(by_barcode.sale_price, Money(9000));
    assert_eq!(by_barcode.kind, ProductKind::Standard);

    let by_sku = catalog::find_by_code(&db, "P000001").unwrap().unwrap();
    assert_eq!(by_sku.id, milk);
}

#[test]
fn chk_02_unknown_code_finds_nothing() {
    let db = test_db();
    assert_eq!(catalog::find_by_code(&db, "123").unwrap(), None);
}
