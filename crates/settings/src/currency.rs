//! Money settings: currency format, cash rounding and cash denominations
//! (rules SET-05, SET-06 and SET-16). The currency list comes from ISO 4217
//! through the `iso_currency` crate (Reuse Map).

use iso_currency::{Currency, IntoEnumIterator};
use pos_core::{
    AppError, CurrencyFormat, Grouping, Money, RoundingAppliesTo, RoundingMode, RoundingRule,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Setting;
use crate::setting::invalid;

impl Setting for CurrencyFormat {
    const KEY: &'static str = "currency";

    /// Bangladeshi taka, the first shops' currency: "৳12,34,567.50".
    fn default_value() -> CurrencyFormat {
        CurrencyFormat {
            code: "BDT".to_string(),
            symbol: "৳".to_string(),
            symbol_before: true,
            minor_digits: 2,
            decimal_sep: ".".to_string(),
            group_sep: ",".to_string(),
            grouping: Grouping::Indian,
        }
    }

    // Locking `minor_digits` after the first sale is checked by `app` (rule SET-05).
    fn validate(&self) -> Result<(), AppError> {
        if Currency::from_code(&self.code).is_none() {
            return Err(invalid("code", "currency_code_unknown"));
        }
        if self.minor_digits > 3 {
            return Err(invalid("minor_digits", "minor_digits_out_of_range"));
        }
        if self.decimal_sep.is_empty() {
            return Err(invalid("decimal_sep", "decimal_sep_missing"));
        }
        if !is_plain_separator(&self.decimal_sep) {
            return Err(invalid("decimal_sep", "separator_invalid"));
        }
        if !is_plain_separator(&self.group_sep) {
            return Err(invalid("group_sep", "separator_invalid"));
        }
        if self.group_sep == self.decimal_sep {
            return Err(invalid("group_sep", "separators_same"));
        }
        // `num-format` can't group with a separator longer than 8 bytes (Module - core).
        if self.group_sep.len() > 8 {
            return Err(invalid("group_sep", "group_sep_too_long"));
        }
        Ok(())
    }
}

/// A separator must not contain digits or a minus sign, or amounts would be misread.
fn is_plain_separator(separator: &str) -> bool {
    !separator
        .chars()
        .any(|character| character.is_ascii_digit() || character == '-')
}

impl Setting for RoundingRule {
    const KEY: &'static str = "rounding";

    /// Cash rounding is off until the shop switches it on.
    fn default_value() -> RoundingRule {
        RoundingRule {
            increment_minor: Money::zero(),
            mode: RoundingMode::Nearest,
            applies_to: RoundingAppliesTo::CashOnly,
        }
    }

    fn validate(&self) -> Result<(), AppError> {
        if self.increment_minor.0 < 0 {
            return Err(invalid("increment_minor", "rounding_step_negative"));
        }
        Ok(())
    }
}

/// The notes and coins counted at shift close, largest first (rule SET-16).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(transparent)]
#[ts(export)]
pub struct Denominations(pub Vec<Money>);

impl Setting for Denominations {
    const KEY: &'static str = "denominations";

    /// Empty: the wizard fills it with `suggested_denominations` for the chosen currency.
    fn default_value() -> Denominations {
        Denominations::default()
    }

    fn validate(&self) -> Result<(), AppError> {
        for (position, value) in self.0.iter().enumerate() {
            if value.0 <= 0 {
                return Err(invalid("denominations", "denomination_not_positive"));
            }
            if self.0[..position].contains(value) {
                return Err(invalid("denominations", "denomination_repeated"));
            }
        }
        Ok(())
    }
}

/// One currency in the setup wizard's list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CurrencyChoice {
    /// ISO 4217 code. Example: "BDT".
    pub code: String,
    /// English name. Example: "Bangladeshi taka".
    pub name: String,
    /// Digits after the decimal point. Example: 2.
    pub minor_digits: u8,
    /// Example: "৳".
    pub symbol: String,
}

/// Every currency in use today with 0 to 3 decimals, sorted by code. Funds, metals
/// and replaced currencies are left out.
pub fn currency_choices() -> Vec<CurrencyChoice> {
    let mut choices = Vec::new();
    for currency in Currency::iter() {
        if !currency.flags().is_empty() {
            continue;
        }
        if let Some(minor_digits) = usable_digits(currency) {
            choices.push(CurrencyChoice {
                code: currency.code().to_string(),
                name: currency.name().to_string(),
                minor_digits,
                symbol: symbol_of(currency),
            });
        }
    }
    choices.sort_by(|left, right| left.code.cmp(&right.code));
    choices
}

/// A ready-made format for a currency code, which the wizard shows for the shop to adjust.
/// Example: "INR" → "₹", 2 decimals, Indian grouping.
pub fn currency_format_for(code: &str) -> Result<CurrencyFormat, AppError> {
    let currency =
        Currency::from_code(code).ok_or_else(|| invalid("code", "currency_code_unknown"))?;
    let minor_digits =
        usable_digits(currency).ok_or_else(|| invalid("code", "currency_decimals_unsupported"))?;
    // These currencies are usually written with Indian digit grouping: 12,34,567.
    let grouping = if ["BDT", "INR", "NPR"].contains(&code) {
        Grouping::Indian
    } else {
        Grouping::Western
    };
    Ok(CurrencyFormat {
        code: currency.code().to_string(),
        symbol: symbol_of(currency),
        symbol_before: true,
        minor_digits,
        decimal_sep: ".".to_string(),
        group_sep: ",".to_string(),
        grouping,
    })
}

/// The ISO number of decimals, if it is one the app supports (0 to 3).
fn usable_digits(currency: Currency) -> Option<u8> {
    let digits = currency.exponent()?;
    u8::try_from(digits).ok().filter(|digits| *digits <= 3)
}

/// The currency's symbol. ISO gives "¤" (any currency) when there is none, so the code is used.
fn symbol_of(currency: Currency) -> String {
    let symbol = currency.symbol().symbol;
    if symbol == "¤" {
        currency.code().to_string()
    } else {
        symbol
    }
}

/// Notes and coins in common use, in hundredths of one unit, largest first.
/// Example: 50_000 is a 500 note. ISO 4217 has no such list, so it is kept here.
const KNOWN_DENOMINATIONS: &[(&str, &[i64])] = &[
    (
        "BDT",
        &[
            100_000, 50_000, 20_000, 10_000, 5_000, 2_000, 1_000, 500, 200, 100,
        ],
    ),
    (
        "INR",
        &[50_000, 20_000, 10_000, 5_000, 2_000, 1_000, 500, 200, 100],
    ),
    (
        "USD",
        &[10_000, 5_000, 2_000, 1_000, 500, 200, 100, 25, 10, 5, 1],
    ),
    (
        "EUR",
        &[
            20_000, 10_000, 5_000, 2_000, 1_000, 500, 200, 100, 50, 20, 10, 5, 2, 1,
        ],
    ),
    (
        "GBP",
        &[5_000, 2_000, 1_000, 500, 200, 100, 50, 20, 10, 5, 2, 1],
    ),
];

/// The usual notes and coins for a currency, in the shop's minor units (rule SET-16).
/// Coins smaller than the shop's smallest unit are left out. Unknown currencies get none.
pub fn suggested_denominations(code: &str, minor_digits: u8) -> Denominations {
    let Some((_, hundredths)) = KNOWN_DENOMINATIONS.iter().find(|(known, _)| *known == code) else {
        return Denominations::default();
    };
    let mut values = Vec::new();
    for value in hundredths.iter() {
        let converted = match minor_digits {
            0 if value % 100 == 0 => Some(value / 100),
            1 if value % 10 == 0 => Some(value / 10),
            2 => Some(*value),
            3 => Some(value * 10),
            _ => None,
        };
        if let Some(minor) = converted {
            values.push(Money(minor));
        }
    }
    Denominations(values)
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

    fn currency_with(change: impl Fn(&mut CurrencyFormat)) -> CurrencyFormat {
        let mut format = CurrencyFormat::default_value();
        change(&mut format);
        format
    }

    #[test]
    fn set_05_default_currency_is_valid() {
        assert!(CurrencyFormat::default_value().validate().is_ok());
    }

    #[test]
    fn set_05_currency_rules() {
        let cases = [
            (
                currency_with(|f| f.code = "XYZ".to_string()),
                "currency_code_unknown",
            ),
            (
                currency_with(|f| f.minor_digits = 4),
                "minor_digits_out_of_range",
            ),
            (
                currency_with(|f| f.decimal_sep = String::new()),
                "decimal_sep_missing",
            ),
            (
                currency_with(|f| f.decimal_sep = "5".to_string()),
                "separator_invalid",
            ),
            (
                currency_with(|f| f.group_sep = "-".to_string()),
                "separator_invalid",
            ),
            (
                currency_with(|f| f.group_sep = ".".to_string()),
                "separators_same",
            ),
            (
                currency_with(|f| f.group_sep = "'''''''''".to_string()),
                "group_sep_too_long",
            ),
        ];
        for (format, expected) in cases {
            assert_eq!(code_of(format.validate()), expected);
        }
        // No grouping at all is allowed.
        assert!(
            currency_with(|f| f.group_sep = String::new())
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn set_06_rounding_step_cannot_be_negative() {
        let mut rule = RoundingRule::default_value();
        assert!(rule.validate().is_ok());
        rule.increment_minor = Money(-1);
        assert_eq!(code_of(rule.validate()), "rounding_step_negative");
    }

    #[test]
    fn set_16_denominations_must_be_positive_and_different() {
        assert!(
            Denominations(vec![Money(500), Money(100)])
                .validate()
                .is_ok()
        );
        assert_eq!(
            code_of(Denominations(vec![Money(0)]).validate()),
            "denomination_not_positive"
        );
        assert_eq!(
            code_of(Denominations(vec![Money(100), Money(100)]).validate()),
            "denomination_repeated"
        );
    }

    #[test]
    fn set_05_currency_without_a_symbol_shows_its_code() {
        let gold = Currency::XAU;
        assert_eq!(symbol_of(gold), "XAU");
        assert_eq!(
            currency_format_for("BDT").map(|f| f.symbol),
            Ok("৳".to_string())
        );
    }
}
