//! RPT-03 sales analysis and RPT-04 profit, over 03-05 and 03-06 of the fixture shop.

#![allow(clippy::unwrap_used)]

mod support;

use pos_core::{AppError, Bps, Cell, ColumnKind};
use reports::{DateRange, GroupBy, SalesDimension, SalesFilters, profit, sales_by};
use support::{date, id, m, q, row, shop, shop_context, t};

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
fn rpt_03_sales_by_category() {
    let conn = shop();
    let report = sales_by(
        &conn,
        SalesDimension::Category,
        both_days(),
        &SalesFilters::default(),
        &shop_context(),
    )
    .unwrap();

    assert_eq!(report.title, "Sales by category");
    let titles: Vec<&str> = report
        .columns
        .iter()
        .map(|column| column.title.as_str())
        .collect();
    assert_eq!(
        titles,
        [
            "Category",
            "Quantity",
            "Net sales",
            "Tax",
            "Discount",
            "Number of sales",
            "Share of total"
        ]
    );
    // Sorted by net sales, largest first. Returns are taken off.
    assert_eq!(
        report.rows,
        vec![
            vec![
                t("Grocery"),
                q(4_000),
                m(18_497),
                m(0),
                m(0),
                Cell::Int(2),
                pct(4_805)
            ],
            vec![
                t("Dairy"),
                q(2_000),
                m(16_521),
                m(2_479),
                m(1_000),
                Cell::Int(2),
                pct(4_292)
            ],
            vec![
                t("Household"),
                q(1_000),
                m(3_478),
                m(522),
                m(500),
                Cell::Int(2),
                pct(903)
            ],
        ]
    );
    assert_eq!(
        report.totals,
        Some(vec![
            t("Total"),
            q(7_000),
            m(38_496),
            m(3_001),
            m(1_500),
            Cell::Int(4),
            pct(10_000)
        ])
    );
}

#[test]
fn rpt_03_returns_count_on_their_own_business_date() {
    let conn = shop();
    let first_day = DateRange {
        from: date("2026-03-05"),
        to: date("2026-03-05"),
    };
    let report = sales_by(
        &conn,
        SalesDimension::Category,
        first_day,
        &SalesFilters::default(),
        &shop_context(),
    )
    .unwrap();

    // The soap sold on 03-05 and returned on 03-06 still counts on 03-05.
    assert_eq!(row(&report, "Household")[2], m(6_521));
}

#[test]
fn rpt_03_sales_by_product_and_brand() {
    let conn = shop();
    let filters = SalesFilters::default();
    let by_product = sales_by(
        &conn,
        SalesDimension::Product,
        both_days(),
        &filters,
        &shop_context(),
    )
    .unwrap();
    assert_eq!(row(&by_product, "Milk 1L")[2], m(16_521));

    let by_brand = sales_by(
        &conn,
        SalesDimension::Brand,
        both_days(),
        &filters,
        &shop_context(),
    )
    .unwrap();
    assert_eq!(row(&by_brand, "No brand")[2], m(18_497));
    assert_eq!(row(&by_brand, "Aarong")[2], m(16_521));
    assert_eq!(row(&by_brand, "Lux")[2], m(3_478));
}

#[test]
fn rpt_03_sales_by_cashier() {
    let conn = shop();
    let report = sales_by(
        &conn,
        SalesDimension::Cashier,
        both_days(),
        &SalesFilters::default(),
        &shop_context(),
    )
    .unwrap();

    // Rina: 467.57 sold − 86.96 − 30.43 returned. Karim: one soap.
    assert_eq!(row(&report, "Rina")[2], m(35_018));
    assert_eq!(row(&report, "Karim")[2], m(3_478));
}

#[test]
fn rpt_03_sales_by_hour_use_local_time() {
    let conn = shop();
    let report = sales_by(
        &conn,
        SalesDimension::Hour,
        both_days(),
        &SalesFilters::default(),
        &shop_context(),
    )
    .unwrap();

    let hours: Vec<&Cell> = report.rows.iter().map(|row| &row[0]).collect();
    assert_eq!(
        hours,
        [
            &t("10:00"),
            &t("11:00"),
            &t("12:00"),
            &t("15:00"),
            &t("16:00")
        ]
    );
    // 10:00 has the 03-05 sale (223.91) and the 03-06 return (30.43).
    assert_eq!(row(&report, "10:00")[2], m(19_348));
    // An hour with only a return is negative and has no share.
    assert_eq!(row(&report, "15:00")[2], m(-8_696));
    assert_eq!(row(&report, "15:00")[6], Cell::Empty);
}

#[test]
fn rpt_03_sales_by_weekday_and_date() {
    let conn = shop();
    let filters = SalesFilters::default();
    let by_weekday = sales_by(
        &conn,
        SalesDimension::Weekday,
        both_days(),
        &filters,
        &shop_context(),
    )
    .unwrap();
    assert_eq!(row(&by_weekday, "Thursday")[2], m(41_539));
    assert_eq!(row(&by_weekday, "Friday")[2], m(-3_043));

    let by_date = sales_by(
        &conn,
        SalesDimension::Date,
        both_days(),
        &filters,
        &shop_context(),
    )
    .unwrap();
    assert_eq!(by_date.columns[0].kind, ColumnKind::Date);
    assert_eq!(by_date.rows[0][0], Cell::Date(date("2026-03-05")));
    assert_eq!(by_date.rows[1][2], m(-3_043));
}

#[test]
fn rpt_03_filters_and_top_n() {
    let conn = shop();
    let karim_only = SalesFilters {
        cashier_id: Some(id("K")),
        ..SalesFilters::default()
    };
    let report = sales_by(
        &conn,
        SalesDimension::Category,
        both_days(),
        &karim_only,
        &shop_context(),
    )
    .unwrap();
    assert_eq!(report.rows.len(), 1);
    assert_eq!(row(&report, "Household")[2], m(3_478));
    assert_eq!(report.params.filters[0].value, "Karim");

    let dairy_only = SalesFilters {
        category_id: Some(id("CD")),
        ..SalesFilters::default()
    };
    let report = sales_by(
        &conn,
        SalesDimension::Product,
        both_days(),
        &dairy_only,
        &shop_context(),
    )
    .unwrap();
    assert_eq!(report.rows.len(), 1);
    assert_eq!(row(&report, "Milk 1L")[2], m(16_521));

    let top_one = SalesFilters {
        top: Some(1),
        ..SalesFilters::default()
    };
    let report = sales_by(
        &conn,
        SalesDimension::Category,
        both_days(),
        &top_one,
        &shop_context(),
    )
    .unwrap();
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0][0], t("Grocery"));
    // The totals still cover every category.
    assert_eq!(report.totals.unwrap()[2], m(38_496));
}

#[test]
fn rpt_03_a_backwards_date_range_is_refused() {
    let conn = shop();
    let backwards = DateRange {
        from: date("2026-03-06"),
        to: date("2026-03-05"),
    };
    let error = sales_by(
        &conn,
        SalesDimension::Date,
        backwards,
        &SalesFilters::default(),
        &shop_context(),
    )
    .unwrap_err();

    assert_eq!(
        AppError::from(error),
        AppError::validation("date_range_backwards")
    );
}

#[test]
fn rpt_04_profit_uses_the_cost_saved_on_each_line() {
    let conn = shop();
    let report = profit(&conn, GroupBy::Category, both_days()).unwrap();

    assert_eq!(report.title, "Profit by category");
    assert_eq!(
        report.rows,
        vec![
            vec![t("Grocery"), m(18_497), m(16_000), m(2_497), pct(1_350)],
            // Milk: 160.00 + 80.00 cost, minus the 80.00 of the returned bottle.
            vec![t("Dairy"), m(16_521), m(16_000), m(521), pct(315)],
            vec![t("Household"), m(3_478), m(2_500), m(978), pct(2_812)],
        ]
    );
    assert_eq!(
        report.totals,
        Some(vec![t("Total"), m(38_496), m(34_500), m(3_996), pct(1_038)])
    );
}

#[test]
fn rpt_04_profit_by_product_and_brand() {
    let conn = shop();
    let by_product = profit(&conn, GroupBy::Product, both_days()).unwrap();
    assert_eq!(row(&by_product, "Soap")[3], m(978));

    let by_brand = profit(&conn, GroupBy::Brand, both_days()).unwrap();
    assert_eq!(row(&by_brand, "Aarong")[3], m(521));
}
