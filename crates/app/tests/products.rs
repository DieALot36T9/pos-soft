//! The product screens' workflows (Req - Catalog): saving a product with its reorder
//! rule, price history and audit in one transaction, barcodes, variants, kits, images,
//! archive and delete, and units, categories and brands.

mod common;

use app::commands::catalog as commands;
use app::session;
use app::state::AppState;
use app::workflows::catalog_lists::{BrandInput, CategoryInput, UnitInput};
use app::workflows::products::{KitPartInput, ProductInput, ReorderInput, VariantAttribute};
use app::workflows::users::{RoleInput, UserInput};
use catalog::{CategoryFields, PriceField, ProductFilter, ProductKind, ProductPage, UnitFields};
use common::*;
use pos_core::{AppError, Bps, Id, Money, Permission, Qty, Secret};

fn shop() -> (tempfile::TempDir, AppState) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    (folder, state)
}

fn piece() -> Id {
    Id::parse(catalog::UNIT_PIECE).expect("id")
}

/// A product form filled in like a shop owner would, with the shop's default tax rate.
fn form(name: &str, sale_minor: i64) -> ProductInput {
    ProductInput {
        id: None,
        kind: ProductKind::Standard,
        name: name.into(),
        sku: String::new(),
        category_id: None,
        brand_id: None,
        unit_id: piece(),
        sale_price: Money(sale_minor),
        cost_price: Money(0),
        tax_rate_id: None,
        is_weighed: false,
        scale_item_code: None,
        track_stock: true,
        track_expiry: false,
        preferred_supplier_id: None,
        description: None,
        barcodes: Vec::new(),
        reorder: None,
    }
}

fn edit_form(state: &AppState, id: &Id) -> ProductInput {
    let product = commands::product_get(state, id).expect("details").product;
    ProductInput {
        id: Some(id.clone()),
        kind: product.kind,
        name: product.name,
        sku: product.sku,
        category_id: Some(product.category_id),
        brand_id: product.brand_id,
        unit_id: product.unit_id,
        sale_price: product.sale_price,
        cost_price: product.cost_price,
        tax_rate_id: Some(product.tax_rate_id),
        is_weighed: product.is_weighed,
        scale_item_code: product.scale_item_code,
        track_stock: product.track_stock,
        track_expiry: product.track_expiry,
        preferred_supplier_id: product.preferred_supplier_id,
        description: product.description,
        barcodes: product.barcodes,
        reorder: None,
    }
}

fn product_count(state: &AppState) -> u64 {
    let page = ProductPage {
        offset: 0,
        limit: 50,
    };
    let everything = ProductFilter {
        include_archived: true,
        ..ProductFilter::default()
    };
    commands::product_list(state, &everything, page)
        .expect("list")
        .total
}

fn validation_code(result: Result<impl std::fmt::Debug, AppError>) -> String {
    match result {
        Err(AppError::Validation { code, .. }) => code,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

fn conflict_code(result: Result<impl std::fmt::Debug, AppError>) -> String {
    match result {
        Err(AppError::Conflict { code, .. }) => code,
        other => panic!("expected a conflict, got {other:?}"),
    }
}

#[test]
fn cat_01_save_product_with_reorder_rule_and_audit_in_one_go() {
    let (_folder, state) = shop();
    let input = ProductInput {
        barcodes: vec!["8901234567890".into()],
        reorder: Some(ReorderInput {
            level: Qty(10_000),
            qty: Qty(24_000),
        }),
        ..form("Milk 1L", 9000)
    };
    let id = commands::product_save(&state, input).expect("saved");

    let details = commands::product_get(&state, &id).expect("details");
    assert_eq!(details.product.sku, "P000001");
    assert_eq!(details.product.barcodes, vec!["8901234567890"]);
    assert_eq!(
        details.reorder,
        Some(ReorderInput {
            level: Qty(10_000),
            qty: Qty(24_000)
        })
    );
    // The shop's default rate was used.
    let default_rate = commands::product_choices(&state)
        .expect("choices")
        .default_tax_rate_id;
    assert_eq!(Some(details.product.tax_rate_id), default_rate);
    let (user, _, audit_details) = last_audit(&state, "product.create");
    assert!(user.is_some());
    assert!(audit_details.contains("Milk 1L"), "{audit_details}");
}

#[test]
fn cat_01_a_refused_step_saves_nothing() {
    let (_folder, state) = shop();
    // The catalog accepts the product, then inventory refuses the reorder rule:
    // the whole save is rolled back.
    let input = ProductInput {
        reorder: Some(ReorderInput {
            level: Qty(-1),
            qty: Qty(0),
        }),
        ..form("Bread", 5000)
    };
    let code = validation_code(commands::product_save(&state, input));
    assert_eq!(code, "reorder_below_zero");
    assert_eq!(product_count(&state), 0);
    assert_eq!(audit_count(&state, "product.create"), 0);

    // An unknown tax rate is refused before anything is written.
    let unknown_rate = ProductInput {
        tax_rate_id: Some(Id::new()),
        ..form("Bread", 5000)
    };
    let refused = commands::product_save(&state, unknown_rate);
    assert!(
        matches!(refused, Err(AppError::NotFound { .. })),
        "{refused:?}"
    );
    assert_eq!(product_count(&state), 0);
}

#[test]
fn cat_09_price_change_writes_history_with_the_users_name() {
    let (_folder, state) = shop();
    let id = commands::product_save(&state, form("Milk 1L", 9000)).expect("saved");
    let mut edit = edit_form(&state, &id);
    edit.sale_price = Money(9500);
    commands::product_save(&state, edit).expect("changed");

    let history = commands::product_get(&state, &id)
        .expect("details")
        .price_history;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].field, PriceField::SalePrice);
    assert_eq!((history[0].old, history[0].new), (Money(9000), Money(9500)));
    assert_eq!(history[0].changed_by_name.as_deref(), Some("Shop Owner"));
    assert_eq!(audit_count(&state, "product.change"), 1);
}

#[test]
fn cat_09_changing_a_price_needs_catalog_price_edit() {
    let (_folder, state) = shop();
    let id = commands::product_save(&state, form("Milk 1L", 9000)).expect("saved");
    // A role that may manage products but not prices.
    let role = app::commands::users::role_save(
        &state,
        RoleInput {
            id: None,
            name: "Stock keeper".into(),
            permissions: vec![Permission::StockView, Permission::CatalogManage],
            max_discount: Bps(0),
        },
    )
    .expect("role");
    app::commands::users::user_save(
        &state,
        UserInput {
            id: None,
            username: "karim".into(),
            display_name: "Karim".into(),
            role_id: role,
            active: true,
            pin: Some(Secret::new("2468".into())),
            password: None,
        },
    )
    .expect("user");
    session::logout(&state).expect("out");
    sign_in(&state, "karim", "2468");

    let mut rename = edit_form(&state, &id);
    rename.name = "Milk 1 litre".into();
    commands::product_save(&state, rename).expect("a name change is fine");

    let mut reprice = edit_form(&state, &id);
    reprice.sale_price = Money(1);
    let refused = commands::product_save(&state, reprice);
    assert!(
        matches!(
            refused,
            Err(AppError::PermissionDenied {
                permission: Permission::CatalogPriceEdit
            })
        ),
        "{refused:?}"
    );
    let new_with_price = commands::product_save(&state, form("Eggs", 1500));
    assert!(matches!(
        new_with_price,
        Err(AppError::PermissionDenied { .. })
    ));
    commands::product_save(&state, form("Free bag", 0)).expect("no price, no price permission");
}

#[test]
fn cat_04_barcode_check_digit_and_duplicates_are_refused() {
    let (_folder, state) = shop();
    let wrong_digit = ProductInput {
        barcodes: vec!["8901234567891".into()],
        ..form("Milk 1L", 9000)
    };
    assert_eq!(
        validation_code(commands::product_save(&state, wrong_digit)),
        "barcode_check_digit_wrong"
    );
    // Codes that don't look like EAN or UPC are kept as typed (Code 128).
    let code128 = ProductInput {
        barcodes: vec!["ABC-123".into()],
        ..form("Rice 5kg", 60000)
    };
    commands::product_save(&state, code128).expect("Code 128 kept");

    let milk = ProductInput {
        barcodes: vec!["8901234567890".into()],
        ..form("Milk 1L", 9000)
    };
    commands::product_save(&state, milk).expect("milk");
    let copy = ProductInput {
        barcodes: vec!["8901234567890".into()],
        ..form("Milk copy", 9000)
    };
    let refused = commands::product_save(&state, copy);
    match refused {
        Err(AppError::Conflict { code, params }) => {
            assert_eq!(code, "barcode_in_use");
            assert_eq!(
                params.get("product_name").map(String::as_str),
                Some("Milk 1L")
            );
            assert_eq!(params.get("sku").map(String::as_str), Some("P000002"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn cat_04_editing_barcodes_adds_and_removes_them() {
    let (_folder, state) = shop();
    let input = ProductInput {
        barcodes: vec!["111".into(), "222".into()],
        ..form("Soap", 3000)
    };
    let id = commands::product_save(&state, input).expect("saved");
    let mut edit = edit_form(&state, &id);
    edit.barcodes = vec!["222".into(), "333".into()];
    commands::product_save(&state, edit).expect("changed");
    let barcodes = commands::product_get(&state, &id)
        .expect("details")
        .product
        .barcodes;
    assert_eq!(barcodes, vec!["222", "333"]);
}

#[test]
fn cat_11_generated_barcode_uses_the_shop_prefix_and_a_check_digit() {
    let (_folder, state) = shop();
    let id = commands::product_save(&state, form("Cake", 25000)).expect("saved");
    let code = commands::barcode_generate(&state, &id).expect("generated");
    assert_eq!(code, "2000000000015");
    assert!(barcode::is_valid_gtin(&code));
    let second = commands::product_save(&state, form("Pie", 25000)).expect("saved");
    // Number 2 was typed by hand on another product, so it is skipped.
    let typed = ProductInput {
        barcodes: vec!["2000000000022".into()],
        ..form("Hand typed", 100)
    };
    commands::product_save(&state, typed).expect("typed");
    assert_eq!(
        commands::barcode_generate(&state, &second).expect("generated"),
        "2000000000039"
    );
    let found = commands::product_search(&state, "2000000000015", 5).expect("search");
    assert!(found[0].exact_match);
    assert_eq!(found[0].name, "Cake");
}

#[test]
fn cat_06_variants_are_generated_and_listed_on_the_parent() {
    let (_folder, state) = shop();
    let parent = ProductInput {
        kind: ProductKind::VariantParent,
        ..form("Shampoo", 30000)
    };
    let parent = commands::product_save(&state, parent).expect("parent");
    let attributes = vec![
        VariantAttribute {
            name: "Size".into(),
            values: vec!["200 ml".into(), "400 ml".into()],
        },
        VariantAttribute {
            name: "Flavour".into(),
            values: vec!["Mango".into(), " ".into(), "Lemon".into()],
        },
    ];
    let made = commands::variants_generate(&state, &parent, &attributes).expect("made");
    assert_eq!(made.len(), 4);
    let again = commands::variants_generate(&state, &parent, &attributes).expect("again");
    assert!(again.is_empty());

    let details = commands::product_get(&state, &parent).expect("details");
    assert_eq!(details.variants.len(), 4);
    assert!(!details.product.track_stock);
    let parent_barcode = commands::barcode_generate(&state, &parent);
    assert!(parent_barcode.is_err(), "a parent can't have barcodes");
}

#[test]
fn cat_07_kit_parts_are_saved_and_cycles_refused() {
    let (_folder, state) = shop();
    let soap = commands::product_save(&state, form("Soap", 3000)).expect("soap");
    let towel = commands::product_save(&state, form("Towel", 20000)).expect("towel");
    let kit = ProductInput {
        kind: ProductKind::Kit,
        ..form("Bath gift set", 25000)
    };
    let kit = commands::product_save(&state, kit).expect("kit");
    let parts = vec![
        KitPartInput {
            product_id: soap.clone(),
            qty: Qty(2000),
        },
        KitPartInput {
            product_id: towel,
            qty: Qty(1000),
        },
    ];
    commands::kit_save(&state, &kit, &parts).expect("parts");
    let details = commands::product_get(&state, &kit).expect("details");
    let names: Vec<&str> = details
        .kit_parts
        .iter()
        .map(|part| part.name.as_str())
        .collect();
    assert!(names.contains(&"Soap") && names.contains(&"Towel"));
    assert!(!details.product.track_stock);

    let itself = vec![KitPartInput {
        product_id: kit.clone(),
        qty: Qty(1000),
    }];
    assert_eq!(
        validation_code(commands::kit_save(&state, &kit, &itself)),
        "kit_contains_itself"
    );
    // A product inside a kit can't be deleted.
    assert_eq!(
        conflict_code(commands::product_delete(&state, &soap)),
        "product_in_use"
    );
}

#[test]
fn cat_08_archive_bring_back_and_delete_rules() {
    let (_folder, state) = shop();
    let milk = commands::product_save(&state, form("Milk", 9000)).expect("milk");
    commands::product_archive(&state, &milk).expect("archived");
    assert!(
        commands::product_search(&state, "milk", 5)
            .expect("search")
            .is_empty()
    );
    commands::product_unarchive(&state, &milk).expect("back");
    assert_eq!(
        commands::product_search(&state, "milk", 5)
            .expect("search")
            .len(),
        1
    );
    assert_eq!(audit_count(&state, "product.archive"), 1);
    assert_eq!(audit_count(&state, "product.unarchive"), 1);

    // Once it moved in stock, it can only be archived.
    state
        .db
        .write(|tx| {
            let context = inventory::MoveCtx {
                at: state.clock.now(),
                business_date: pos_core::BusinessDate::parse("2026-10-05")?,
                till_id: Id::new(),
                user_id: None,
            };
            let movement =
                inventory::Movement::new(&milk, Qty(1000), inventory::MoveKind::Receive, context);
            inventory::record(tx, movement, true)?;
            Ok(())
        })
        .expect("stock moved");
    assert_eq!(
        conflict_code(commands::product_delete(&state, &milk)),
        "product_in_use"
    );

    let bread = ProductInput {
        reorder: Some(ReorderInput {
            level: Qty(1000),
            qty: Qty(5000),
        }),
        ..form("Bread", 5000)
    };
    let bread = commands::product_save(&state, bread).expect("bread");
    commands::product_delete(&state, &bread).expect("deleted");
    assert!(matches!(
        commands::product_get(&state, &bread),
        Err(AppError::NotFound { .. })
    ));
    let rule = state
        .db
        .read(|conn| Ok(inventory::reorder_rule(conn, &bread)?))
        .expect("read");
    assert_eq!(rule, None, "the reorder rule goes with the product");
    assert_eq!(audit_count(&state, "product.delete"), 1);
}

#[test]
fn cat_10_image_is_resized_and_shown_as_a_data_address() {
    let (folder, state) = shop();
    let id = commands::product_save(&state, form("Mango", 12000)).expect("saved");
    let picture = folder.path().join("mango.png");
    image::RgbImage::from_pixel(1024, 768, image::Rgb([250, 180, 20]))
        .save(&picture)
        .expect("picture");
    commands::product_image_set(&state, &id, &picture.to_string_lossy()).expect("set");

    let details = commands::product_get(&state, &id).expect("details");
    let address = details.image.expect("an image");
    assert!(address.starts_with("data:image/jpeg;base64,"));
    assert!(
        state
            .paths
            .images()
            .join(format!("{id}.thumb.jpg"))
            .exists()
    );

    let text_file = folder.path().join("notes.txt");
    std::fs::write(&text_file, "not a picture").expect("file");
    let refused = commands::product_image_set(&state, &id, &text_file.to_string_lossy());
    assert_eq!(validation_code(refused), "image_not_supported");
}

#[test]
fn cat_02_cat_03_units_categories_and_brands() {
    let (_folder, state) = shop();
    let dozen = commands::unit_save(
        &state,
        UnitInput {
            id: None,
            fields: UnitFields {
                name: "Dozen".into(),
                symbol: "dz".into(),
                unit_code: None,
                decimals: 0,
            },
        },
    )
    .expect("unit");
    let dairy = commands::category_save(
        &state,
        CategoryInput {
            id: None,
            fields: CategoryFields::new("Dairy"),
        },
    )
    .expect("category");
    let milk_category = commands::category_save(
        &state,
        CategoryInput {
            id: None,
            fields: CategoryFields {
                parent_id: Some(dairy.clone()),
                ..CategoryFields::new("Milk")
            },
        },
    )
    .expect("child category");
    let brand = commands::brand_save(
        &state,
        BrandInput {
            id: None,
            name: "Aarong".into(),
        },
    )
    .expect("brand");
    let eggs = ProductInput {
        unit_id: dozen.clone(),
        category_id: Some(milk_category.clone()),
        brand_id: Some(brand.clone()),
        ..form("Eggs", 15000)
    };
    commands::product_save(&state, eggs).expect("eggs");

    assert_eq!(
        conflict_code(commands::unit_archive(&state, &dozen)),
        "unit_in_use"
    );
    assert_eq!(
        conflict_code(commands::category_archive(&state, &dairy)),
        "category_has_children"
    );
    assert_eq!(
        conflict_code(commands::category_archive(&state, &milk_category)),
        "category_has_products"
    );
    // A parent category's filter finds products in its children.
    let in_dairy = ProductFilter {
        category_id: Some(dairy),
        ..ProductFilter::default()
    };
    let page = ProductPage {
        offset: 0,
        limit: 10,
    };
    assert_eq!(
        commands::product_list(&state, &in_dairy, page)
            .expect("list")
            .total,
        1
    );
    commands::brand_archive(&state, &brand).expect("brand archived");
    assert!(
        commands::brand_list(&state, false)
            .expect("brands")
            .is_empty()
    );
    assert_eq!(commands::brand_list(&state, true).expect("brands").len(), 1);
    assert_eq!(audit_count(&state, "unit.change"), 1);
    assert_eq!(audit_count(&state, "category.change"), 2);
    assert_eq!(audit_count(&state, "brand.change"), 2);
    assert!(
        commands::unit_list(&state, false)
            .expect("units")
            .iter()
            .any(|unit| unit.name == "Dozen")
    );
    assert_eq!(
        commands::category_list(&state, false)
            .expect("categories")
            .len(),
        3
    );
}

#[test]
fn chk_01_a_cashier_can_look_products_up_but_not_change_them() {
    let (_folder, state) = shop();
    commands::product_save(&state, form("Milk", 9000)).expect("milk");
    add_cashier(&state);
    session::logout(&state).expect("out");
    sign_in(&state, "rina", "1234");
    assert_eq!(
        commands::product_search(&state, "mil", 5)
            .expect("search")
            .len(),
        1
    );
    let refused = commands::product_save(&state, form("Bread", 0));
    assert!(matches!(
        refused,
        Err(AppError::PermissionDenied {
            permission: Permission::CatalogManage
        })
    ));
}
