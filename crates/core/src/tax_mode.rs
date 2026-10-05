//! Tax mode: whether shelf prices include tax. One shop-wide choice (ADR-008),
//! shared by `settings` (where it is stored), `tax` (which calculates with it)
//! and `sales` (which records it on each sale).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Whether shelf prices already include tax (rules TAX-04, SET-07). Chosen once for the whole shop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TaxMode {
    /// The shelf price is what the customer pays. Tax is taken out of it.
    /// Example: 115.00 already contains 15.00 VAT.
    Inclusive,
    /// Tax is added on top of the shelf price.
    Exclusive,
}

impl TaxMode {
    /// The text saved in the database and sent to the UI: "inclusive" or "exclusive".
    pub fn as_str(self) -> &'static str {
        match self {
            TaxMode::Inclusive => "inclusive",
            TaxMode::Exclusive => "exclusive",
        }
    }

    /// Reads the text from `as_str` back. Any other text gives `None`.
    pub fn parse(text: &str) -> Option<TaxMode> {
        match text {
            "inclusive" => Some(TaxMode::Inclusive),
            "exclusive" => Some(TaxMode::Exclusive),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reads_back_what_as_str_wrote() {
        for mode in [TaxMode::Inclusive, TaxMode::Exclusive] {
            assert_eq!(TaxMode::parse(mode.as_str()), Some(mode));
        }
    }

    #[test]
    fn parse_refuses_other_text() {
        assert_eq!(TaxMode::parse("Inclusive"), None);
        assert_eq!(TaxMode::parse(""), None);
    }

    #[test]
    fn the_ui_gets_the_same_text_as_the_database() {
        for mode in [TaxMode::Inclusive, TaxMode::Exclusive] {
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(json, format!("\"{}\"", mode.as_str()));
        }
    }
}
