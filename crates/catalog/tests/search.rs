//! Product search (rules CAT-05 and CHK-03).

#![allow(clippy::unwrap_used)]

mod common;

use catalog::{CategoryFields, NewProduct};
use common::{NOW, create, create_with_barcode, search_names, standard, test_db};

#[test]
fn cat_05_prefix_finds_the_product() {
    let mut db = test_db();
    create(&mut db, standard("Milk 1L", 9000));
    create(&mut db, standard("Bread", 5000));

    assert_eq!(search_names(&db, "mil"), vec!["Milk 1L"]);
    assert_eq!(search_names(&db, "MI"), vec!["Milk 1L"]);
}

#[test]
fn cat_05_every_word_must_match() {
    let mut db = test_db();
    create(&mut db, standard("Mango Juice 1L", 100));
    create(&mut db, standard("Mango Pickle", 100));

    assert_eq!(search_names(&db, "man jui"), vec!["Mango Juice 1L"]);
}

#[test]
fn cat_05_accented_letters_match_plain_letters() {
    let mut db = test_db();
    create(&mut db, standard("Café Crème", 100));

    assert_eq!(search_names(&db, "cafe"), vec!["Café Crème"]);
    assert_eq!(search_names(&db, "crème"), vec!["Café Crème"]);
}

#[test]
fn cat_05_finds_by_sku_barcode_category_and_brand() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let dairy = catalog::create_category(&tx, &CategoryFields::new("Dairy")).unwrap();
    let brand = catalog::create_brand(&tx, "Aarong").unwrap();
    let milk = NewProduct {
        category_id: Some(dairy),
        brand_id: Some(brand),
        barcodes: vec!["8901234567890".to_string()],
        ..standard("Milk 1L", 9000)
    };
    catalog::create_product(&tx, milk, NOW).unwrap();
    tx.commit().unwrap();

    for query in [
        "P000001",
        "p0000",
        "8901234567890",
        "890123",
        "dairy",
        "aaro",
    ] {
        assert_eq!(search_names(&db, query), vec!["Milk 1L"], "query {query}");
    }
}

#[test]
fn cat_05_exact_barcode_or_sku_is_ranked_first() {
    let mut db = test_db();
    // "Milk" matches the word "milk" well, but the exact barcode wins.
    create(&mut db, standard("Milk Milk Milk", 100));
    let code_owner = create_with_barcode(&mut db, "Cheese", "milk", 100);

    let hits = catalog::search(&db, "milk", 10).unwrap();
    assert_eq!(hits[0].product_id, code_owner);
    assert!(hits[0].exact_match);
    assert_eq!(hits.len(), 2);
    assert!(!hits[1].exact_match);
}

#[test]
fn cat_05_search_row_follows_barcode_changes() {
    let mut db = test_db();
    let milk = create(&mut db, standard("Milk", 100));
    assert!(search_names(&db, "4455").is_empty());

    let tx = db.transaction().unwrap();
    catalog::add_barcode(&tx, &milk, "44556677", NOW).unwrap();
    assert_eq!(search_names(&tx, "4455"), vec!["Milk"]);

    catalog::remove_barcode(&tx, "44556677").unwrap();
    assert!(search_names(&tx, "4455").is_empty());
}

#[test]
fn cat_05_search_row_follows_category_and_brand_renames() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let category = catalog::create_category(&tx, &CategoryFields::new("Dairy")).unwrap();
    let brand = catalog::create_brand(&tx, "Aarong").unwrap();
    let milk = NewProduct {
        category_id: Some(category.clone()),
        brand_id: Some(brand.clone()),
        ..standard("Milk", 100)
    };
    catalog::create_product(&tx, milk, NOW).unwrap();

    catalog::update_category(&tx, &category, &CategoryFields::new("Chilled")).unwrap();
    catalog::rename_brand(&tx, &brand, "Pran").unwrap();

    assert!(search_names(&tx, "dairy").is_empty());
    assert!(search_names(&tx, "aarong").is_empty());
    assert_eq!(search_names(&tx, "chilled"), vec!["Milk"]);
    assert_eq!(search_names(&tx, "pran"), vec!["Milk"]);
}

#[test]
fn cat_05_search_row_follows_product_edits() {
    let mut db = test_db();
    let milk = create(&mut db, standard("Milk", 100));
    let tx = db.transaction().unwrap();
    let mut edit = catalog::ProductUpdate::from_product(&catalog::get(&tx, &milk).unwrap());
    edit.name = "Yoghurt".to_string();
    catalog::update_product(&tx, &milk, edit, &common::user(), NOW).unwrap();

    assert!(search_names(&tx, "milk").is_empty());
    assert_eq!(search_names(&tx, "yog"), vec!["Yoghurt"]);
}

#[test]
fn chk_03_search_ignores_symbols_and_respects_the_limit() {
    let mut db = test_db();
    for number in 0..5 {
        create(&mut db, standard(&format!("Soap {number}"), 100));
    }
    assert_eq!(catalog::search(&db, "soap", 3).unwrap().len(), 3);
    // Search syntax typed by a user must not break the query.
    assert_eq!(catalog::search(&db, "\"soap*\" (", 10).unwrap().len(), 5);
    assert!(catalog::search(&db, "  ", 10).unwrap().is_empty());
}

#[test]
fn cat_05_rebuild_search_gives_the_same_results() {
    let mut db = test_db();
    create_with_barcode(&mut db, "Milk", "111222", 100);
    let tx = db.transaction().unwrap();
    catalog::rebuild_search(&tx).unwrap();
    assert_eq!(search_names(&tx, "111"), vec!["Milk"]);
    assert_eq!(search_names(&tx, "mil"), vec!["Milk"]);
}
