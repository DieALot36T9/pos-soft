//! Contract tests: every report keeps the `ReportTable` shape the UI, `io` and
//! `printing` rely on, and every report reads only views that exist.

#![allow(clippy::unwrap_used)]

mod support;

use pos_core::{Cell, ColumnKind, ReportTable};
use reports::{
    DateRange, GroupBy, SalesDimension, SalesFilters, customer_statement, daily_summary,
    discounts_overrides, low_stock, near_expiry, payables_aging, profit, receivables_aging,
    sales_by, shift_report, stock_movement, stock_value, supplier_statement, tax_report,
};
use rusqlite::Connection;
use support::{date, empty_db, id, shop, shop_context};

/// Every report, run once each on the given database.
fn every_report(conn: &Connection) -> Vec<ReportTable> {
    let shop = shop_context();
    let range = DateRange {
        from: date("2026-03-01"),
        to: date("2026-03-31"),
    };
    let today = date("2026-03-05");
    let filters = SalesFilters::default();
    let mut reports = vec![daily_summary(conn, today, None, &shop).unwrap()];
    for dimension in [
        SalesDimension::Product,
        SalesDimension::Category,
        SalesDimension::Brand,
        SalesDimension::Cashier,
        SalesDimension::Hour,
        SalesDimension::Weekday,
        SalesDimension::Date,
    ] {
        reports.push(sales_by(conn, dimension, range, &filters, &shop).unwrap());
    }
    for group in [GroupBy::Product, GroupBy::Category, GroupBy::Brand] {
        reports.push(profit(conn, group, range).unwrap());
        reports.push(stock_value(conn, group).unwrap());
    }
    reports.push(low_stock(conn).unwrap());
    reports.push(near_expiry(conn, today, 30).unwrap());
    reports.push(stock_movement(conn, range).unwrap());
    reports.push(tax_report(conn, range).unwrap());
    reports.push(payables_aging(conn, today).unwrap());
    reports.push(receivables_aging(conn, today).unwrap());
    reports.push(discounts_overrides(conn, range, &shop).unwrap());
    reports
}

fn cell_fits(cell: &Cell, kind: ColumnKind) -> bool {
    matches!(
        (cell, kind),
        (Cell::Empty, _)
            | (Cell::Text(_), ColumnKind::Text)
            | (Cell::Money(_), ColumnKind::Money)
            | (Cell::Qty(_), ColumnKind::Qty)
            | (Cell::Int(_), ColumnKind::Int)
            | (Cell::Percent(_), ColumnKind::Percent)
            | (Cell::Date(_), ColumnKind::Date)
    )
}

/// One cell per column, each of the column's kind, in the table and all its sections.
fn check_shape(table: &ReportTable) {
    assert!(!table.title.is_empty());
    let mut lines: Vec<&Vec<Cell>> = table.rows.iter().collect();
    lines.extend(table.totals.iter());
    for line in lines {
        assert_eq!(line.len(), table.columns.len(), "{}: {line:?}", table.title);
        for (cell, column) in line.iter().zip(&table.columns) {
            assert!(
                cell_fits(cell, column.kind),
                "{}: {cell:?} in {}",
                table.title,
                column.key
            );
        }
    }
    for section in &table.sections {
        check_shape(section);
    }
}

#[test]
fn every_report_has_one_cell_of_the_right_kind_per_column() {
    let conn = shop();
    let mut reports = every_report(&conn);
    reports.push(shift_report(&conn, &id("SH1"), &shop_context()).unwrap());
    reports.push(shift_report(&conn, &id("SH3"), &shop_context()).unwrap());
    let range = DateRange {
        from: date("2026-01-01"),
        to: date("2026-03-31"),
    };
    reports.push(supplier_statement(&conn, &id("SP1"), range).unwrap());
    reports.push(customer_statement(&conn, &id("CN"), range).unwrap());
    for report in &reports {
        check_shape(report);
    }
}

#[test]
fn every_report_runs_on_an_empty_database() {
    // This proves every view and column the reports read exists in the owners' migrations.
    let conn = empty_db();
    for report in every_report(&conn) {
        check_shape(&report);
        assert!(
            report.rows.is_empty() || report.title == "Daily summary",
            "{}",
            report.title
        );
    }
}

#[test]
fn column_keys_are_unique_in_each_table() {
    let conn = shop();
    for report in every_report(&conn) {
        let mut keys: Vec<&str> = report
            .columns
            .iter()
            .map(|column| column.key.as_str())
            .collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), report.columns.len(), "{}", report.title);
    }
}

#[test]
fn reports_owns_no_tables() {
    // The module page: "Owns no tables. Reads only *_v_* views."
    assert!(reports::migrations().migrations.is_empty());
    assert_eq!(reports::migrations().module, "reports");
}
