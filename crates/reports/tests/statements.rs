//! RPT-07 supplier and customer statements: opening balance, every ledger row in the
//! period with the running balance, and the closing balance.

#![allow(clippy::unwrap_used)]

mod support;

use pos_core::{AppError, Cell, Id};
use reports::{DateRange, customer_statement, supplier_statement};
use support::{date, id, m, shop, t};

const E: Cell = Cell::Empty;

fn day(text: &str) -> Cell {
    Cell::Date(date(text))
}

#[test]
fn rpt_07_supplier_statement_has_opening_rows_and_closing() {
    let conn = shop();
    let range = DateRange {
        from: date("2026-02-01"),
        to: date("2026-03-05"),
    };
    let report = supplier_statement(&conn, &id("SP1"), range).unwrap();

    assert_eq!(report.title, "Supplier statement");
    assert_eq!(report.params.filters[0].value, "Fresh Dairy");
    assert_eq!(
        report.rows,
        vec![
            // The 01-10 invoice is before the period, so it is the opening balance.
            vec![day("2026-02-01"), t("Opening balance"), E, E, m(500_000)],
            vec![day("2026-02-15"), t("Invoice"), E, m(300_000), m(800_000)],
            vec![day("2026-03-01"), t("Invoice"), E, m(200_000), m(1_000_000)],
            vec![day("2026-03-02"), t("Payment"), E, m(-600_000), m(400_000)],
        ]
    );
    assert_eq!(
        report.totals,
        Some(vec![
            day("2026-03-05"),
            t("Closing balance"),
            E,
            E,
            m(400_000)
        ])
    );
}

#[test]
fn rpt_07_customer_statement_has_opening_rows_and_closing() {
    let conn = shop();
    let range = DateRange {
        from: date("2026-03-01"),
        to: date("2026-03-05"),
    };
    let report = customer_statement(&conn, &id("CN"), range).unwrap();

    assert_eq!(report.title, "Customer statement");
    assert_eq!(report.params.filters[0].value, "Nadia");
    assert_eq!(
        report.rows,
        vec![
            vec![day("2026-03-01"), t("Opening balance"), E, E, m(50_000)],
            vec![day("2026-03-01"), t("Credit sale"), E, m(30_000), m(80_000)],
            vec![day("2026-03-02"), t("Payment"), E, m(-40_000), m(40_000)],
        ]
    );
    assert_eq!(report.totals.unwrap()[4], m(40_000));
}

#[test]
fn rpt_07_statement_for_an_unknown_supplier_or_customer_is_not_found() {
    let conn = shop();
    let range = DateRange {
        from: date("2026-03-01"),
        to: date("2026-03-05"),
    };
    let supplier = supplier_statement(&conn, &Id::new(), range).unwrap_err();
    assert!(matches!(
        AppError::from(supplier),
        AppError::NotFound { entity, .. } if entity == "supplier"
    ));
    let customer = customer_statement(&conn, &Id::new(), range).unwrap_err();
    assert!(matches!(
        AppError::from(customer),
        AppError::NotFound { entity, .. } if entity == "customer"
    ));
}

#[test]
fn rpt_07_statement_refuses_a_backwards_date_range() {
    let conn = shop();
    let backwards = DateRange {
        from: date("2026-03-05"),
        to: date("2026-03-01"),
    };
    let error = customer_statement(&conn, &id("CN"), backwards).unwrap_err();
    assert_eq!(
        AppError::from(error),
        AppError::validation("date_range_backwards")
    );
}
