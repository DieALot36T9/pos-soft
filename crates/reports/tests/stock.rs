//! RPT-05 stock reports: value, low stock, near expiry and movement.

#![allow(clippy::unwrap_used)]

mod support;

use pos_core::{Cell, Money};
use reports::{DateRange, GroupBy, low_stock, near_expiry, stock_movement, stock_value};
use support::{date, m, q, shop, t};

const E: Cell = Cell::Empty;

#[test]
fn rpt_05_stock_value_by_product() {
    let conn = shop();
    let report = stock_value(&conn, GroupBy::Product).unwrap();

    assert_eq!(report.title, "Stock value by product");
    // On hand × cost; Oil has no stock so it isn't listed.
    assert_eq!(
        report.rows,
        vec![
            vec![t("Milk 1L"), q(20_000), m(8_000), m(160_000)],
            vec![t("Soap"), q(50_000), m(2_500), m(125_000)],
            vec![t("Rice"), q(5_500), m(4_000), m(22_000)],
        ]
    );
    assert_eq!(report.totals, Some(vec![t("Total"), E, E, m(307_000)]));
}

#[test]
fn rpt_05_stock_value_by_category_and_brand() {
    let conn = shop();
    let by_category = stock_value(&conn, GroupBy::Category).unwrap();
    assert_eq!(
        by_category.rows,
        vec![
            vec![t("Dairy"), Cell::Int(1), m(160_000)],
            vec![t("Household"), Cell::Int(1), m(125_000)],
            vec![t("Grocery"), Cell::Int(1), m(22_000)],
        ]
    );

    let by_brand = stock_value(&conn, GroupBy::Brand).unwrap();
    assert_eq!(
        by_brand.rows[2],
        vec![t("No brand"), Cell::Int(1), m(22_000)]
    );
    assert_eq!(by_brand.totals.unwrap()[2], Cell::Money(Money(307_000)));
}

#[test]
fn rpt_05_low_stock_with_supplier_and_suggested_order() {
    let conn = shop();
    let report = low_stock(&conn).unwrap();

    assert_eq!(
        report.rows,
        vec![
            // Below the level by 5: order the reorder quantity of 30.
            vec![
                t("Milk 1L"),
                t("P1"),
                q(20_000),
                q(25_000),
                q(30_000),
                t("Fresh Dairy")
            ],
            // Never stocked, but has a rule.
            vec![t("Oil 1L"), t("P4"), q(0), q(5_000), q(12_000), E],
            // No reorder quantity: suggest enough to reach the level again.
            vec![t("Rice"), t("P2"), q(5_500), q(10_000), q(4_500), E],
        ]
    );
}

#[test]
fn rpt_05_near_expiry_and_expired_batches() {
    let conn = shop();
    let report = near_expiry(&conn, date("2026-03-05"), 7).unwrap();

    assert_eq!(
        report.rows,
        vec![
            vec![
                t("Soap"),
                t("S-01"),
                Cell::Date(date("2026-03-01")),
                Cell::Int(-4),
                q(3_000),
                m(7_500),
                t("Expired"),
            ],
            vec![
                t("Milk 1L"),
                t("M-07"),
                Cell::Date(date("2026-03-07")),
                Cell::Int(2),
                q(12_000),
                m(96_000),
                t("Expires soon"),
            ],
        ]
    );
    assert_eq!(
        report.totals,
        Some(vec![t("Total"), E, E, E, E, m(103_500), E])
    );
}

#[test]
fn rpt_05_stock_movement_from_opening_to_closing() {
    let conn = shop();
    let range = DateRange {
        from: date("2026-03-04"),
        to: date("2026-03-05"),
    };
    let report = stock_movement(&conn, range).unwrap();

    assert_eq!(
        report.rows,
        vec![
            vec![
                t("Milk 1L"),
                q(15_000),
                q(10_000),
                q(-3_000),
                q(1_000),
                q(-3_000),
                q(20_000)
            ],
            vec![t("Rice"), q(9_500), q(0), q(-4_000), q(0), q(0), q(5_500)],
        ]
    );
}
