//! The plain data shapes that go in and out of this crate.

use chrono::{NaiveDate, NaiveDateTime};
use pos_core::{Bps, Money, Qty};

/// A sheet of raw text cells, as read from a file.
/// `rows[0]` is line 2 of the file (line 1 holds the headers), so a row's
/// number in the file is its index + 2. Every row has at least as many cells
/// as there are headers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// One sheet to write into an .xlsx file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XlsxSheet {
    /// The tab name. Characters Excel refuses are replaced and it is cut to 31 characters.
    pub name: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<XlsxCell>>,
}

/// One typed cell for an .xlsx file. Numbers are written as real numbers, so
/// Excel can add them up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XlsxCell {
    Empty,
    /// Always stored as text, never as a formula, even if it starts with "=".
    Text(String),
    Int(i64),
    /// Written as a number with the currency's decimals. Example: Money(1250), 2 → 12.50.
    Money {
        amount: Money,
        minor_digits: u8,
    },
    /// Written as a number with 3 decimals. Example: Qty(1234) → 1.234.
    Qty(Qty),
    /// Written as a percentage. Example: Bps(1250) → 12.50%.
    Percent(Bps),
    Date(NaiveDate),
}

/// The lines at the top of an exported report (rule RPT-09).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportHeader {
    pub shop_name: String,
    /// When the report was made, in the shop's local time.
    pub generated_at: NaiveDateTime,
    /// How many decimals the shop's currency has. Example: 2 for taka.
    pub money_minor_digits: u8,
    pub labels: ExportLabels,
}

/// The words written into an exported report. They come from the caller so
/// other languages can be added later (ADR-022).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportLabels {
    /// Example: "Period", as in "Period: 2026-03-01 to 2026-03-07".
    pub period: String,
    /// Example: "to".
    pub period_to: String,
    /// Example: "Generated", as in "Generated: 2026-03-08 09:15".
    pub generated: String,
    /// Written in the first cell of the totals row. Example: "Total".
    pub total: String,
}

impl Default for ExportLabels {
    /// The English words.
    fn default() -> ExportLabels {
        ExportLabels {
            period: "Period".to_string(),
            period_to: "to".to_string(),
            generated: "Generated".to_string(),
            total: "Total".to_string(),
        }
    }
}
