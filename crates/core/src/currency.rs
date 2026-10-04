//! Showing money as text and reading typed amounts (rule SET-05).
//! The UI's `formatMoney` must give the same text: both are tested against
//! `crates/core/tests/money_vectors.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{AppError, Money};

/// How digits are grouped in large numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Grouping {
    /// Groups of three: 1,234,567
    Western,
    /// Three, then groups of two: 12,34,567
    Indian,
}

/// The shop's currency and number format (rule SET-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CurrencyFormat {
    /// ISO 4217 code. Example: "BDT"
    pub code: String,
    /// Symbol to show. Example: "৳". May be empty.
    pub symbol: String,
    /// true: "৳12.50". false: "12.50 ৳" (with a space).
    pub symbol_before: bool,
    /// Digits after the decimal point, 0 to 3. Example: 2
    pub minor_digits: u8,
    /// Example: "."
    pub decimal_sep: String,
    /// Example: ",". Empty means no grouping.
    pub group_sep: String,
    /// Western (1,234,567) or Indian (12,34,567).
    pub grouping: Grouping,
}

/// Shows an amount the way the shop writes money. Example: Money(123456750) → "৳12,34,567.50".
pub fn format_money(amount: Money, format: &CurrencyFormat) -> String {
    // 18 digits is far more than any currency uses; the cap keeps the maths safe.
    let digits = u32::from(format.minor_digits.min(18));
    let size = amount.0.unsigned_abs();
    let divisor = 10u64.pow(digits);
    let whole = size / divisor;
    let fraction = size % divisor;

    let mut number = group_digits(&whole.to_string(), &format.group_sep, format.grouping);
    if digits > 0 {
        number.push_str(&format.decimal_sep);
        number.push_str(&format!("{fraction:0width$}", width = digits as usize));
    }

    let with_symbol = if format.symbol.is_empty() {
        number
    } else if format.symbol_before {
        format!("{}{number}", format.symbol)
    } else {
        format!("{number} {}", format.symbol)
    };

    if amount.0 < 0 {
        format!("-{with_symbol}")
    } else {
        with_symbol
    }
}

/// Puts group separators into a string of digits, working from the right.
fn group_digits(digits: &str, separator: &str, grouping: Grouping) -> String {
    if separator.is_empty() {
        return digits.to_string();
    }
    let characters: Vec<char> = digits.chars().collect();
    let mut groups: Vec<String> = Vec::new();
    let mut end = characters.len();
    let mut group_size = 3;
    while end > 0 {
        let start = end.saturating_sub(group_size);
        groups.push(characters[start..end].iter().collect());
        end = start;
        // After the first group of three, Indian grouping uses groups of two.
        if grouping == Grouping::Indian {
            group_size = 2;
        }
    }
    groups.reverse();
    groups.join(separator)
}

/// Why a typed amount could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseMoneyError {
    /// Nothing was typed.
    #[error("no amount was typed")]
    Empty,
    /// The text is not a plain number like 123.45.
    #[error("the amount is not a number")]
    NotANumber,
    /// More decimals than the currency has. Example: 1.234 when the currency has 2.
    #[error("the amount has more than {max_decimals} decimals")]
    TooManyDecimals { max_decimals: u8 },
    /// The amount is too large to store.
    #[error("the amount is too large")]
    TooLarge,
}

impl ParseMoneyError {
    /// The stable code the UI turns into a message.
    pub fn code(&self) -> &'static str {
        match self {
            ParseMoneyError::Empty => "money_empty",
            ParseMoneyError::NotANumber => "money_not_a_number",
            ParseMoneyError::TooManyDecimals { .. } => "money_too_many_decimals",
            ParseMoneyError::TooLarge => "money_too_large",
        }
    }
}

impl From<ParseMoneyError> for AppError {
    fn from(error: ParseMoneyError) -> AppError {
        let mut params = BTreeMap::new();
        if let ParseMoneyError::TooManyDecimals { max_decimals } = &error {
            params.insert("max_decimals".to_string(), max_decimals.to_string());
        }
        AppError::Validation {
            field: None,
            code: error.code().to_string(),
            params,
        }
    }
}

/// Reads a typed amount such as "123.45" or "-7.5" into minor units.
/// It accepts only digits, one optional minus sign and one optional "." with
/// at most `minor_digits` decimals. Example: ("123.4", 2) → Money(12340).
pub fn parse_money(text: &str, minor_digits: u8) -> Result<Money, ParseMoneyError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(ParseMoneyError::Empty);
    }
    let (is_negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, fraction) = match unsigned.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (unsigned, ""),
    };
    let has_point = unsigned.contains('.');
    if !is_all_digits(whole) || (has_point && !is_all_digits(fraction)) {
        return Err(ParseMoneyError::NotANumber);
    }
    if fraction.len() > usize::from(minor_digits) {
        return Err(ParseMoneyError::TooManyDecimals {
            max_decimals: minor_digits,
        });
    }
    // An i64 has 19 digits, so anything longer is too large for sure.
    if whole.len() > 19 || minor_digits > 18 {
        return Err(ParseMoneyError::TooLarge);
    }

    let scale = 10i128.pow(u32::from(minor_digits));
    let whole_value: i128 = whole.parse().map_err(|_| ParseMoneyError::NotANumber)?;
    let padded_fraction = format!("{fraction:0<width$}", width = usize::from(minor_digits));
    let fraction_value: i128 = if padded_fraction.is_empty() {
        0
    } else {
        padded_fraction
            .parse()
            .map_err(|_| ParseMoneyError::NotANumber)?
    };
    let size = whole_value * scale + fraction_value;
    let signed = if is_negative { -size } else { size };
    i64::try_from(signed)
        .map(Money)
        .map_err(|_| ParseMoneyError::TooLarge)
}

/// True when the text is one or more ASCII digits.
fn is_all_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}
