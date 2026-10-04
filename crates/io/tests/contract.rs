//! Contract tests: the public API behaves as the module page says.
//! Page: `docs/05 Modules/Module - io.md`. Test names start with the requirement ID.

// Test helpers outside #[test] functions may unwrap too: a broken test file should stop the test.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use chrono::NaiveDate;
use io::{
    CellErr, FieldSpec, Table, XlsxCell, XlsxSheet, auto_map, parse_bool, parse_date, parse_money,
    parse_qty, read_file, sheet_names, write_csv, write_xlsx,
};
use pos_core::{AppError, Money, Qty};
use tempfile::TempDir;

fn text(cells: &[&str]) -> Vec<String> {
    cells.iter().map(|cell| cell.to_string()).collect()
}

fn write_bytes(dir: &TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

// ---------- IO-01.2: choosing a file ----------

#[test]
fn io_01_csv_with_bom_and_commas_is_read() {
    let dir = TempDir::new().unwrap();
    let path = write_bytes(
        &dir,
        "products.csv",
        "\u{feff}SKU,Name,Price\r\nA1,Milk ৳,12.50\r\n".as_bytes(),
    );

    let table = read_file(&path, None).unwrap();

    assert_eq!(table.headers, text(&["SKU", "Name", "Price"]));
    assert_eq!(table.rows, vec![text(&["A1", "Milk ৳", "12.50"])]);
}

#[test]
fn io_01_csv_with_semicolons_is_read() {
    let dir = TempDir::new().unwrap();
    let path = write_bytes(
        &dir,
        "products.csv",
        b"SKU;Name;Price\nA1;Rice, 5 kg;450.00\n",
    );

    let table = read_file(&path, None).unwrap();

    assert_eq!(table.headers, text(&["SKU", "Name", "Price"]));
    assert_eq!(table.rows, vec![text(&["A1", "Rice, 5 kg", "450.00"])]);
}

#[test]
fn io_01_csv_with_quoted_newlines_keeps_them_in_the_cell() {
    let dir = TempDir::new().unwrap();
    let path = write_bytes(
        &dir,
        "customers.csv",
        b"Name,Address\nRina,\"House 4\nRoad 7\"\nKarim,Dhaka\n",
    );

    let table = read_file(&path, None).unwrap();

    assert_eq!(
        table.rows,
        vec![
            text(&["Rina", "House 4\nRoad 7"]),
            text(&["Karim", "Dhaka"])
        ]
    );
}

#[test]
fn io_01_short_rows_are_filled_and_trailing_blank_rows_dropped() {
    let dir = TempDir::new().unwrap();
    let path = write_bytes(&dir, "short.csv", b"A,B,C\n1\n,,\n2,3,4\n,,\n,,\n");

    let table = read_file(&path, None).unwrap();

    // The blank row in the middle stays, so row numbers still match the file.
    assert_eq!(
        table.rows,
        vec![
            text(&["1", "", ""]),
            text(&["", "", ""]),
            text(&["2", "3", "4"])
        ]
    );
}

#[test]
fn io_01_xlsx_numbers_stored_as_numbers_and_as_text_both_read_as_text() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("products.xlsx");
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.write_string(0, 0, "SKU").unwrap();
    sheet.write_string(0, 1, "Price").unwrap();
    sheet.write_string(0, 2, "Stock").unwrap();
    sheet.write_string(0, 3, "Expiry").unwrap();
    sheet.write_string(1, 0, "A1").unwrap();
    sheet.write_number(1, 1, 12.5).unwrap();
    sheet.write_number(1, 2, 40.0).unwrap();
    let date = rust_xlsxwriter::ExcelDateTime::from_ymd(2026, 3, 1).unwrap();
    let date_format = rust_xlsxwriter::Format::new().set_num_format("dd/mm/yyyy");
    sheet
        .write_datetime_with_format(1, 3, &date, &date_format)
        .unwrap();
    sheet.write_string(2, 0, "A2").unwrap();
    sheet.write_string(2, 1, "12.50").unwrap();
    sheet.write_string(2, 2, "7").unwrap();
    workbook.save(&path).unwrap();

    let table = read_file(&path, None).unwrap();

    assert_eq!(table.headers, text(&["SKU", "Price", "Stock", "Expiry"]));
    assert_eq!(table.rows[0], text(&["A1", "12.5", "40", "2026-03-01"]));
    assert_eq!(table.rows[1], text(&["A2", "12.50", "7", ""]));
    assert_eq!(parse_money(&table.rows[0][1], 2), Ok(Money(1250)));
    assert_eq!(parse_money(&table.rows[1][1], 2), Ok(Money(1250)));
}

#[test]
fn io_01_xlsx_reads_the_first_sheet_or_a_chosen_sheet() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("two.xlsx");
    let sheets = vec![
        XlsxSheet {
            name: "Products".to_string(),
            headers: text(&["SKU"]),
            rows: vec![vec![XlsxCell::Text("A1".to_string())]],
        },
        XlsxSheet {
            name: "Customers".to_string(),
            headers: text(&["Name"]),
            rows: vec![vec![XlsxCell::Text("Rina".to_string())]],
        },
    ];
    write_xlsx(&path, &sheets).unwrap();

    assert_eq!(
        sheet_names(&path).unwrap(),
        text(&["Products", "Customers"])
    );
    assert_eq!(read_file(&path, None).unwrap().headers, text(&["SKU"]));
    let customers = read_file(&path, Some("Customers")).unwrap();
    assert_eq!(customers.rows, vec![text(&["Rina"])]);
}

#[test]
fn io_01_a_missing_sheet_is_a_clear_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("one.xlsx");
    let sheets = vec![XlsxSheet {
        name: "Products".to_string(),
        headers: text(&["SKU"]),
        rows: vec![],
    }];
    write_xlsx(&path, &sheets).unwrap();

    let error = AppError::from(read_file(&path, Some("Stock")).unwrap_err());

    assert!(matches!(error, AppError::Validation { code, .. } if code == "io_sheet_not_found"));
}

#[test]
fn io_01_csv_files_have_one_sheet_with_no_name() {
    let dir = TempDir::new().unwrap();
    let path = write_bytes(&dir, "a.csv", b"A\n1\n");

    assert_eq!(sheet_names(&path).unwrap(), Vec::<String>::new());
}

#[test]
fn io_01_unknown_file_types_and_missing_files_are_errors() {
    let dir = TempDir::new().unwrap();
    let pdf = write_bytes(&dir, "a.pdf", b"%PDF");
    let missing = dir.path().join("missing.csv");

    let wrong_type = AppError::from(read_file(&pdf, None).unwrap_err());
    let not_found = AppError::from(read_file(&missing, None).unwrap_err());

    assert!(
        matches!(wrong_type, AppError::Validation { code, .. } if code == "io_unknown_file_type")
    );
    assert!(matches!(not_found, AppError::Validation { code, .. } if code == "io_file_unreadable"));
}

#[test]
fn io_01_an_empty_file_is_an_error() {
    let dir = TempDir::new().unwrap();
    let path = write_bytes(&dir, "empty.csv", b"\xef\xbb\xbf");

    let error = AppError::from(read_file(&path, None).unwrap_err());

    assert!(matches!(error, AppError::Validation { code, .. } if code == "io_file_empty"));
}

// ---------- IO-01.3: mapping columns ----------

#[test]
fn io_01_auto_map_matches_names_and_common_alternatives() {
    let headers = text(&["Item code", "MRP", "Product name", "Colour"]);
    let fields = vec![
        FieldSpec::new("sku", &["SKU", "Item code", "Code"]),
        FieldSpec::new("name", &["Name", "Product name"]),
        FieldSpec::new("sale_price", &["Price", "Sale price", "MRP"]),
        FieldSpec::new("barcode", &["Barcode"]),
    ];

    assert_eq!(
        auto_map(&headers, &fields),
        vec![Some(0), Some(2), Some(1), None]
    );
}

#[test]
fn io_01_auto_map_ignores_case_spaces_and_punctuation() {
    let headers = text(&[" sale-price ", "SALE_PRICE"]);
    let fields = vec![FieldSpec::new("sale_price", &["Sale price"])];

    // Each header is used once; the first match wins.
    assert_eq!(auto_map(&headers, &fields), vec![Some(0)]);
}

#[test]
fn io_01_auto_map_uses_each_header_only_once() {
    let headers = text(&["Price"]);
    let fields = vec![
        FieldSpec::new("sale_price", &["Price"]),
        FieldSpec::new("cost_price", &["Price", "Cost"]),
    ];

    assert_eq!(auto_map(&headers, &fields), vec![Some(0), None]);
}

#[test]
fn io_01_cell_errors_carry_row_column_code_and_value() {
    let table = Table {
        headers: text(&["Sale price"]),
        rows: vec![text(&["12,5O"])],
    };

    let errors = CellErr::check_column(&table, 0, |cell| parse_money(cell, 2).map(|_| ()));

    assert_eq!(
        errors,
        vec![CellErr {
            row: 2,
            column: "Sale price".to_string(),
            code: "money_has_separator".to_string(),
            value: "12,5O".to_string(),
        }]
    );
}

// ---------- IO-04: numbers and money in files ----------

#[test]
fn io_04_money_uses_the_currency_decimals_and_never_rounds() {
    assert_eq!(parse_money("12.50", 2), Ok(Money(1250)));
    assert_eq!(parse_money(" 12.5 ", 2), Ok(Money(1250)));
    assert_eq!(parse_money("-3", 2), Ok(Money(-300)));
    assert_eq!(parse_money("12", 0), Ok(Money(12)));
    assert_eq!(
        parse_money("12.505", 2),
        Err("money_too_many_decimals".to_string())
    );
    assert_eq!(
        parse_money("12.5", 0),
        Err("money_too_many_decimals".to_string())
    );
}

#[test]
fn io_04_money_rejects_thousands_separators_because_they_are_ambiguous() {
    // "1,234" is 1234 in Bangladesh and 1.234 in much of Europe.
    assert_eq!(
        parse_money("1,234", 2),
        Err("money_has_separator".to_string())
    );
    assert_eq!(
        parse_money("1.234,50", 2),
        Err("money_has_separator".to_string())
    );
    assert_eq!(
        parse_money("1 234.50", 2),
        Err("money_has_separator".to_string())
    );
    assert_eq!(
        parse_money("1'234.50", 2),
        Err("money_has_separator".to_string())
    );
    assert_eq!(parse_money("", 2), Err("money_empty".to_string()));
    assert_eq!(parse_money("৳12", 2), Err("money_not_a_number".to_string()));
    assert_eq!(
        parse_money("12.5O", 2),
        Err("money_not_a_number".to_string())
    );
}

#[test]
fn io_04_quantities_use_up_to_three_decimals() {
    assert_eq!(parse_qty("1.234", 3), Ok(Qty(1234)));
    assert_eq!(parse_qty("2", 0), Ok(Qty(2000)));
    assert_eq!(parse_qty("0.5", 3), Ok(Qty(500)));
    assert_eq!(parse_qty("-1.5", 1), Ok(Qty(-1500)));
    assert_eq!(
        parse_qty("1.5", 0),
        Err("qty_too_many_decimals".to_string())
    );
    assert_eq!(
        parse_qty("1.2345", 3),
        Err("qty_too_many_decimals".to_string())
    );
    assert_eq!(parse_qty("1", 4), Err("qty_too_many_decimals".to_string()));
    assert_eq!(parse_qty("1,000", 3), Err("qty_has_separator".to_string()));
    assert_eq!(parse_qty("", 3), Err("qty_empty".to_string()));
    assert_eq!(parse_qty("two", 3), Err("qty_not_a_number".to_string()));
}

#[test]
fn io_04_yes_no_true_false_one_zero() {
    for yes in ["yes", "YES", " Yes ", "true", "TRUE", "1"] {
        assert_eq!(parse_bool(yes), Ok(true), "{yes}");
    }
    for no in ["no", "No", "false", "False", "0"] {
        assert_eq!(parse_bool(no), Ok(false), "{no}");
    }
    assert_eq!(parse_bool(""), Err("bool_empty".to_string()));
    assert_eq!(parse_bool("maybe"), Err("bool_not_yes_no".to_string()));
}

#[test]
fn io_04_dates_in_iso_dd_mm_yyyy_and_excel_serial() {
    let first_march = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
    assert_eq!(parse_date("2026-03-01"), Ok(first_march));
    assert_eq!(parse_date("01/03/2026"), Ok(first_march));
    assert_eq!(parse_date("1/3/2026"), Ok(first_march));
    // Excel counts days from 1899-12-30. 46082 is 2026-03-01.
    assert_eq!(parse_date("46082"), Ok(first_march));
    assert_eq!(parse_date("46082.75"), Ok(first_march));
    assert_eq!(
        parse_date("1"),
        Ok(NaiveDate::from_ymd_opt(1900, 1, 1).unwrap())
    );
    assert_eq!(parse_date(""), Err("date_empty".to_string()));
    assert_eq!(parse_date("31/02/2026"), Err("date_not_a_date".to_string()));
    assert_eq!(parse_date("tomorrow"), Err("date_not_a_date".to_string()));
    // Day 60 is 29 February 1900, which never existed (an old Excel mistake).
    assert_eq!(parse_date("60"), Err("date_not_a_date".to_string()));
    assert_eq!(parse_date("0"), Err("date_not_a_date".to_string()));
}

#[test]
fn io_errors_become_validation_errors_for_the_ui() {
    let dir = TempDir::new().unwrap();
    let folder_as_file = dir.path().join("no_such_folder").join("out.csv");
    let table = Table {
        headers: text(&["A"]),
        rows: vec![],
    };

    let error = AppError::from(write_csv(&folder_as_file, &table).unwrap_err());

    assert!(matches!(error, AppError::Validation { code, .. } if code == "io_file_not_saved"));
}
