//! Variants and kits (rules CAT-06, CAT-07, CHK-14, CHK-15).

#![allow(clippy::unwrap_used)]

mod common;

use catalog::{CatalogError, NewProduct, ProductKind};
use common::{NOW, create, create_with_barcode, search_names, standard, test_db, vat};
use pos_core::{Money, Qty};

fn shampoo_parent(db: &mut rusqlite::Connection) -> pos_core::Id {
    let parent = NewProduct {
        kind: ProductKind::VariantParent,
        sale_price: Money(25000),
        ..NewProduct::new(
            ProductKind::VariantParent,
            "Shampoo",
            common::id(catalog::UNIT_PIECE),
            vat(),
        )
    };
    create(db, parent)
}

fn size_and_flavour() -> Vec<(String, Vec<String>)> {
    vec![
        (
            "Size".to_string(),
            vec!["200 ml".to_string(), "400 ml".to_string()],
        ),
        (
            "Flavour".to_string(),
            vec!["Mango".to_string(), "Lemon".to_string()],
        ),
    ]
}

#[test]
fn cat_06_generate_variants_makes_every_combination() {
    let mut db = test_db();
    let parent = shampoo_parent(&mut db);
    let tx = db.transaction().unwrap();

    let variants = catalog::generate_variants(&tx, &parent, &size_and_flavour(), NOW).unwrap();
    assert_eq!(variants.len(), 4);

    let first = catalog::get(&tx, &variants[0]).unwrap();
    assert_eq!(first.kind, ProductKind::Variant);
    assert_eq!(first.parent_id, Some(parent.clone()));
    assert_eq!(first.name, "Shampoo 200 ml Mango");
    assert_eq!(first.variant_label.as_deref(), Some("200 ml / Mango"));
    assert_eq!(first.sale_price, Money(25000));
    assert_eq!(
        first.attributes,
        vec![
            ("Flavour".to_string(), "Mango".to_string()),
            ("Size".to_string(), "200 ml".to_string())
        ]
    );

    let names: Vec<String> = variants
        .iter()
        .map(|variant| catalog::get(&tx, variant).unwrap().name)
        .collect();
    assert_eq!(
        names,
        vec![
            "Shampoo 200 ml Mango",
            "Shampoo 200 ml Lemon",
            "Shampoo 400 ml Mango",
            "Shampoo 400 ml Lemon"
        ]
    );
    // Each variant gets its own SKU.
    let skus: Vec<String> = variants
        .iter()
        .map(|variant| catalog::get(&tx, variant).unwrap().sku)
        .collect();
    assert_eq!(skus, vec!["P000002", "P000003", "P000004", "P000005"]);
}

#[test]
fn cat_06_generating_again_adds_only_new_combinations() {
    let mut db = test_db();
    let parent = shampoo_parent(&mut db);
    let tx = db.transaction().unwrap();
    catalog::generate_variants(&tx, &parent, &size_and_flavour(), NOW).unwrap();

    let mut more = size_and_flavour();
    more[1].1.push("Rose".to_string());
    let added = catalog::generate_variants(&tx, &parent, &more, NOW).unwrap();
    assert_eq!(added.len(), 2);
}

#[test]
fn cat_06_only_a_parent_can_have_variants() {
    let mut db = test_db();
    let soap = create(&mut db, standard("Soap", 100));
    let tx = db.transaction().unwrap();
    let error = catalog::generate_variants(&tx, &soap, &size_and_flavour(), NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "not_a_variant_parent",
            ..
        }
    ));
}

#[test]
fn chk_15_parent_is_not_sellable_but_its_variants_are() {
    let mut db = test_db();
    let parent = shampoo_parent(&mut db);
    let tx = db.transaction().unwrap();
    let variants = catalog::generate_variants(&tx, &parent, &size_and_flavour(), NOW).unwrap();
    catalog::add_barcode(&tx, &variants[0], "5550001", NOW).unwrap();

    // The parent can't be found by its SKU and has no barcode.
    assert_eq!(catalog::find_by_code(&tx, "P000001").unwrap(), None);
    let error = catalog::add_barcode(&tx, &parent, "5550002", NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "parent_not_sellable",
            ..
        }
    ));

    // Search lists the variants, not the parent.
    let names = search_names(&tx, "shampoo");
    assert_eq!(names.len(), 4);
    assert!(!names.contains(&"Shampoo".to_string()));

    // Scanning a variant's barcode finds that variant.
    let found = catalog::find_by_code(&tx, "5550001").unwrap().unwrap();
    assert_eq!(found.id, variants[0]);
    assert_eq!(found.kind, ProductKind::Variant);
}

#[test]
fn cat_06_parent_holds_no_stock() {
    let mut db = test_db();
    let parent = shampoo_parent(&mut db);
    assert!(!catalog::get(&db, &parent).unwrap().track_stock);
}

fn eid_box(db: &mut rusqlite::Connection) -> pos_core::Id {
    let kit = NewProduct::new(
        ProductKind::Kit,
        "Eid Box",
        common::id(catalog::UNIT_PIECE),
        vat(),
    );
    create(db, kit)
}

#[test]
fn chk_14_kit_components_are_saved_with_quantities() {
    let mut db = test_db();
    let sugar = create_with_barcode(&mut db, "Sugar 1kg", "111", 12000);
    let oil = create_with_barcode(&mut db, "Oil 1L", "222", 18000);
    let kit = eid_box(&mut db);
    let tx = db.transaction().unwrap();

    let components = vec![(sugar.clone(), Qty(2000)), (oil.clone(), Qty(1000))];
    catalog::set_kit_components(&tx, &kit, &components).unwrap();

    let mut saved = catalog::kit_components(&tx, &kit).unwrap();
    saved.sort();
    let mut expected = components;
    expected.sort();
    assert_eq!(saved, expected);
    // Kits have no stock of their own (CAT-07).
    assert!(!catalog::get(&tx, &kit).unwrap().track_stock);
}

#[test]
fn cat_07_kit_cannot_contain_itself() {
    let mut db = test_db();
    let kit = eid_box(&mut db);
    let tx = db.transaction().unwrap();
    let error = catalog::set_kit_components(&tx, &kit, &[(kit.clone(), Qty(1000))]).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "kit_contains_itself",
            ..
        }
    ));
}

#[test]
fn cat_07_kit_cannot_contain_another_kit() {
    let mut db = test_db();
    let kit = eid_box(&mut db);
    let other_kit = eid_box(&mut db);
    let tx = db.transaction().unwrap();
    let error = catalog::set_kit_components(&tx, &kit, &[(other_kit, Qty(1000))]).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "kit_contains_kit",
            ..
        }
    ));
}

#[test]
fn cat_07_kit_quantities_must_be_above_zero_and_listed_once() {
    let mut db = test_db();
    let sugar = create(&mut db, standard("Sugar", 100));
    let kit = eid_box(&mut db);
    let tx = db.transaction().unwrap();

    let zero = catalog::set_kit_components(&tx, &kit, &[(sugar.clone(), Qty(0))]).unwrap_err();
    assert!(matches!(
        zero,
        CatalogError::Invalid {
            code: "kit_qty_not_positive",
            ..
        }
    ));

    let twice = [(sugar.clone(), Qty(1000)), (sugar, Qty(2000))];
    let error = catalog::set_kit_components(&tx, &kit, &twice).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "kit_component_repeated",
            ..
        }
    ));
}

#[test]
fn cat_07_only_a_kit_can_have_components() {
    let mut db = test_db();
    let sugar = create(&mut db, standard("Sugar", 100));
    let soap = create(&mut db, standard("Soap", 100));
    let tx = db.transaction().unwrap();
    let error = catalog::set_kit_components(&tx, &soap, &[(sugar, Qty(1000))]).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "not_a_kit",
            ..
        }
    ));
}
