//! Contract tests: the public API and views behave as the module page says.
//! Other modules and `app` rely on these. Change them only together with the page (rule R5).

#![allow(clippy::unwrap_used)]

mod common;

use catalog::{CategoryFields, NewProduct, ProductFilter, ProductKind, ProductPage, UNCATEGORISED};
use common::{NOW, create, create_with_barcode, id, standard, test_db, vat};
use pos_core::{AppError, Money};

#[test]
fn migrations_belong_to_this_module() {
    let migrations = catalog::migrations();
    assert_eq!(migrations.module, "catalog");
    assert_eq!(migrations.migrations[0].version, 1);
}

#[test]
fn every_table_and_view_has_the_catalog_prefix() {
    let db = test_db();
    let mut statement = db
        .prepare("SELECT name FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%'")
        .unwrap();
    let names: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|name| name.unwrap())
        .collect();
    assert!(!names.is_empty());
    for name in names {
        assert!(name.starts_with("catalog_"), "{name}");
    }
}

#[test]
fn view_catalog_v_products_has_the_published_columns() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let brand = catalog::create_brand(&tx, "Pran").unwrap();
    let milk = NewProduct {
        brand_id: Some(brand),
        cost_price: Money(7000),
        ..standard("Milk 1L", 9000)
    };
    let milk_id = catalog::create_product(&tx, milk, NOW).unwrap();
    tx.commit().unwrap();

    let row = db
        .query_row(
            "SELECT id, kind, name, sku, category_id, category_name, brand_id, brand_name,
                    unit_symbol, unit_decimals, sale_price_minor, cost_price_minor,
                    tax_rate_id, track_stock, archived_at
             FROM catalog_v_products",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, Option<i64>>(14)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        row,
        (
            milk_id.to_string(),
            "standard".to_string(),
            "Milk 1L".to_string(),
            "Uncategorised".to_string(),
            "Pran".to_string(),
            "pc".to_string(),
            9000,
            7000,
            vat().to_string(),
            None
        )
    );
}

#[test]
fn view_catalog_v_categories_has_id_parent_and_name() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let food = catalog::create_category(&tx, &CategoryFields::new("Food")).unwrap();
    let child = CategoryFields {
        parent_id: Some(food.clone()),
        ..CategoryFields::new("Dairy")
    };
    catalog::create_category(&tx, &child).unwrap();

    let parent: String = tx
        .query_row(
            "SELECT parent_id FROM catalog_v_categories WHERE name = 'Dairy'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(parent, food.to_string());
}

#[test]
fn price_info_gives_price_tax_category_path_and_brand() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let food = catalog::create_category(&tx, &CategoryFields::new("Food")).unwrap();
    let dairy_fields = CategoryFields {
        parent_id: Some(food.clone()),
        ..CategoryFields::new("Dairy")
    };
    let dairy = catalog::create_category(&tx, &dairy_fields).unwrap();
    let brand = catalog::create_brand(&tx, "Pran").unwrap();
    let milk = NewProduct {
        category_id: Some(dairy.clone()),
        brand_id: Some(brand.clone()),
        ..standard("Milk", 9000)
    };
    let milk_id = catalog::create_product(&tx, milk, NOW).unwrap();
    let bread_id = catalog::create_product(&tx, standard("Bread", 5000), NOW).unwrap();

    let info = catalog::price_info(&tx, &[milk_id.clone(), bread_id.clone()]).unwrap();
    let milk_info = &info[&milk_id];
    assert_eq!(milk_info.sale_price, Money(9000));
    assert_eq!(milk_info.tax_rate_id, vat());
    assert_eq!(milk_info.category_path, vec![dairy, food]);
    assert_eq!(milk_info.brand_id, Some(brand));
    assert_eq!(info[&bread_id].category_path, vec![id(UNCATEGORISED)]);
    assert_eq!(info[&bread_id].brand_id, None);
}

#[test]
fn list_filters_by_text_category_and_kind_and_pages() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let dairy = catalog::create_category(&tx, &CategoryFields::new("Dairy")).unwrap();
    for name in ["Milk", "Cheese", "Butter"] {
        let product = NewProduct {
            category_id: Some(dairy.clone()),
            ..standard(name, 100)
        };
        catalog::create_product(&tx, product, NOW).unwrap();
    }
    catalog::create_product(&tx, standard("Soap", 100), NOW).unwrap();
    let kit = NewProduct::new(ProductKind::Kit, "Gift", id(catalog::UNIT_PIECE), vat());
    catalog::create_product(&tx, kit, NOW).unwrap();
    tx.commit().unwrap();

    let all = ProductPage {
        offset: 0,
        limit: 2,
    };
    let in_dairy = ProductFilter {
        category_id: Some(dairy),
        ..ProductFilter::default()
    };
    let first_page = catalog::list(&db, &in_dairy, all).unwrap();
    assert_eq!(first_page.total, 3);
    let names: Vec<&str> = first_page
        .rows
        .iter()
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(names, vec!["Butter", "Cheese"]);
    assert_eq!(first_page.rows[0].category_name, "Dairy");

    let second_page = catalog::list(
        &db,
        &in_dairy,
        ProductPage {
            offset: 2,
            limit: 2,
        },
    )
    .unwrap();
    assert_eq!(second_page.rows[0].name, "Milk");

    let text = ProductFilter {
        text: Some("soa".to_string()),
        ..ProductFilter::default()
    };
    assert_eq!(catalog::list(&db, &text, all).unwrap().rows[0].name, "Soap");

    let kits = ProductFilter {
        kind: Some(ProductKind::Kit),
        ..ProductFilter::default()
    };
    assert_eq!(catalog::list(&db, &kits, all).unwrap().rows[0].name, "Gift");
}

#[test]
fn list_filters_by_parent_to_show_a_parents_variants() {
    let mut db = test_db();
    let tx = db.transaction().unwrap();
    let parent = NewProduct::new(
        ProductKind::VariantParent,
        "Shampoo",
        id(catalog::UNIT_PIECE),
        vat(),
    );
    let parent = catalog::create_product(&tx, parent, NOW).unwrap();
    let sizes = vec![(
        "Size".to_string(),
        vec!["200 ml".to_string(), "400 ml".to_string()],
    )];
    catalog::generate_variants(&tx, &parent, &sizes, NOW).unwrap();
    catalog::create_product(&tx, standard("Soap", 100), NOW).unwrap();
    tx.commit().unwrap();

    let variants = ProductFilter {
        parent_id: Some(parent),
        ..ProductFilter::default()
    };
    let page = ProductPage {
        offset: 0,
        limit: 10,
    };
    let found = catalog::list(&db, &variants, page).unwrap();
    let names: Vec<&str> = found.rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, vec!["Shampoo 200 ml", "Shampoo 400 ml"]);
}

#[test]
fn get_returns_every_field_and_unknown_ids_are_not_found() {
    let mut db = test_db();
    let milk = create_with_barcode(&mut db, "Milk", "111", 9000);
    let product = catalog::get(&db, &milk).unwrap();
    assert_eq!(product.kind, ProductKind::Standard);
    assert_eq!(product.barcodes, vec!["111"]);
    assert_eq!(product.created_at, NOW);
    assert_eq!(product.tax_rate_id, vat());
    assert!(product.track_stock);

    let error = catalog::get(&db, &pos_core::Id::new()).unwrap_err();
    assert!(matches!(AppError::from(error), AppError::NotFound { .. }));
}

#[test]
fn search_hit_and_sale_product_carry_what_checkout_needs() {
    let mut db = test_db();
    let milk = create_with_barcode(&mut db, "Milk 1L", "8901234567890", 9000);
    let hit = &catalog::search(&db, "milk", 5).unwrap()[0];
    assert_eq!(hit.product_id, milk);
    assert_eq!(hit.sku, "P000001");
    assert_eq!(hit.sale_price, Money(9000));
    assert_eq!(hit.unit_symbol, "pc");
    assert_eq!(hit.thumbnail, None);

    let sale = catalog::find_by_code(&db, "8901234567890")
        .unwrap()
        .unwrap();
    assert_eq!(sale.unit_decimals, 0);
    assert_eq!(sale.tax_rate_id, vat());
    assert!(sale.track_stock);
    assert!(!sale.is_weighed);
}

#[test]
fn errors_turn_into_app_errors_with_stable_codes() {
    let mut db = test_db();
    create(&mut db, standard("Milk", 100));
    let tx = db.transaction().unwrap();
    let error = catalog::create_product(&tx, standard("", 100), NOW).unwrap_err();
    let AppError::Validation { field, code, .. } = AppError::from(error) else {
        panic!("expected a validation error");
    };
    assert_eq!(field.as_deref(), Some("name"));
    assert_eq!(code, "name_missing");
}
