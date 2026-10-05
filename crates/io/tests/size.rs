//! IO-05: big files. Reading 50,000 rows must take 5 seconds or less in the
//! shipped (release) build, so the whole import (read, check, save) fits in the
//! 60 seconds IO-05 allows. Test builds are not optimised and run about twice as
//! slowly, so they get twice the time. Run `cargo nextest run -p io --release`
//! to check the real limit.

// Test helpers outside #[test] functions may unwrap too: a broken test file should stop the test.
#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use io::{Table, XlsxCell, XlsxSheet, read_file, write_csv, write_xlsx};
use pos_core::{Money, Qty};
use tempfile::TempDir;

const ROWS: usize = 50_000;
#[cfg(not(debug_assertions))]
const LIMIT: Duration = Duration::from_secs(5);
#[cfg(debug_assertions)]
const LIMIT: Duration = Duration::from_secs(10);

fn product_row(number: usize) -> Vec<String> {
    vec![
        format!("SKU-{number:06}"),
        format!("Product number {number} ৳"),
        format!("{}.{:02}", number % 1000, number % 100),
        format!("{}.{:03}", number % 50, number % 1000),
        format!("890{number:010}|891{number:010}"),
        "yes".to_string(),
        "2026-03-01".to_string(),
        "Dhaka".to_string(),
    ]
}

fn headers() -> Vec<String> {
    [
        "SKU", "Name", "Price", "Stock", "Barcodes", "Active", "Expiry", "Shop",
    ]
    .iter()
    .map(|header| header.to_string())
    .collect()
}

#[test]
fn io_05_reads_50k_csv_rows_within_5_seconds() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("big.csv");
    let table = Table {
        headers: headers(),
        rows: (0..ROWS).map(product_row).collect(),
    };
    write_csv(&path, &table).unwrap();

    let started = Instant::now();
    let read_back = read_file(&path, None).unwrap();
    let took = started.elapsed();

    assert_eq!(read_back.rows.len(), ROWS);
    assert!(took <= LIMIT, "reading 50k CSV rows took {took:?}");
}

#[test]
fn io_05_reads_50k_xlsx_rows_within_5_seconds() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("big.xlsx");
    let rows = (0..ROWS)
        .map(|number| {
            let mut cells: Vec<XlsxCell> = product_row(number)
                .into_iter()
                .map(XlsxCell::Text)
                .collect();
            cells[2] = XlsxCell::Money {
                amount: Money(i64::try_from(number).unwrap()),
                minor_digits: 2,
            };
            cells[3] = XlsxCell::Qty(Qty(i64::try_from(number).unwrap()));
            cells
        })
        .collect();
    let sheets = vec![XlsxSheet {
        name: "Products".to_string(),
        headers: headers(),
        rows,
    }];
    write_xlsx(&path, &sheets).unwrap();

    let started = Instant::now();
    let read_back = read_file(&path, None).unwrap();
    let took = started.elapsed();

    assert_eq!(read_back.rows.len(), ROWS);
    assert!(took <= LIMIT, "reading 50k XLSX rows took {took:?}");
}
