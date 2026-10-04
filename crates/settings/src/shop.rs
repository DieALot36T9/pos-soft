//! Settings about the shop itself, its tax, this till and the receipt.

use pos_core::{AppError, Id};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Setting;
use crate::setting::{invalid, is_filled};

/// The language for screens and receipts (rule SET-03). Only English in v1.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct LanguageSettings {
    /// Language code. Example: "en".
    pub language: String,
}

impl Default for LanguageSettings {
    fn default() -> LanguageSettings {
        LanguageSettings {
            language: "en".to_string(),
        }
    }
}

impl Setting for LanguageSettings {
    const KEY: &'static str = "language";

    fn default_value() -> LanguageSettings {
        LanguageSettings::default()
    }

    fn validate(&self) -> Result<(), AppError> {
        // ADR-022: only English is shipped in v1.0.
        if self.language != "en" {
            return Err(invalid("language", "language_not_supported"));
        }
        Ok(())
    }
}

/// The shop's name and contact details, printed on receipts (rule SET-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct ShopProfile {
    /// Required. Example: "Rahim Store".
    pub name: String,
    pub address_lines: Vec<String>,
    pub phone: String,
    /// Empty, or an address like "shop@example.com".
    pub email: String,
    /// What the tax number is called. Example: "BIN", "VAT No", "GSTIN".
    pub tax_reg_label: String,
    pub tax_reg_no: String,
    /// Where the logo file is kept. `None` means no logo.
    pub logo_path: Option<String>,
    pub receipt_footer: Vec<String>,
}

impl Default for ShopProfile {
    fn default() -> ShopProfile {
        ShopProfile {
            name: String::new(),
            address_lines: Vec::new(),
            phone: String::new(),
            email: String::new(),
            tax_reg_label: "BIN".to_string(),
            tax_reg_no: String::new(),
            logo_path: None,
            receipt_footer: Vec::new(),
        }
    }
}

impl Setting for ShopProfile {
    const KEY: &'static str = "shop_profile";

    fn default_value() -> ShopProfile {
        ShopProfile::default()
    }

    fn validate(&self) -> Result<(), AppError> {
        if !is_filled(&self.name) {
            return Err(invalid("name", "shop_name_missing"));
        }
        // A light check only: a typing slip, not full address rules.
        let email = self.email.trim();
        if !email.is_empty() && !email.contains('@') {
            return Err(invalid("email", "email_invalid"));
        }
        Ok(())
    }
}

/// Whether shelf prices already contain tax (rule SET-07, ADR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TaxMode {
    /// Prices include tax. Example: 115.00 already contains 15.00 VAT.
    Inclusive,
    /// Tax is added on top of the price.
    Exclusive,
}

/// The tax name and price mode. The rates themselves live in the `tax` module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct TaxSettings {
    /// Example: "VAT", "GST", "Sales Tax".
    pub label: String,
    pub mode: TaxMode,
    /// The rate new products get. A `tax` rate id; `app` checks that it exists.
    pub default_rate_id: Option<Id>,
}

impl Default for TaxSettings {
    fn default() -> TaxSettings {
        TaxSettings {
            label: "VAT".to_string(),
            mode: TaxMode::Inclusive,
            default_rate_id: None,
        }
    }
}

impl Setting for TaxSettings {
    const KEY: &'static str = "tax";

    fn default_value() -> TaxSettings {
        TaxSettings::default()
    }

    fn validate(&self) -> Result<(), AppError> {
        if !is_filled(&self.label) {
            return Err(invalid("label", "tax_label_missing"));
        }
        Ok(())
    }
}

/// Who this till is and when its business day starts (rule SET-09).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct TillSettings {
    /// Set by `app` at the till step. `None` until then, so the default never changes.
    pub till_id: Option<Id>,
    /// 1 to 6 letters or digits, used in invoice numbers. Example: "T1".
    pub till_code: String,
    /// 0 to 23. With 4, a sale at 01:30 counts for the day before.
    pub business_day_cutoff_hour: u8,
}

impl Default for TillSettings {
    fn default() -> TillSettings {
        TillSettings {
            till_id: None,
            till_code: "T1".to_string(),
            business_day_cutoff_hour: 4,
        }
    }
}

impl Setting for TillSettings {
    const KEY: &'static str = "till";

    fn default_value() -> TillSettings {
        TillSettings::default()
    }

    fn validate(&self) -> Result<(), AppError> {
        let code = &self.till_code;
        let right_length = (1..=6).contains(&code.len());
        if !right_length || !code.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(invalid("till_code", "till_code_invalid"));
        }
        if self.business_day_cutoff_hour > 23 {
            return Err(invalid(
                "business_day_cutoff_hour",
                "cutoff_hour_out_of_range",
            ));
        }
        Ok(())
    }
}

/// Receipt paper width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PaperWidth {
    Mm58,
    Mm80,
}

/// What the printed receipt shows (rule SET-14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct ReceiptSettings {
    pub paper_width: PaperWidth,
    pub show_logo: bool,
    pub show_tax_breakdown: bool,
    pub show_tax_reg_no: bool,
    /// The invoice barcode, scanned to find the sale for a return.
    pub show_invoice_barcode: bool,
    /// The customer's points and credit balance.
    pub show_customer_balance: bool,
    /// How many copies to print, 1 to 3.
    pub copies: u8,
    pub header_lines: Vec<String>,
    pub footer_lines: Vec<String>,
}

impl Default for ReceiptSettings {
    fn default() -> ReceiptSettings {
        ReceiptSettings {
            paper_width: PaperWidth::Mm80,
            show_logo: true,
            show_tax_breakdown: true,
            show_tax_reg_no: true,
            show_invoice_barcode: true,
            show_customer_balance: true,
            copies: 1,
            header_lines: Vec::new(),
            footer_lines: Vec::new(),
        }
    }
}

impl Setting for ReceiptSettings {
    const KEY: &'static str = "receipt";

    fn default_value() -> ReceiptSettings {
        ReceiptSettings::default()
    }

    fn validate(&self) -> Result<(), AppError> {
        if !(1..=3).contains(&self.copies) {
            return Err(invalid("copies", "receipt_copies_out_of_range"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code_of(result: Result<(), AppError>) -> String {
        match result {
            Err(AppError::Validation { code, .. }) => code,
            other => format!("{other:?}"),
        }
    }

    #[test]
    fn set_03_only_english_is_accepted() {
        assert!(LanguageSettings::default().validate().is_ok());
        let bangla = LanguageSettings {
            language: "bn".to_string(),
        };
        assert_eq!(code_of(bangla.validate()), "language_not_supported");
    }

    #[test]
    fn set_04_shop_name_is_required_and_email_is_checked() {
        assert_eq!(
            code_of(ShopProfile::default().validate()),
            "shop_name_missing"
        );
        let mut shop = ShopProfile {
            name: "Rahim Store".to_string(),
            ..ShopProfile::default()
        };
        assert!(shop.validate().is_ok());
        shop.email = "shop.example.com".to_string();
        assert_eq!(code_of(shop.validate()), "email_invalid");
    }

    #[test]
    fn set_07_tax_label_is_required() {
        let tax = TaxSettings {
            label: " ".to_string(),
            ..TaxSettings::default()
        };
        assert_eq!(code_of(tax.validate()), "tax_label_missing");
    }

    #[test]
    fn set_09_till_code_is_one_to_six_letters_or_digits() {
        for good in ["T1", "A", "TILL01"] {
            let till = TillSettings {
                till_code: good.to_string(),
                ..TillSettings::default()
            };
            assert!(till.validate().is_ok(), "{good}");
        }
        for bad in ["", "TILL001", "T-1", "T 1"] {
            let till = TillSettings {
                till_code: bad.to_string(),
                ..TillSettings::default()
            };
            assert_eq!(code_of(till.validate()), "till_code_invalid", "{bad}");
        }
    }

    #[test]
    fn set_09_cutoff_hour_is_0_to_23() {
        let mut till = TillSettings {
            business_day_cutoff_hour: 23,
            ..TillSettings::default()
        };
        assert!(till.validate().is_ok());
        till.business_day_cutoff_hour = 24;
        assert_eq!(code_of(till.validate()), "cutoff_hour_out_of_range");
    }

    #[test]
    fn set_14_receipt_copies_are_1_to_3() {
        for (copies, ok) in [(0, false), (1, true), (3, true), (4, false)] {
            let receipt = ReceiptSettings {
                copies,
                ..ReceiptSettings::default()
            };
            assert_eq!(receipt.validate().is_ok(), ok, "{copies}");
        }
    }
}
