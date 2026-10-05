//! Sample Z report, label and shelf tag.

use pos_core::{
    Bps, BusinessDate, Cell, Column, ColumnKind, Money, ReportFilter, ReportParams, ReportTable,
};
use printing::{
    LabelData, LabelSize, LabelTemplate, PromotionOnTag, ShelfTagData, TagTemplate, UnitPrice,
};

use super::{fake_barcode, taka};

fn text_column(key: &str, title: &str) -> Column {
    Column {
        key: key.into(),
        title: title.into(),
        kind: ColumnKind::Text,
    }
}

fn money_column(key: &str, title: &str) -> Column {
    Column {
        key: key.into(),
        title: title.into(),
        kind: ColumnKind::Money,
    }
}

fn filter(name: &str, value: &str) -> ReportFilter {
    ReportFilter {
        name: name.into(),
        value: value.into(),
    }
}

/// A smaller table inside a report.
fn section(
    title: &str,
    columns: Vec<Column>,
    rows: Vec<Vec<Cell>>,
    totals: Option<Vec<Cell>>,
) -> ReportTable {
    ReportTable {
        title: title.into(),
        params: ReportParams::default(),
        columns,
        rows,
        totals,
        sections: vec![],
    }
}

fn label_and_money(label: &str, minor_units: i64) -> Vec<Cell> {
    vec![Cell::Text(label.into()), Cell::Money(Money(minor_units))]
}

/// A Z report with the parts listed in rule SHF-08. `reports` will build the real one.
pub fn sample_z_report() -> ReportTable {
    let date = BusinessDate::parse("2026-10-04").expect("a valid date");
    let counts = section(
        "Counts",
        vec![
            text_column("item", "Item"),
            Column {
                key: "count".into(),
                title: "Count".into(),
                kind: ColumnKind::Int,
            },
        ],
        vec![
            vec![Cell::Text("Sales".into()), Cell::Int(212)],
            vec![Cell::Text("Returns".into()), Cell::Int(3)],
            vec![Cell::Text("Voids".into()), Cell::Int(1)],
            vec![Cell::Text("Cancelled carts".into()), Cell::Int(4)],
        ],
        None,
    );
    let tax = section(
        "Tax by rate",
        vec![
            text_column("rate", "Rate"),
            Column {
                key: "percent".into(),
                title: "%".into(),
                kind: ColumnKind::Percent,
            },
            money_column("tax", "Tax"),
        ],
        vec![vec![
            Cell::Text("VAT".into()),
            Cell::Percent(Bps(1500)),
            Cell::Money(Money(705757)),
        ]],
        None,
    );
    let cash = section(
        "Cash",
        vec![
            text_column("item", "Item"),
            money_column("amount", "Amount"),
        ],
        vec![
            label_and_money("Float", 500000),
            label_and_money("Cash sales", 3120000),
            label_and_money("Cash out", -200000),
            label_and_money("Expected", 3420000),
            label_and_money("Counted", 3419000),
        ],
        Some(label_and_money("Difference", -1000)),
    );
    ReportTable {
        title: "Z report".into(),
        params: ReportParams {
            from: Some(date),
            to: Some(date),
            filters: vec![
                filter("Till", "T1"),
                filter("Cashier", "Rina"),
                filter("Opened", "08:01"),
                filter("Closed", "22:47"),
            ],
        },
        columns: vec![
            text_column("item", "Item"),
            money_column("amount", "Amount"),
        ],
        rows: vec![
            label_and_money("Gross sales", 5432100),
            label_and_money("Discounts", -12300),
            label_and_money("Returns", -9000),
        ],
        totals: Some(label_and_money("Net sales", 5410800)),
        sections: vec![counts, tax, cash],
    }
}

/// A weighed-item label (rule CAT-12).
pub fn sample_label() -> LabelData {
    LabelData {
        name: "Beef, boneless".into(),
        price: Money(92550),
        unit_price: Some(UnitPrice {
            price: Money(75000),
            unit: "kg".into(),
        }),
        barcode: Some(fake_barcode(220, 60)),
        barcode_text: "2100123012343".into(),
        packed_on: Some("2026-10-04".into()),
        expires_on: Some("2026-10-07".into()),
    }
}

/// A label template with every field turned on.
pub fn label_template(width_mm: u32, height_mm: u32) -> LabelTemplate {
    LabelTemplate {
        size: LabelSize {
            width_mm,
            height_mm,
            gap_mm: 2,
        },
        currency: taka(),
        show_name: true,
        show_price: true,
        show_unit_price: true,
        show_barcode: true,
        show_packed_on: true,
        show_expires_on: true,
    }
}

/// A shelf tag with an active promotion (rule CAT-13).
pub fn sample_shelf_tag() -> ShelfTagData {
    ShelfTagData {
        name: "Teer Soybean Oil 5L".into(),
        price: Money(92000),
        unit_price: Some(UnitPrice {
            price: Money(17000),
            unit: "L".into(),
        }),
        barcode: Some(fake_barcode(220, 50)),
        barcode_text: "8941100500019".into(),
        promotion: Some(PromotionOnTag {
            price: Money(85000),
            from: "2026-10-01".into(),
            to: "2026-10-15".into(),
        }),
    }
}

/// A shelf tag template with every field turned on.
pub fn tag_template(width_mm: u32, height_mm: u32) -> TagTemplate {
    TagTemplate {
        size: LabelSize {
            width_mm,
            height_mm,
            gap_mm: 2,
        },
        currency: taka(),
        show_unit_price: true,
        show_barcode: true,
    }
}
