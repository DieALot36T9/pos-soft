//! RPT-02 / SHF-08 shift reports (X while open, Z once closed).

#![allow(clippy::unwrap_used)]

mod support;

use pos_core::{AppError, Cell, Id};
use reports::shift_report;
use support::{id, m, q, row, section, shop, shop_context, t};

const E: Cell = Cell::Empty;

#[test]
fn shf_08_z_report_shows_the_shift_details() {
    let conn = shop();
    let report = shift_report(&conn, &id("SH1"), &shop_context()).unwrap();

    assert_eq!(report.title, "Z report");
    let details = section(&report, "Shift");
    assert_eq!(
        details.rows,
        vec![
            vec![t("Till"), t("Till 1")],
            vec![t("Cashier"), t("Rina")],
            vec![t("Business date"), t("2026-03-05")],
            vec![t("Opened"), t("2026-03-05 09:00")],
            vec![t("Closed"), t("2026-03-05 18:00")],
            vec![t("Status"), t("Closed")],
        ]
    );
}

#[test]
fn shf_08_z_report_counts_only_this_shift() {
    let conn = shop();
    let report = shift_report(&conn, &id("SH1"), &shop_context()).unwrap();

    // Till 2's sale and the 03-06 return are on other shifts.
    assert_eq!(
        row(&report, "Gross sales"),
        &vec![t("Gross sales"), Cell::Int(3), E, m(52_497)]
    );
    assert_eq!(row(&report, "Promotion discounts")[3], m(500));
    assert_eq!(row(&report, "Manual discounts")[3], m(1_000));
    assert_eq!(row(&report, "Returns")[3], m(10_000));
    assert_eq!(row(&report, "Net sales")[3], m(40_997));
    assert_eq!(row(&report, "Tax")[3], m(2_936));
    assert_eq!(row(&report, "Total taken")[3], m(41_000));
    assert_eq!(row(&report, "Voided sales")[1], Cell::Int(1));
    assert_eq!(row(&report, "Cancelled carts")[1], Cell::Int(1));
    assert_eq!(row(&report, "Average sale")[3], m(17_000));
    assert_eq!(row(&report, "Items sold")[2], q(8_000));

    let payments = section(&report, "Payments by method");
    assert_eq!(
        payments.rows,
        vec![
            vec![t("Card"), m(12_500), m(0), m(12_500)],
            vec![t("Cash"), m(38_500), m(10_000), m(28_500)],
        ]
    );
    let tax = section(&report, "Tax by rate");
    assert_eq!(tax.totals.as_ref().unwrap()[3], m(2_936));
}

#[test]
fn shf_08_cash_summary_matches_the_count() {
    let conn = shop();
    let report = shift_report(&conn, &id("SH1"), &shop_context()).unwrap();
    let cash = section(&report, "Cash");

    assert_eq!(
        cash.rows,
        vec![
            vec![t("Float"), m(100_000)],
            vec![t("Cash sales"), m(42_500)],
            vec![t("Cash refunds"), m(-10_000)],
            vec![t("Void reversals"), m(-4_000)],
            vec![t("Cash in"), m(20_000)],
            vec![t("Cash out"), m(-5_000)],
            vec![t("Cash drops"), m(-50_000)],
            vec![t("Customer payments"), m(0)],
            vec![t("Supplier payments"), m(0)],
            vec![t("Expected cash"), m(93_500)],
            vec![t("Counted cash"), m(93_000)],
            vec![t("Difference"), m(-500)],
        ]
    );
}

#[test]
fn shf_05_x_report_while_the_shift_is_open() {
    let conn = shop();
    let report = shift_report(&conn, &id("SH3"), &shop_context()).unwrap();

    assert_eq!(report.title, "X report – shift still open");
    assert_eq!(row(section(&report, "Shift"), "Closed")[1], E);
    assert_eq!(row(section(&report, "Shift"), "Status")[1], t("Open"));
    assert_eq!(row(&report, "Total taken")[3], m(4_000));
    assert_eq!(row(&report, "Cancelled carts")[1], Cell::Int(1));
    let cash = section(&report, "Cash");
    // 500.00 float + 40.00 cash sale; nothing counted yet.
    assert_eq!(row(cash, "Expected cash")[1], m(54_000));
    assert_eq!(row(cash, "Counted cash")[1], E);
    assert_eq!(row(cash, "Difference")[1], E);
}

#[test]
fn rpt_02_unknown_shift_is_not_found() {
    let conn = shop();
    let error = shift_report(&conn, &Id::new(), &shop_context()).unwrap_err();

    assert!(matches!(
        AppError::from(error),
        AppError::NotFound { entity, .. } if entity == "shift"
    ));
}
