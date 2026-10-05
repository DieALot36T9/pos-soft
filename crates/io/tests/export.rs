//! Contract tests for exports (IO-03) and report export (RPT-09).
//! Page: `docs/05 Modules/Module - io.md`. Test names start with the requirement ID.

// Test helpers outside #[test] functions may unwrap too: a broken test file should stop the test.
#![allow(clippy::unwrap_used)]

use std::path::Path;

use calamine::{Data, Reader, Xlsx, open_workbook};
use chrono::{NaiveDate, NaiveDateTime};
use io::{
    ExportHeader, ExportLabels, Table, XlsxCell, XlsxSheet, parse_money, parse_qty, read_file,
    report_to_xlsx, sheet_names, write_csv, write_xlsx,
};
use pos_core::{
    Bps, BusinessDate, Cell, Column, ColumnKind, Money, Qty, ReportFilter, ReportParams,
    ReportTable,
};
use tempfile::TempDir;

fn text(cells: &[&str]) -> Vec<String> {
    cells.iter().map(|cell| cell.to_string()).collect()
}

fn read_back_xlsx(path: &Path, sheet: &str) -> calamine::Range<Data> {
    let mut workbook: Xlsx<_> = open_workbook(path).unwrap();
    workbook.worksheet_range(sheet).unwrap()
}

// ---------- IO-03: exports ----------

#[test]
fn io_03_csv_round_trip_gives_the_same_table_including_symbols() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("export.csv");
    let table = Table {
        headers: text(&["SKU", "Name", "Price", "Barcodes", "Note"]),
        rows: vec![
            text(&[
                "A1",
                "Milk ৳ 1L",
                "12.50",
                "8901|8902",
                "line one\nline two",
            ]),
            text(&["A2", "Rice; \"Miniket\"", "-3.00", "", "=SUM(A1:A9)"]),
            text(&["A3", "+Plus", "+880171", "@home", "-minus"]),
        ],
    };

    write_csv(&path, &table).unwrap();
    let read_back = read_file(&path, None).unwrap();

    assert_eq!(read_back, table);
}

#[test]
fn io_03_csv_starts_with_a_bom_so_excel_shows_symbols() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("export.csv");
    let table = Table {
        headers: text(&["Name"]),
        rows: vec![text(&["৳"])],
    };

    write_csv(&path, &table).unwrap();
    let bytes = std::fs::read(&path).unwrap();

    assert!(bytes.starts_with(b"\xef\xbb\xbf"));
    assert_eq!(String::from_utf8_lossy(&bytes[3..]), "Name\r\n৳\r\n");
}

#[test]
fn io_03_csv_guards_against_spreadsheet_formulas() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("export.csv");
    let table = Table {
        headers: text(&["Name"]),
        rows: vec![
            text(&["=HYPERLINK(\"http://x\")"]),
            text(&["+1+2"]),
            text(&["-2-3"]),
            text(&["@SUM(A1)"]),
            text(&["-12.50"]),
            text(&["+8801711000000"]),
        ],
    };

    write_csv(&path, &table).unwrap();
    let written = String::from_utf8(std::fs::read(&path).unwrap()[3..].to_vec()).unwrap();

    let lines: Vec<&str> = written.split("\r\n").collect();
    assert_eq!(lines[1], "\"'=HYPERLINK(\"\"http://x\"\")\"");
    assert_eq!(lines[2], "'+1+2");
    assert_eq!(lines[3], "'-2-3");
    assert_eq!(lines[4], "'@SUM(A1)");
    // Plain numbers are not formulas, so they stay as they are.
    assert_eq!(lines[5], "-12.50");
    assert_eq!(lines[6], "+8801711000000");
}

#[test]
fn io_03_xlsx_round_trip_gives_the_same_table() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("export.xlsx");
    let sheets = vec![XlsxSheet {
        name: "Products".to_string(),
        headers: text(&["SKU", "Name", "Price", "Stock", "Units", "Expiry", "Note"]),
        rows: vec![
            vec![
                XlsxCell::Text("A1".to_string()),
                XlsxCell::Text("Milk ৳ 1L".to_string()),
                XlsxCell::Money {
                    amount: Money(1250),
                    minor_digits: 2,
                },
                XlsxCell::Qty(Qty(1234)),
                XlsxCell::Int(12),
                XlsxCell::Date(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
                XlsxCell::Text("=SUM(A1:A2)".to_string()),
            ],
            vec![
                XlsxCell::Text("A2".to_string()),
                XlsxCell::Empty,
                XlsxCell::Money {
                    amount: Money(-300),
                    minor_digits: 2,
                },
                XlsxCell::Qty(Qty(2000)),
                XlsxCell::Int(-1),
                XlsxCell::Empty,
                XlsxCell::Empty,
            ],
        ],
    }];

    write_xlsx(&path, &sheets).unwrap();
    let table = read_file(&path, None).unwrap();

    assert_eq!(
        table.headers,
        text(&["SKU", "Name", "Price", "Stock", "Units", "Expiry", "Note"])
    );
    assert_eq!(
        table.rows[0],
        text(&[
            "A1",
            "Milk ৳ 1L",
            "12.5",
            "1.234",
            "12",
            "2026-03-01",
            "=SUM(A1:A2)"
        ])
    );
    assert_eq!(table.rows[1], text(&["A2", "", "-3", "2", "-1", "", ""]));
    // Each cell reads back as the value that was written.
    assert_eq!(parse_money(&table.rows[0][2], 2), Ok(Money(1250)));
    assert_eq!(parse_qty(&table.rows[0][3], 3), Ok(Qty(1234)));
}

#[test]
fn io_03_xlsx_money_and_qty_are_real_numbers_and_text_is_never_a_formula() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("export.xlsx");
    let sheets = vec![XlsxSheet {
        name: "Products".to_string(),
        headers: text(&["Price", "Stock", "Note"]),
        rows: vec![vec![
            XlsxCell::Money {
                amount: Money(1250),
                minor_digits: 2,
            },
            XlsxCell::Qty(Qty(1500)),
            XlsxCell::Text("=1+1".to_string()),
        ]],
    }];

    write_xlsx(&path, &sheets).unwrap();
    let range = read_back_xlsx(&path, "Products");

    assert_eq!(range.get_value((1, 0)), Some(&Data::Float(12.5)));
    assert_eq!(range.get_value((1, 1)), Some(&Data::Float(1.5)));
    assert_eq!(
        range.get_value((1, 2)),
        Some(&Data::String("=1+1".to_string()))
    );
}

#[test]
fn io_03_bad_sheet_names_are_made_safe() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("export.xlsx");
    let sheets = vec![XlsxSheet {
        name: "Sales [1/2]: a very long report name indeed".to_string(),
        headers: text(&["A"]),
        rows: vec![],
    }];

    write_xlsx(&path, &sheets).unwrap();

    // Excel allows at most 31 characters and none of [ ] : * ? / \
    assert_eq!(
        sheet_names(&path).unwrap(),
        text(&["Sales (1-2)- a very long report"])
    );
}

// ---------- RPT-09: report export ----------

fn sample_report() -> ReportTable {
    let date = |text: &str| BusinessDate::parse(text).unwrap();
    ReportTable {
        title: "Daily summary".to_string(),
        params: ReportParams {
            from: Some(date("2026-03-01")),
            to: Some(date("2026-03-07")),
            filters: vec![ReportFilter {
                name: "Cashier".to_string(),
                value: "Rina".to_string(),
            }],
        },
        columns: vec![
            Column {
                key: "day".to_string(),
                title: "Day".to_string(),
                kind: ColumnKind::Date,
            },
            Column {
                key: "net_sales".to_string(),
                title: "Net sales".to_string(),
                kind: ColumnKind::Money,
            },
            Column {
                key: "kg".to_string(),
                title: "Kg sold".to_string(),
                kind: ColumnKind::Qty,
            },
            Column {
                key: "sales".to_string(),
                title: "Sales".to_string(),
                kind: ColumnKind::Int,
            },
            Column {
                key: "margin".to_string(),
                title: "Margin".to_string(),
                kind: ColumnKind::Percent,
            },
            Column {
                key: "note".to_string(),
                title: "Note".to_string(),
                kind: ColumnKind::Text,
            },
        ],
        rows: vec![vec![
            Cell::Date(date("2026-03-01")),
            Cell::Money(Money(123_450)),
            Cell::Qty(Qty(1234)),
            Cell::Int(42),
            Cell::Percent(Bps(1250)),
            Cell::Text("=cmd".to_string()),
        ]],
        totals: Some(vec![
            Cell::Empty,
            Cell::Money(Money(123_450)),
            Cell::Qty(Qty(1234)),
            Cell::Int(42),
            Cell::Empty,
            Cell::Empty,
        ]),
        sections: vec![ReportTable {
            title: "Payments".to_string(),
            params: ReportParams::default(),
            columns: vec![
                Column {
                    key: "method".to_string(),
                    title: "Method".to_string(),
                    kind: ColumnKind::Text,
                },
                Column {
                    key: "amount".to_string(),
                    title: "Amount".to_string(),
                    kind: ColumnKind::Money,
                },
            ],
            rows: vec![vec![
                Cell::Text("Cash".to_string()),
                Cell::Money(Money(100_000)),
            ]],
            totals: None,
            sections: vec![],
        }],
    }
}

fn sample_header() -> ExportHeader {
    ExportHeader {
        shop_name: "Rahim Store".to_string(),
        generated_at: NaiveDateTime::parse_from_str("2026-03-08 09:15", "%Y-%m-%d %H:%M").unwrap(),
        money_minor_digits: 2,
        labels: ExportLabels::default(),
    }
}

#[test]
fn rpt_09_report_export_has_the_header_lines() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("report.xlsx");

    report_to_xlsx(&path, &sample_report(), &sample_header()).unwrap();
    let range = read_back_xlsx(&path, "Daily summary");

    let cell = |row: u32, column: u32| range.get_value((row, column)).cloned();
    assert_eq!(cell(0, 0), Some(Data::String("Rahim Store".to_string())));
    assert_eq!(cell(1, 0), Some(Data::String("Daily summary".to_string())));
    assert_eq!(
        cell(2, 0),
        Some(Data::String("Period: 2026-03-01 to 2026-03-07".to_string()))
    );
    assert_eq!(
        cell(3, 0),
        Some(Data::String("Generated: 2026-03-08 09:15".to_string()))
    );
    assert_eq!(cell(4, 0), Some(Data::String("Cashier: Rina".to_string())));
}

#[test]
fn rpt_09_report_export_writes_real_numbers_totals_and_sections() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("report.xlsx");

    report_to_xlsx(&path, &sample_report(), &sample_header()).unwrap();
    let range = read_back_xlsx(&path, "Daily summary");

    let cell = |row: u32, column: u32| range.get_value((row, column)).cloned();
    // Row 5 is blank, row 6 has the column titles, row 7 the data.
    assert_eq!(cell(6, 1), Some(Data::String("Net sales".to_string())));
    assert!(matches!(cell(7, 0), Some(Data::DateTime(_))));
    assert_eq!(cell(7, 1), Some(Data::Float(1234.5)));
    assert_eq!(cell(7, 2), Some(Data::Float(1.234)));
    assert_eq!(cell(7, 3), Some(Data::Float(42.0)));
    assert_eq!(cell(7, 4), Some(Data::Float(0.125)));
    assert_eq!(cell(7, 5), Some(Data::String("=cmd".to_string())));
    assert_eq!(cell(8, 0), Some(Data::String("Total".to_string())));
    assert_eq!(cell(8, 1), Some(Data::Float(1234.5)));
    // A blank row, then the section title, its column titles and its rows.
    assert_eq!(cell(10, 0), Some(Data::String("Payments".to_string())));
    assert_eq!(cell(11, 1), Some(Data::String("Amount".to_string())));
    assert_eq!(cell(12, 0), Some(Data::String("Cash".to_string())));
    assert_eq!(cell(12, 1), Some(Data::Float(1000.0)));
}

#[test]
fn rpt_09_report_export_header_words_come_from_the_labels() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("report.xlsx");
    let mut header = sample_header();
    header.labels = ExportLabels {
        period: "Zeitraum".to_string(),
        period_to: "bis".to_string(),
        generated: "Erstellt".to_string(),
        total: "Summe".to_string(),
    };

    report_to_xlsx(&path, &sample_report(), &header).unwrap();
    let range = read_back_xlsx(&path, "Daily summary");

    assert_eq!(
        range.get_value((2, 0)),
        Some(&Data::String(
            "Zeitraum: 2026-03-01 bis 2026-03-07".to_string()
        ))
    );
    assert_eq!(
        range.get_value((8, 0)),
        Some(&Data::String("Summe".to_string()))
    );
}
