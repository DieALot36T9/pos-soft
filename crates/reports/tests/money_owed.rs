//! RPT-06 tax report (TAX-09), RPT-07 aging and RPT-08 discounts and overrides.

#![allow(clippy::unwrap_used)]

mod support;

use pos_core::{Bps, Cell};
use reports::{DateRange, discounts_overrides, payables_aging, receivables_aging, tax_report};
use support::{date, m, section, shop, shop_context, t};

const E: Cell = Cell::Empty;

fn both_days() -> DateRange {
    DateRange {
        from: date("2026-03-05"),
        to: date("2026-03-06"),
    }
}

fn pct(bps: u32) -> Cell {
    Cell::Percent(Bps(bps))
}

#[test]
fn tax_09_tax_report_by_rate_and_component() {
    let conn = shop();
    let report = tax_report(&conn, both_days()).unwrap();

    assert_eq!(report.title, "Tax report");
    assert_eq!(
        report.rows,
        vec![
            vec![t("Exempt 0%"), E, pct(0), m(18_497), m(0), m(0), m(0), m(0)],
            vec![
                t("Standard 15%"),
                E,
                pct(1_500),
                m(31_738),
                m(4_762),
                m(11_739),
                m(1_761),
                m(3_001)
            ],
            // Each refund is split with the parts of the sale line it came from.
            vec![
                t("Standard 15%"),
                t("CGST"),
                pct(750),
                E,
                m(2_380),
                E,
                m(880),
                m(1_500)
            ],
            vec![
                t("Standard 15%"),
                t("SGST"),
                pct(750),
                E,
                m(2_382),
                E,
                m(881),
                m(1_501)
            ],
        ]
    );
    assert_eq!(
        report.totals,
        Some(vec![
            t("Total"),
            E,
            E,
            m(50_235),
            m(4_762),
            m(11_739),
            m(1_761),
            m(3_001)
        ])
    );
}

#[test]
fn rpt_07_supplier_aging_pays_off_the_oldest_first() {
    let conn = shop();
    let report = payables_aging(&conn, date("2026-03-05")).unwrap();

    assert_eq!(report.title, "Supplier aging");
    assert_eq!(
        report.rows,
        vec![
            // 5,000 (44 days late) and 1,000 of the 3,000 are paid.
            vec![
                t("Fresh Dairy"),
                m(200_000),
                m(200_000),
                m(0),
                m(0),
                m(0),
                m(400_000)
            ],
            vec![t("Rice Mill"), m(0), m(0), m(0), m(70_000), m(0), m(70_000)],
            // Paid more than owed: the shop is owed 500, shown as a negative "current".
            vec![t("Soap Co"), m(-50_000), m(0), m(0), m(0), m(0), m(-50_000)],
        ]
    );
    assert_eq!(
        report.totals,
        Some(vec![
            t("Total"),
            m(150_000),
            m(200_000),
            m(0),
            m(70_000),
            m(0),
            m(420_000)
        ])
    );
}

#[test]
fn rpt_07_customer_aging() {
    let conn = shop();
    let report = receivables_aging(&conn, date("2026-03-05")).unwrap();

    assert_eq!(report.title, "Customer aging");
    // Zara paid everything, so she isn't listed.
    assert_eq!(
        report.rows,
        vec![
            vec![
                t("Nadia"),
                m(0),
                m(30_000),
                m(10_000),
                m(0),
                m(0),
                m(40_000)
            ],
            vec![t("Omar"), m(25_000), m(0), m(0), m(0), m(0), m(25_000)],
        ]
    );
}

#[test]
fn rpt_08_discounts_and_overrides_per_cashier() {
    let conn = shop();
    let report = discounts_overrides(&conn, both_days(), &shop_context()).unwrap();

    assert_eq!(report.title, "Discounts and overrides");
    assert_eq!(
        report.rows,
        vec![
            vec![
                t("Karim"),
                Cell::Int(0),
                m(0),
                Cell::Int(0),
                Cell::Int(0),
                Cell::Int(1),
                Cell::Int(0),
                Cell::Int(0)
            ],
            vec![
                t("Rina"),
                Cell::Int(1),
                m(1_000),
                Cell::Int(1),
                Cell::Int(1),
                Cell::Int(1),
                Cell::Int(1),
                Cell::Int(2)
            ],
        ]
    );
    let approvals = section(&report, "Overrides approved");
    assert_eq!(
        approvals.rows,
        vec![
            vec![
                t("Rina"),
                t("line.price_override"),
                t("Admin"),
                Cell::Int(1)
            ],
            vec![t("Rina"), t("sale.void"), t("Admin"), Cell::Int(1)],
        ]
    );
}
