//! Units, categories and brands (rules CAT-02 and CAT-03).

#![allow(clippy::unwrap_used)]

mod common;

use catalog::{CatalogError, CategoryFields, NewProduct, UNCATEGORISED, UNIT_KG, UnitFields};
use common::{NOW, create, id, standard, test_db};

fn dozen() -> UnitFields {
    UnitFields {
        name: "dozen".to_string(),
        symbol: "dz".to_string(),
        unit_code: Some("DZN".to_string()),
        decimals: 0,
    }
}

#[test]
fn cat_02_built_in_units_are_there() {
    let db = test_db();
    let units: Vec<(String, u8, bool)> = catalog::list_units(&db, false)
        .unwrap()
        .into_iter()
        .map(|unit| (unit.name, unit.decimals, unit.is_system))
        .collect();
    let expected = [
        ("g", 0),
        ("kg", 3),
        ("litre", 3),
        ("ml", 0),
        ("pack", 0),
        ("piece", 0),
    ];
    for (name, decimals) in expected {
        assert!(
            units.contains(&(name.to_string(), decimals, true)),
            "unit {name}"
        );
    }
    assert_eq!(units.len(), 6);
    let kg = catalog::list_units(&db, false)
        .unwrap()
        .into_iter()
        .find(|unit| unit.id == id(UNIT_KG));
    assert_eq!(kg.unwrap().unit_code.as_deref(), Some("KGM"));
}

#[test]
fn cat_02_admin_adds_a_unit_with_0_to_3_decimals() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let unit_id = catalog::create_unit(&tx, &dozen()).unwrap();
    let saved = catalog::list_units(&tx, false).unwrap();
    assert!(
        saved
            .iter()
            .any(|unit| unit.id == unit_id && !unit.is_system)
    );

    let too_many = UnitFields {
        decimals: 4,
        name: "x".to_string(),
        ..dozen()
    };
    let error = catalog::create_unit(&tx, &too_many).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "unit_decimals_invalid",
            ..
        }
    ));

    let error = catalog::create_unit(&tx, &dozen()).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Conflict {
            code: "unit_name_in_use",
            ..
        }
    ));
}

#[test]
fn cat_02_unit_in_use_cannot_be_archived_or_change_decimals() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let unit_id = catalog::create_unit(&tx, &dozen()).unwrap();
    let eggs = NewProduct {
        unit_id: unit_id.clone(),
        ..standard("Eggs", 100)
    };
    catalog::create_product(&tx, eggs, NOW).unwrap();

    let error = catalog::archive_unit(&tx, &unit_id, NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Conflict {
            code: "unit_in_use",
            ..
        }
    ));

    let with_decimals = UnitFields {
        decimals: 2,
        ..dozen()
    };
    let error = catalog::update_unit(&tx, &unit_id, &with_decimals).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Conflict {
            code: "unit_in_use",
            ..
        }
    ));

    // Renaming is fine.
    let renamed = UnitFields {
        name: "dozen eggs".to_string(),
        ..dozen()
    };
    catalog::update_unit(&tx, &unit_id, &renamed).unwrap();
}

#[test]
fn cat_02_built_in_units_cannot_be_archived() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let error = catalog::archive_unit(&tx, &id(UNIT_KG), NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Conflict {
            code: "unit_is_built_in",
            ..
        }
    ));
}

#[test]
fn cat_02_unused_unit_can_be_archived() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let unit_id = catalog::create_unit(&tx, &dozen()).unwrap();
    catalog::archive_unit(&tx, &unit_id, NOW).unwrap();
    assert!(
        !catalog::list_units(&tx, false)
            .unwrap()
            .iter()
            .any(|unit| unit.id == unit_id)
    );
    assert!(
        catalog::list_units(&tx, true)
            .unwrap()
            .iter()
            .any(|unit| unit.id == unit_id)
    );
}

#[test]
fn cat_03_categories_form_a_tree() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let food = catalog::create_category(&tx, &CategoryFields::new("Food")).unwrap();
    let dairy = catalog::create_category(
        &tx,
        &CategoryFields {
            parent_id: Some(food.clone()),
            ..CategoryFields::new("Dairy")
        },
    )
    .unwrap();

    let categories = catalog::list_categories(&tx, false).unwrap();
    let saved = categories
        .iter()
        .find(|category| category.id == dairy)
        .unwrap();
    assert_eq!(saved.parent_id, Some(food.clone()));

    // A category can't be moved under itself or its own child.
    let loop_fields = CategoryFields {
        parent_id: Some(dairy.clone()),
        ..CategoryFields::new("Food")
    };
    let error = catalog::update_category(&tx, &food, &loop_fields).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "category_parent_loop",
            ..
        }
    ));
}

#[test]
fn cat_03_archiving_a_category_with_active_products_is_refused() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let dairy = catalog::create_category(&tx, &CategoryFields::new("Dairy")).unwrap();
    let milk = NewProduct {
        category_id: Some(dairy.clone()),
        ..standard("Milk", 100)
    };
    let milk_id = catalog::create_product(&tx, milk, NOW).unwrap();

    let error = catalog::archive_category(&tx, &dairy, NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Conflict {
            code: "category_has_products",
            ..
        }
    ));

    // Once the product is archived, the category can be archived too.
    catalog::archive_product(&tx, &milk_id, NOW).unwrap();
    catalog::archive_category(&tx, &dairy, NOW).unwrap();
    assert!(
        !catalog::list_categories(&tx, false)
            .unwrap()
            .iter()
            .any(|c| c.id == dairy)
    );
}

#[test]
fn cat_03_uncategorised_and_categories_with_children_stay() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let error = catalog::archive_category(&tx, &id(UNCATEGORISED), NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Conflict {
            code: "category_is_built_in",
            ..
        }
    ));

    let food = catalog::create_category(&tx, &CategoryFields::new("Food")).unwrap();
    let child = CategoryFields {
        parent_id: Some(food.clone()),
        ..CategoryFields::new("Dairy")
    };
    catalog::create_category(&tx, &child).unwrap();
    let error = catalog::archive_category(&tx, &food, NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Conflict {
            code: "category_has_children",
            ..
        }
    ));
}

#[test]
fn cat_03_brands_are_a_flat_list_with_unique_names() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let pran = catalog::create_brand(&tx, "Pran").unwrap();
    let error = catalog::create_brand(&tx, "Pran").unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Conflict {
            code: "brand_name_in_use",
            ..
        }
    ));

    catalog::archive_brand(&tx, &pran, NOW).unwrap();
    assert!(catalog::list_brands(&tx, false).unwrap().is_empty());
    assert_eq!(catalog::list_brands(&tx, true).unwrap().len(), 1);
}

#[test]
fn cat_01_product_with_an_unknown_or_archived_unit_or_category_is_refused() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let unknown_unit = NewProduct {
        unit_id: pos_core::Id::new(),
        ..standard("Milk", 100)
    };
    let error = catalog::create_product(&tx, unknown_unit, NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::NotFound { entity: "unit", .. }
    ));

    let dairy = catalog::create_category(&tx, &CategoryFields::new("Dairy")).unwrap();
    catalog::archive_category(&tx, &dairy, NOW).unwrap();
    let in_archived = NewProduct {
        category_id: Some(dairy),
        ..standard("Milk", 100)
    };
    let error = catalog::create_product(&tx, in_archived, NOW).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::NotFound {
            entity: "category",
            ..
        }
    ));
    tx.commit().unwrap();

    create(&mut db, standard("Ok", 100));
}
