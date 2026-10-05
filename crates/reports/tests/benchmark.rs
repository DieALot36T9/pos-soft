//! Speed budgets over one year of reference data (RPT-11, NFR-P6, NFR-P7):
//! the daily summary within 1 s, every other report within 3 s.
//!
//! Building the year (about 1.6 million sale lines) takes a while, so this test runs
//! only when asked: `cargo nextest run -p reports --release --run-ignored only`.
//! `cargo bench -p reports` measures the same reports in more detail (criterion).

#![allow(clippy::unwrap_used)]

mod reference;

use std::time::{Duration, Instant};

use pos_core::{BusinessDate, Id, ReportTable};
use reports::{
    DateRange, GroupBy, SalesDimension, SalesFilters, ShopContext, customer_statement,
    daily_summary, discounts_overrides, low_stock, near_expiry, payables_aging, profit,
    receivables_aging, sales_by, shift_report, stock_movement, stock_value, supplier_statement,
    tax_report,
};
use rusqlite::Connection;

/// The fastest of three runs, so one slow moment on a busy computer doesn't fail the test.
fn best_of_three(mut work: impl FnMut() -> ReportTable) -> Duration {
    let mut best = Duration::MAX;
    for _ in 0..3 {
        let started = Instant::now();
        work();
        best = best.min(started.elapsed());
    }
    best
}

fn date(text: &str) -> BusinessDate {
    BusinessDate::parse(text).unwrap()
}

/// Every report with a name, as the app would run it over the reference year.
pub fn every_report(conn: &Connection) -> Vec<(String, Box<dyn Fn() -> ReportTable + '_>)> {
    let year = DateRange {
        from: date("2025-01-01"),
        to: date("2025-12-31"),
    };
    let today = date("2025-12-31");
    let mut list: Vec<(String, Box<dyn Fn() -> ReportTable + '_>)> = vec![
        (
            "shift report".to_string(),
            Box::new(move || {
                shift_report(
                    conn,
                    &id(&reference::shift_id(200, 1)),
                    &ShopContext::default(),
                )
                .unwrap()
            }),
        ),
        (
            "stock movement".to_string(),
            Box::new(move || stock_movement(conn, year).unwrap()),
        ),
        (
            "tax report".to_string(),
            Box::new(move || tax_report(conn, year).unwrap()),
        ),
        (
            "low stock".to_string(),
            Box::new(|| low_stock(conn).unwrap()),
        ),
        (
            "near expiry".to_string(),
            Box::new(move || near_expiry(conn, today, 30).unwrap()),
        ),
        (
            "payables aging".to_string(),
            Box::new(move || payables_aging(conn, today).unwrap()),
        ),
        (
            "receivables aging".to_string(),
            Box::new(move || receivables_aging(conn, today).unwrap()),
        ),
        (
            "discounts and overrides".to_string(),
            Box::new(move || discounts_overrides(conn, year, &ShopContext::default()).unwrap()),
        ),
        (
            "supplier statement".to_string(),
            Box::new(move || {
                supplier_statement(conn, &id(&reference::supplier_id(7)), year).unwrap()
            }),
        ),
        (
            "customer statement".to_string(),
            Box::new(move || {
                customer_statement(conn, &id(&reference::customer_id(5)), year).unwrap()
            }),
        ),
    ];
    for dimension in [
        SalesDimension::Product,
        SalesDimension::Category,
        SalesDimension::Brand,
        SalesDimension::Cashier,
        SalesDimension::Hour,
        SalesDimension::Weekday,
        SalesDimension::Date,
    ] {
        list.push((
            format!("sales by {dimension:?}"),
            Box::new(move || {
                let filters = SalesFilters::default();
                sales_by(conn, dimension, year, &filters, &ShopContext::default()).unwrap()
            }),
        ));
    }
    for group in [GroupBy::Product, GroupBy::Category, GroupBy::Brand] {
        list.push((
            format!("profit by {group:?}"),
            Box::new(move || profit(conn, group, year).unwrap()),
        ));
        list.push((
            format!("stock value by {group:?}"),
            Box::new(move || stock_value(conn, group).unwrap()),
        ));
    }
    list
}

fn id(text: &str) -> Id {
    Id::parse(text).unwrap()
}

/// The reference database, opened the way the app opens it (`db::Db`, with its cache
/// and memory-map settings). Building it takes a minute or two, so set
/// `REPORTS_REFERENCE_DB=/some/file.sqlite` to build it once and reuse it.
pub fn reference_db() -> (db::Db, Option<tempfile::TempDir>) {
    let (path, folder) = match std::env::var_os("REPORTS_REFERENCE_DB") {
        Some(path) => (std::path::PathBuf::from(path), None),
        None => {
            let folder = tempfile::tempdir().unwrap();
            (folder.path().join("reference.sqlite"), Some(folder))
        }
    };
    if !path.exists() {
        drop(reference::build(&path));
    }
    (db::Db::open(&path).unwrap(), folder)
}

#[test]
#[ignore = "builds one year of reference data; run with --run-ignored only"]
fn rpt_11_every_report_over_a_year_of_reference_data_is_fast_enough() {
    let (database, _folder) = reference_db();
    database
        .read(|conn| {
            check_budgets(conn);
            Ok(())
        })
        .unwrap();
}

fn check_budgets(conn: &Connection) {
    // NFR-P6: the daily summary within 1 s.
    let took = best_of_three(|| {
        daily_summary(conn, date("2025-07-01"), None, &ShopContext::default()).unwrap()
    });
    println!("daily summary: {took:?}");
    assert!(
        took <= Duration::from_secs(1),
        "daily summary took {took:?}"
    );

    // NFR-P7 / RPT-11: any report over one year within 3 s. Every report is timed
    // before the test fails, so one run shows all the slow ones.
    let mut too_slow = Vec::new();
    for (name, report) in every_report(conn) {
        let took = best_of_three(&report);
        println!("{name}: {took:?}");
        if took > Duration::from_secs(3) {
            too_slow.push(format!("{name} took {took:?}"));
        }
    }
    assert!(too_slow.is_empty(), "too slow: {too_slow:?}");
}
