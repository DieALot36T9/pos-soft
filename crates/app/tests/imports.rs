//! The import wizard and exports (Req - Import and Export): preview with every error,
//! the backup first, all or nothing, skipping bad rows, every entity, exports that
//! import back unchanged, cancel, and 50,000 products in under a minute.

mod common;

use std::path::Path;
use std::time::{Duration, Instant};

use app::commands::io as commands;
use app::state::AppState;
use app::workflows::exports::{ExportEntity, ExportRequest, FileFormat};
use app::workflows::import_file::{ImportEntity, ImportRequest, MatchBy, RowAction};
use app::workflows::pricing_admin::PriceListSave;
use app::workflows::purchasing_admin::SupplierSave;
use catalog::{PriceField, ProductFilter, ProductPage};
use common::*;
use pos_core::{AppError, Id, Money, Qty};

fn shop() -> (tempfile::TempDir, AppState) {
    let folder = tempfile::tempdir().expect("folder");
    let state = start(folder.path());
    finish_setup(&state);
    (folder, state)
}

/// Writes a CSV file with these lines and gives its path.
fn csv(folder: &Path, name: &str, lines: &[&str]) -> String {
    let path = folder.join(name);
    std::fs::write(&path, lines.join("\r\n")).expect("file written");
    path.to_string_lossy().into_owned()
}

fn request(entity: ImportEntity, file: &str) -> ImportRequest {
    ImportRequest {
        entity,
        file: file.into(),
        sheet: None,
        mapping: None,
        match_by: MatchBy::Sku,
        create_missing: false,
        skip_errors: false,
        price_list_id: None,
    }
}

fn product(state: &AppState, sku: &str) -> catalog::Product {
    state
        .db
        .read(|conn| {
            let id = catalog::find_by_code(conn, sku)?.expect("found").id;
            Ok(catalog::get(conn, &id)?)
        })
        .expect("read")
}

fn product_count(state: &AppState) -> u64 {
    let page = ProductPage {
        offset: 0,
        limit: 1,
    };
    state
        .db
        .read(|conn| Ok(catalog::list(conn, &ProductFilter::default(), page)?.total))
        .expect("count")
}

#[test]
fn io_01_columns_match_by_their_common_names_and_the_preview_lists_every_error() {
    let (folder, state) = shop();
    let file = csv(
        folder.path(),
        "products.csv",
        &[
            "Product name,Item code,MRP,Cost,Barcode",
            "Milk 1L,P100,90.00,75.00,8901234567890",
            "Rice 5kg,P101,12,5O,400.00,",
            "Sugar 1kg,P102,140.505,120,",
            ",P103,10,5,",
        ],
    );
    let preview =
        commands::import_preview(&state, &request(ImportEntity::Products, &file)).expect("preview");

    let mapped = |key: &str| {
        let index = preview
            .fields
            .iter()
            .position(|field| field.key == key)
            .expect("field");
        preview.mapping[index]
    };
    assert_eq!(mapped("name"), Some(0));
    assert_eq!(mapped("sku"), Some(1));
    assert_eq!(mapped("sale_price"), Some(2));
    assert_eq!(mapped("cost_price"), Some(3));
    assert_eq!(mapped("barcodes"), Some(4));
    assert_eq!(preview.total_rows, 4);

    // Row 3 has "12,5O" split by the comma: "12" and "5O" (the cost column).
    let errors: Vec<(u32, &str, &str)> = preview
        .errors
        .iter()
        .map(|error| (error.row, error.column.as_str(), error.code.as_str()))
        .collect();
    assert!(
        errors.contains(&(3, "Cost", "money_not_a_number")),
        "{errors:?}"
    );
    assert!(
        errors.contains(&(4, "MRP", "money_too_many_decimals")),
        "{errors:?}"
    );
    assert!(
        errors.contains(&(5, "Product name", "cell_empty")),
        "{errors:?}"
    );
    assert_eq!(preview.counts.created, 1);
    assert_eq!(preview.counts.skipped, 3);
    assert_eq!(preview.rows[0].action, RowAction::Create);
    assert_eq!(preview.rows[0].cells[6], "90.00");

    // The preview saved nothing.
    assert_eq!(product_count(&state), 0);
}

#[test]
fn io_01_import_with_errors_is_refused_and_saves_nothing() {
    let (folder, state) = shop();
    let file = csv(
        folder.path(),
        "products.csv",
        &["Name,SKU,Sale price", "Milk 1L,P100,90", "Rice,P101,abc"],
    );
    let refused = commands::import_commit(&state, &request(ImportEntity::Products, &file))
        .expect_err("refused");
    assert!(
        matches!(&refused, AppError::Validation { code, .. } if code == "import_has_errors"),
        "{refused:?}"
    );
    assert_eq!(product_count(&state), 0);
}

#[test]
fn io_01_skip_rows_with_errors_imports_the_rest_after_a_backup_and_audits_the_counts() {
    let (folder, state) = shop();
    let file = csv(
        folder.path(),
        "products.csv",
        &["Name,SKU,Sale price", "Milk 1L,P100,90", "Rice,P101,abc"],
    );
    let mut skip = request(ImportEntity::Products, &file);
    skip.skip_errors = true;
    let done = commands::import_commit(&state, &skip).expect("imported");

    assert_eq!((done.counts.created, done.counts.skipped), (1, 1));
    assert_eq!(done.errors.len(), 1);
    assert_eq!(done.errors[0].row, 3);
    assert!(
        Path::new(&done.backup_file).exists(),
        "{}",
        done.backup_file
    );
    assert!(done.backup_file.contains("pre-import"));
    assert_eq!(product(&state, "P100").sale_price, Money(9000));
    let (_, _, details) = last_audit(&state, "data.import");
    assert!(details.contains("\"created\":1"), "{details}");
    assert!(details.contains("\"entity\":\"products\""), "{details}");

    // The error file has the row, column, value and problem.
    let errors_file = folder.path().join("errors.csv");
    let lines = vec![app::workflows::import_file::ErrorLine {
        row: 3,
        column: "Sale price".into(),
        value: "abc".into(),
        problem: "Not a number".into(),
    }];
    commands::import_errors_save(&state, &lines, &errors_file.to_string_lossy()).expect("saved");
    let table = io::read_file(&errors_file, None).expect("read back");
    assert_eq!(
        table.rows[0],
        vec!["3", "Sale price", "abc", "Not a number"]
    );
}

#[test]
fn io_01_existing_products_are_updated_by_sku_with_import_price_history() {
    let (folder, state) = shop();
    let first = csv(
        folder.path(),
        "first.csv",
        &["Name,SKU,Sale price,Cost price", "Milk 1L,P100,90,70"],
    );
    commands::import_commit(&state, &request(ImportEntity::Products, &first)).expect("first");
    let second = csv(
        folder.path(),
        "second.csv",
        &["Name,SKU,Sale price", "Milk 1 litre,P100,95"],
    );
    let done =
        commands::import_commit(&state, &request(ImportEntity::Products, &second)).expect("second");
    assert_eq!((done.counts.created, done.counts.updated), (0, 1));

    let milk = product(&state, "P100");
    assert_eq!(milk.name, "Milk 1 litre");
    assert_eq!(milk.sale_price, Money(9500));
    // A blank or missing column keeps what was saved.
    assert_eq!(milk.cost_price, Money(7000));
    let history = state
        .db
        .read(|conn| Ok(catalog::price_history(conn, &milk.id)?))
        .expect("history");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].field, PriceField::SalePrice);
    assert_eq!(history[0].reason.as_deref(), Some("import"));
}

#[test]
fn io_01_match_by_barcode_updates_the_product_with_that_barcode() {
    let (folder, state) = shop();
    till_product(
        &state,
        "Milk",
        "8901234567890",
        9000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    let file = csv(
        folder.path(),
        "by-barcode.csv",
        &["Name,Barcode,Sale price", "Milk 1L,8901234567890,99"],
    );
    let mut by_barcode = request(ImportEntity::Products, &file);
    by_barcode.match_by = MatchBy::Barcode;
    let done = commands::import_commit(&state, &by_barcode).expect("imported");
    assert_eq!((done.counts.created, done.counts.updated), (0, 1));
    assert_eq!(product(&state, "8901234567890").sale_price, Money(9900));
}

#[test]
fn io_01_missing_categories_brands_and_units_are_made_only_when_asked() {
    let (folder, state) = shop();
    let file = csv(
        folder.path(),
        "new-names.csv",
        &[
            "Name,SKU,Sale price,Category,Brand,Unit",
            "Eggs,P200,12,Dairy,Farm Fresh,dozen",
            "Butter,P201,300,dairy,Farm Fresh,pc",
        ],
    );
    let preview =
        commands::import_preview(&state, &request(ImportEntity::Products, &file)).expect("preview");
    let codes: Vec<&str> = preview
        .errors
        .iter()
        .map(|error| error.code.as_str())
        .collect();
    assert!(codes.contains(&"category_unknown"), "{codes:?}");
    assert!(codes.contains(&"brand_unknown"), "{codes:?}");
    assert!(codes.contains(&"unit_unknown"), "{codes:?}");

    let mut make = request(ImportEntity::Products, &file);
    make.create_missing = true;
    let done = commands::import_commit(&state, &make).expect("imported");
    assert_eq!(done.counts.created, 2);
    let eggs = product(&state, "P200");
    let butter = product(&state, "P201");
    // "dairy" is the same category as "Dairy": made once.
    assert_eq!(eggs.category_id, butter.category_id);
    assert_eq!(eggs.brand_id, butter.brand_id);
    assert_eq!(butter.unit_id, Id::parse(catalog::UNIT_PIECE).expect("id"));
}

#[test]
fn io_01_a_required_field_without_a_column_is_shown_and_refused() {
    let (folder, state) = shop();
    let file = csv(folder.path(), "no-price.csv", &["Name,SKU", "Milk,P1"]);
    let preview =
        commands::import_preview(&state, &request(ImportEntity::Products, &file)).expect("preview");
    assert_eq!(preview.errors.len(), 1);
    assert_eq!(preview.errors[0].code, "column_not_mapped");
    assert_eq!(preview.errors[0].column, "Sale price");

    // The user maps the columns by hand instead: SKU, then the name.
    let mut chosen = request(ImportEntity::Products, &file);
    let mut mapping = vec![None; preview.fields.len()];
    mapping[0] = Some(1);
    mapping[1] = Some(0);
    chosen.mapping = Some(mapping);
    let refused = commands::import_commit(&state, &chosen).expect_err("price missing");
    assert!(
        matches!(&refused, AppError::Validation { code, .. } if code == "import_field_not_mapped"),
        "{refused:?}"
    );
}

#[test]
fn io_02_customers_and_suppliers_are_added_and_updated() {
    let (folder, state) = shop();
    let customers = csv(
        folder.path(),
        "customers.csv",
        &[
            "Mobile,Customer name,Credit limit",
            "01711000001,Asha,500",
            "+8801711000001,Asha Begum,",
            "01711000002,Karim,",
        ],
    );
    let done =
        commands::import_commit(&state, &request(ImportEntity::Customers, &customers)).expect("ok");
    // The second row is the same phone number written another way.
    assert_eq!((done.counts.created, done.counts.updated), (2, 1));

    let suppliers = csv(
        folder.path(),
        "suppliers.csv",
        &[
            "Supplier,Phone,Payment terms",
            "Rahim Traders,01811000000,30",
            "Bengal Foods,,x",
        ],
    );
    let preview = commands::import_preview(&state, &request(ImportEntity::Suppliers, &suppliers))
        .expect("preview");
    assert_eq!(preview.errors.len(), 1);
    assert_eq!(preview.errors[0].code, "number_not_whole");
    let mut skip = request(ImportEntity::Suppliers, &suppliers);
    skip.skip_errors = true;
    let done = commands::import_commit(&state, &skip).expect("ok");
    assert_eq!(done.counts.created, 1);
}

#[test]
fn io_02_opening_stock_goes_in_as_opening_movements_with_batches() {
    let (folder, state) = shop();
    let milk = till_product(
        &state,
        "Milk",
        "MILK-1",
        9000,
        catalog::UNIT_PIECE,
        false,
        None,
    );
    let file = csv(
        folder.path(),
        "stock.csv",
        &[
            "SKU,Quantity,Unit cost,Batch,Expiry date",
            &format!("{},24,70.00,B1,2027-03-31", product_sku(&state, &milk)),
            &format!("{},6,,,", product_sku(&state, &milk)),
            &format!("{},1.5,,,", product_sku(&state, &milk)),
            "NOPE,1,,,",
        ],
    );
    let preview = commands::import_preview(&state, &request(ImportEntity::OpeningStock, &file))
        .expect("preview");
    let codes: Vec<(u32, &str)> = preview
        .errors
        .iter()
        .map(|error| (error.row, error.code.as_str()))
        .collect();
    assert_eq!(
        codes,
        vec![(4, "qty_too_many_decimals"), (5, "product_unknown")]
    );

    let mut skip = request(ImportEntity::OpeningStock, &file);
    skip.skip_errors = true;
    commands::import_commit(&state, &skip).expect("imported");
    let on_hand = state
        .db
        .read(|conn| Ok(inventory::on_hand(conn, &milk)?))
        .expect("on hand");
    assert_eq!(on_hand, Qty(30_000));
    let batches = state
        .db
        .read(|conn| Ok(inventory::batches_of(conn, &milk)?))
        .expect("batches");
    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].batch_no.as_deref(), Some("B1"));
    assert_eq!(batches[0].unit_cost, Money(7000));
    assert!(
        state
            .db
            .read(|conn| Ok(inventory::verify(conn)?))
            .expect("verify")
            .is_empty()
    );
}

fn product_sku(state: &AppState, id: &Id) -> String {
    state
        .db
        .read(|conn| Ok(catalog::get(conn, id)?.sku))
        .expect("sku")
}

#[test]
fn io_02_price_list_prices_and_extra_barcodes() {
    let (folder, state) = shop();
    let milk = till_product(&state, "Milk", "", 9000, catalog::UNIT_PIECE, false, None);
    let sku = product_sku(&state, &milk);
    let list = app::commands::pricing::price_list_save(
        &state,
        &PriceListSave {
            id: None,
            name: "Wholesale".into(),
            active: true,
        },
    )
    .expect("list");
    let prices = csv(
        folder.path(),
        "prices.csv",
        &["SKU,Price", &format!("{sku},85")],
    );

    let no_list =
        commands::import_preview(&state, &request(ImportEntity::PriceListPrices, &prices))
            .expect_err("needs a list");
    assert!(
        matches!(&no_list, AppError::Validation { code, .. } if code == "price_list_needed"),
        "{no_list:?}"
    );
    let mut into_list = request(ImportEntity::PriceListPrices, &prices);
    into_list.price_list_id = Some(list.clone());
    let done = commands::import_commit(&state, &into_list).expect("prices");
    assert_eq!(done.counts.created, 1);
    let items = state
        .db
        .read(|conn| Ok(pricing::price_list_items(conn, &list)?))
        .expect("items");
    assert_eq!(items[0].price, Money(8500));

    let codes = csv(
        folder.path(),
        "barcodes.csv",
        &[
            "SKU,Barcode",
            &format!("{sku},8901234567890"),
            &format!("{sku},8901234567891"),
        ],
    );
    let preview = commands::import_preview(&state, &request(ImportEntity::Barcodes, &codes))
        .expect("preview");
    assert_eq!(preview.errors[0].code, "barcode_check_digit_wrong");
    let mut skip = request(ImportEntity::Barcodes, &codes);
    skip.skip_errors = true;
    commands::import_commit(&state, &skip).expect("barcodes");
    assert_eq!(product(&state, "8901234567890").id, milk);
}

/// Exports, then imports the file back: nothing changes (IO-03).
fn round_trip(state: &AppState, folder: &Path, entity: ExportEntity, import: ImportEntity) {
    for (format, name) in [
        (FileFormat::Csv, "export.csv"),
        (FileFormat::Xlsx, "export.xlsx"),
    ] {
        let file = folder.join(name).to_string_lossy().into_owned();
        let _ = std::fs::remove_file(&file);
        let export = ExportRequest {
            entity,
            format,
            file: file.clone(),
            price_list_id: None,
        };
        let rows = commands::export_entity(state, &export).expect("exported");
        assert!(rows > 0, "{entity:?}");
        let preview = commands::import_preview(state, &request(import, &file)).expect("preview");
        assert!(
            preview.errors.is_empty(),
            "{entity:?} {format:?}: {:?}",
            preview.errors
        );
        assert_eq!(preview.counts.created, 0, "{entity:?} {format:?}");
        assert_eq!(
            preview.counts.updated, 0,
            "{entity:?} {format:?}: {:?}",
            preview.rows
        );
        assert_eq!(preview.counts.skipped, rows, "{entity:?} {format:?}");
    }
}

#[test]
fn io_03_exports_import_back_unchanged() {
    let (folder, state) = shop();
    let products = csv(
        folder.path(),
        "products.csv",
        &[
            "Name,SKU,Sale price,Cost price,Barcodes,Category,Brand,Unit,Weighed,Reorder level,Reorder quantity,Description",
            "Milk 1L,P100,90.00,75.00,8901234567890|MILK-1,Dairy,Farm Fresh,pc,no,10,24,\"Fresh, cold\"",
            "Potato,P101,60.50,40,,Vegetables,,kg,yes,5.5,20,=SUM(A1)",
            "Taka bag ৳,P102,0,0,,,,,,,,",
        ],
    );
    let mut make = request(ImportEntity::Products, &products);
    make.create_missing = true;
    commands::import_commit(&state, &make).expect("products");
    let customers = csv(
        folder.path(),
        "customers.csv",
        &[
            "Phone,Name,Email,Notes",
            "01711000001,Asha,asha@example.com,+1 call first",
        ],
    );
    commands::import_commit(&state, &request(ImportEntity::Customers, &customers)).expect("ok");
    app::commands::purchasing::supplier_save(
        &state,
        &SupplierSave {
            id: None,
            input: purchasing::SupplierInput {
                name: "Rahim Traders".into(),
                payment_terms_days: 30,
                ..purchasing::SupplierInput::default()
            },
        },
    )
    .expect("supplier");

    round_trip(
        &state,
        folder.path(),
        ExportEntity::Products,
        ImportEntity::Products,
    );
    round_trip(
        &state,
        folder.path(),
        ExportEntity::Customers,
        ImportEntity::Customers,
    );
    round_trip(
        &state,
        folder.path(),
        ExportEntity::Suppliers,
        ImportEntity::Suppliers,
    );
    let (_, _, details) = last_audit(&state, "data.export");
    assert!(details.contains("suppliers"), "{details}");

    // The CSV starts with a BOM and keeps the barcodes in one column.
    let file = folder.path().join("products-out.csv");
    let export = ExportRequest {
        entity: ExportEntity::Products,
        format: FileFormat::Csv,
        file: file.to_string_lossy().into_owned(),
        price_list_id: None,
    };
    commands::export_entity(&state, &export).expect("exported");
    let bytes = std::fs::read(&file).expect("file");
    assert!(bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("8901234567890|MILK-1"), "{text}");
    assert!(text.contains("5.500"), "{text}");
}

#[test]
fn io_03_stock_and_batch_exports() {
    let (folder, state) = shop();
    let milk = till_product(&state, "Milk", "", 9000, catalog::UNIT_PIECE, false, None);
    stock_in(&state, &milk, Qty(12_000));
    for (entity, name) in [
        (ExportEntity::StockLevels, "stock.xlsx"),
        (ExportEntity::Batches, "batches.csv"),
    ] {
        let file = folder.path().join(name);
        let format = if name.ends_with("xlsx") {
            FileFormat::Xlsx
        } else {
            FileFormat::Csv
        };
        let export = ExportRequest {
            entity,
            format,
            file: file.to_string_lossy().into_owned(),
            price_list_id: None,
        };
        commands::export_entity(&state, &export).expect("exported");
        let table = io::read_file(&file, None).expect("read back");
        if entity == ExportEntity::StockLevels {
            assert_eq!(table.headers[3], "On hand");
            assert_eq!(table.rows[0][3], "12");
            assert_eq!(table.rows[0][5], "648");
        } else {
            // Stock without a batch is not a batch.
            assert!(table.rows.is_empty(), "{table:?}");
        }
    }
}

#[test]
fn io_01_templates_have_the_headers_and_an_example_that_imports() {
    let (folder, state) = shop();
    for format in [FileFormat::Csv, FileFormat::Xlsx] {
        let name = if format == FileFormat::Csv {
            "t.csv"
        } else {
            "t.xlsx"
        };
        let file = folder.path().join(name).to_string_lossy().into_owned();
        commands::template_download(&state, ImportEntity::Products, format, &file).expect("saved");
        let mut make = request(ImportEntity::Products, &file);
        make.create_missing = true;
        let preview = commands::import_preview(&state, &make).expect("preview");
        assert!(preview.errors.is_empty(), "{:?}", preview.errors);
        assert_eq!(preview.headers[0], "SKU");
        assert_eq!(preview.counts.created, 1);
    }
}

/// A CSV of `count` products with prices and barcodes.
fn many_products(folder: &Path, count: usize) -> String {
    let mut text = String::from("SKU,Name,Sale price,Cost price,Barcodes,Category\r\n");
    for number in 0..count {
        text.push_str(&format!(
            "S{number:06},Product {number},{}.{:02},10.00,C{number:07},Group {}\r\n",
            10 + number % 500,
            number % 100,
            number % 20
        ));
    }
    let path = folder.join("many.csv");
    std::fs::write(&path, text).expect("written");
    path.to_string_lossy().into_owned()
}

#[test]
fn io_05_cancel_stops_the_import_and_saves_nothing() {
    let (folder, state) = shop();
    let file = many_products(folder.path(), 20_000);
    let mut make = request(ImportEntity::Products, &file);
    make.create_missing = true;
    let state = std::sync::Arc::new(state);
    let worker = {
        let state = std::sync::Arc::clone(&state);
        std::thread::spawn(move || commands::import_commit(&state, &make))
    };
    let started = Instant::now();
    while commands::import_progress(&state).expect("progress").done < 10 {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the import never started"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    commands::import_cancel(&state).expect("cancel");
    let result = worker.join().expect("the import thread ends");
    assert!(
        matches!(&result, Err(AppError::Conflict { code, .. }) if code == "import_cancelled"),
        "{result:?}"
    );
    assert_eq!(product_count(&state), 0);
    assert!(!commands::import_progress(&state).expect("progress").running);
}

#[test]
fn io_05_fifty_thousand_products_import_in_under_a_minute() {
    let (folder, state) = shop();
    let file = many_products(folder.path(), 50_000);
    let mut make = request(ImportEntity::Products, &file);
    make.create_missing = true;
    let started = Instant::now();
    let done = commands::import_commit(&state, &make).expect("imported");
    let took = started.elapsed();
    assert_eq!(done.counts.created, 50_000);
    assert_eq!(product_count(&state), 50_000);
    // Test builds are not optimised; the release build is several times faster.
    let limit = if cfg!(debug_assertions) { 180 } else { 60 };
    assert!(took < Duration::from_secs(limit), "took {took:?}");
}
