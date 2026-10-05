//! RPT-01 daily summary, checked against the fixture shop's hand-counted day.

#![allow(clippy::unwrap_used)]

mod support;

use pos_core::{Bps, Cell, Qty};
use reports::daily_summary;
use support::{date, id, m, q, row, section, shop, shop_context, t};

const E: Cell = Cell::Empty;

#[test]
fn rpt_01_daily_summary_adds_up_the_day() {
    let conn = shop();
    let report = daily_summary(&conn, date("2026-03-05"), None, &shop_context()).unwrap();

    assert_eq!(report.title, "Daily summary");
    assert_eq!(report.params.from, Some(date("2026-03-05")));
    // Four completed sales (the voided one is left out): 250.00 + 140.00 + 134.97 + 40.00.
    assert_eq!(
        row(&report, "Gross sales"),
        &vec![t("Gross sales"), Cell::Int(4), E, m(56_497)]
    );
    assert_eq!(row(&report, "Promotion discounts")[3], m(500));
    assert_eq!(row(&report, "Customer discounts")[3], m(0));
    assert_eq!(row(&report, "Manual discounts")[3], m(1_000));
    assert_eq!(
        row(&report, "Returns"),
        &vec![t("Returns"), Cell::Int(1), E, m(10_000)]
    );
    // 549.97 after discounts − 100.00 returned.
    assert_eq!(row(&report, "Net sales")[3], m(44_997));
    // 47.62 collected − 13.04 refunded.
    assert_eq!(row(&report, "Tax")[3], m(3_458));
    assert_eq!(row(&report, "Rounding")[3], m(3));
    assert_eq!(row(&report, "Total taken")[3], m(45_000));
    assert_eq!(
        row(&report, "Voided sales"),
        &vec![t("Voided sales"), Cell::Int(1), E, m(4_000)]
    );
    assert_eq!(
        row(&report, "Cancelled carts"),
        &vec![t("Cancelled carts"), Cell::Int(2), E, E]
    );
    // 550.00 taken by 4 sales.
    assert_eq!(row(&report, "Average sale")[3], m(13_750));
    assert_eq!(
        row(&report, "Items sold"),
        &vec![t("Items sold"), E, q(9_000), E]
    );
}

#[test]
fn rpt_01_tax_is_listed_by_rate() {
    let conn = shop();
    let report = daily_summary(&conn, date("2026-03-05"), None, &shop_context()).unwrap();
    let tax = section(&report, "Tax by rate");

    assert_eq!(
        tax.rows,
        vec![
            vec![t("Exempt 0%"), Cell::Percent(Bps(0)), m(18_497), m(0)],
            vec![
                t("Standard 15%"),
                Cell::Percent(Bps(1_500)),
                m(23_042),
                m(3_458)
            ],
        ]
    );
    assert_eq!(tax.totals, Some(vec![t("Total"), E, m(41_539), m(3_458)]));
}

#[test]
fn rpt_01_payments_by_method_are_sales_minus_refunds() {
    let conn = shop();
    let report = daily_summary(&conn, date("2026-03-05"), None, &shop_context()).unwrap();
    let payments = section(&report, "Payments by method");

    assert_eq!(
        payments.rows,
        vec![
            vec![t("Card"), m(12_500), m(0), m(12_500)],
            // 250.00 + 135.00 + 40.00 (till 2); the voided 40.00 is not counted.
            vec![t("Cash"), m(42_500), m(10_000), m(32_500)],
        ]
    );
    assert_eq!(
        payments.totals,
        Some(vec![t("Total"), m(55_000), m(10_000), m(45_000)])
    );
}

#[test]
fn rpt_01_voided_sales_are_listed_separately() {
    let conn = shop();
    let report = daily_summary(&conn, date("2026-03-05"), None, &shop_context()).unwrap();
    let voids = section(&report, "Voided sales");

    assert_eq!(
        voids.rows,
        vec![vec![t("T1-000004"), t("Rina"), t("Wrong item"), m(4_000)]]
    );
}

#[test]
fn rpt_01_one_till_only() {
    let conn = shop();
    let till_one = id("T1");
    let report =
        daily_summary(&conn, date("2026-03-05"), Some(&till_one), &shop_context()).unwrap();

    assert_eq!(report.params.filters[0].value, "Till 1");
    assert_eq!(row(&report, "Gross sales")[1], Cell::Int(3));
    assert_eq!(row(&report, "Total taken")[3], m(41_000));
    assert_eq!(row(&report, "Cancelled carts")[1], Cell::Int(1));
    let payments = section(&report, "Payments by method");
    assert_eq!(
        payments.totals,
        Some(vec![t("Total"), m(51_000), m(10_000), m(41_000)])
    );
}

#[test]
fn rpt_01_a_day_without_sales_shows_zeros() {
    let conn = shop();
    let report = daily_summary(&conn, date("2026-03-04"), None, &shop_context()).unwrap();

    assert_eq!(row(&report, "Gross sales")[1], Cell::Int(0));
    assert_eq!(row(&report, "Net sales")[3], m(0));
    // No sales, so no average.
    assert_eq!(row(&report, "Average sale")[3], E);
    assert_eq!(row(&report, "Items sold")[2], Cell::Qty(Qty(0)));
    assert!(section(&report, "Tax by rate").rows.is_empty());
}

#[test]
fn rpt_01_returns_count_on_their_own_day() {
    let conn = shop();
    // 03-06 has no sales, only the soap return of 35.00 by card.
    let report = daily_summary(&conn, date("2026-03-06"), None, &shop_context()).unwrap();

    assert_eq!(
        row(&report, "Returns"),
        &vec![t("Returns"), Cell::Int(1), E, m(3_500)]
    );
    assert_eq!(row(&report, "Net sales")[3], m(-3_500));
    assert_eq!(row(&report, "Tax")[3], m(-457));
    assert_eq!(row(&report, "Total taken")[3], m(-3_500));
}
