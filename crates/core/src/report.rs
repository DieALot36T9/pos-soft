//! The shape of every report. `reports` makes it; the UI, `io` (Excel) and
//! `printing` (80 mm paper) show it. It lives here because modules can't depend
//! on each other. Details: `docs/05 Modules/Module - reports.md`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Bps, BusinessDate, Money, Qty};

/// A finished report: a title, columns, rows, optional totals and sub-sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReportTable {
    /// Plain English title. Example: "Daily summary".
    pub title: String,
    /// The date range and filters the report was run with.
    pub params: ReportParams,
    pub columns: Vec<Column>,
    /// One inner list per row, with one cell per column.
    pub rows: Vec<Vec<Cell>>,
    pub totals: Option<Vec<Cell>>,
    /// Smaller tables inside this one. Example: the payments part of a daily summary.
    pub sections: Vec<ReportTable>,
}

/// What a report was run for.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReportParams {
    pub from: Option<BusinessDate>,
    pub to: Option<BusinessDate>,
    pub filters: Vec<ReportFilter>,
}

/// One filter a report was run with. Example: name "Cashier", value "Rina".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReportFilter {
    pub name: String,
    pub value: String,
}

/// One column of a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Column {
    /// A stable key. Example: "net_sales".
    pub key: String,
    /// The heading people read. Example: "Net sales".
    pub title: String,
    pub kind: ColumnKind,
}

/// What kind of value a column holds, so each place can format and align it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ColumnKind {
    Text,
    Money,
    Qty,
    Int,
    Percent,
    Date,
}

/// One value in a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[ts(export)]
pub enum Cell {
    Empty,
    Text(String),
    Money(Money),
    Qty(Qty),
    Int(i64),
    Percent(Bps),
    Date(BusinessDate),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_are_sent_with_their_kind() {
        assert_eq!(
            serde_json::to_string(&Cell::Money(Money(1250))).unwrap(),
            r#"{"kind":"money","value":1250}"#
        );
        assert_eq!(
            serde_json::to_string(&Cell::Empty).unwrap(),
            r#"{"kind":"empty"}"#
        );
    }
}
