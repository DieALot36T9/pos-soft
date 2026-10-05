//! Reading one cell's text as money, a quantity, yes/no or a date (IO-04).
//! Each function returns the value, or a short code that names the problem.
//! The code goes into a `CellErr`, and the UI turns it into plain English.
//! Nothing is ever rounded: too many decimals is an error.

use chrono::{Days, NaiveDate};
use pos_core::{Money, ParseMoneyError, Qty};

/// Reads an amount such as "12.50" using the currency's decimals.
/// Example: ("12.5", 2) → Money(1250). ("12.505", 2) → "money_too_many_decimals".
/// Thousands separators are refused, because "1,234" means 1234 in one country
/// and 1.234 in another.
pub fn parse_money(cell: &str, minor_digits: u8) -> Result<Money, String> {
    if has_separator(cell) {
        return Err("money_has_separator".to_string());
    }
    pos_core::parse_money(cell, minor_digits).map_err(|error| error.code().to_string())
}

/// Reads a quantity such as "1.234" with at most `decimals` decimals (0 to 3).
/// Example: ("1.234", 3) → Qty(1234). ("2", 0) → Qty(2000).
pub fn parse_qty(cell: &str, decimals: u8) -> Result<Qty, String> {
    if has_separator(cell) {
        return Err("qty_has_separator".to_string());
    }
    if decimals > 3 {
        return Err("qty_too_many_decimals".to_string());
    }
    // A quantity is read like an amount with `decimals` decimals, then scaled
    // up to thousandths. Example: "1.5" with 1 decimal → 15 → 1500.
    let scaled = pos_core::parse_money(cell, decimals).map_err(qty_code)?;
    let to_thousandths = 10i64.pow(u32::from(3 - decimals));
    scaled
        .0
        .checked_mul(to_thousandths)
        .map(Qty)
        .ok_or_else(|| "qty_too_large".to_string())
}

/// Reads yes/no, true/false or 1/0, in any letter case.
pub fn parse_bool(cell: &str) -> Result<bool, String> {
    let word = cell.trim().to_lowercase();
    match word.as_str() {
        "" => Err("bool_empty".to_string()),
        "yes" | "true" | "1" => Ok(true),
        "no" | "false" | "0" => Ok(false),
        _ => Err("bool_not_yes_no".to_string()),
    }
}

/// Reads a date written as 2026-03-01, 01/03/2026 (day first) or an Excel day
/// number such as 46082. A time after an Excel day number is ignored.
pub fn parse_date(cell: &str) -> Result<NaiveDate, String> {
    let text = cell.trim();
    if text.is_empty() {
        return Err("date_empty".to_string());
    }
    let not_a_date = || "date_not_a_date".to_string();
    if let Ok(date) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        return Ok(date);
    }
    if let Ok(date) = NaiveDate::parse_from_str(text, "%d/%m/%Y") {
        return Ok(date);
    }
    let day_part = match text.split_once('.') {
        Some((whole, fraction)) if is_digits(fraction) => whole,
        Some(_) => return Err(not_a_date()),
        None => text,
    };
    if !is_digits(day_part) {
        return Err(not_a_date());
    }
    let day_number: u64 = day_part.parse().map_err(|_| not_a_date())?;
    excel_day(day_number).ok_or_else(not_a_date)
}

/// Turns an Excel day number into a date. Excel calls 1900-01-01 day 1, but it
/// also counts a 29 February 1900 that never existed (day 60). So from day 61
/// on, the days are counted from 1899-12-30.
fn excel_day(day_number: u64) -> Option<NaiveDate> {
    let start = match day_number {
        0 | 60 => return None,
        1..=59 => NaiveDate::from_ymd_opt(1899, 12, 31)?,
        _ => NaiveDate::from_ymd_opt(1899, 12, 30)?,
    };
    start.checked_add_days(Days::new(day_number))
}

/// True when a number has a thousands separator in it: a comma, an apostrophe
/// or a space between the digits. Example: "1,234" or "1 234.50".
fn has_separator(cell: &str) -> bool {
    cell.trim()
        .chars()
        .any(|character| character == ',' || character == '\'' || character.is_whitespace())
}

/// True when the text is one or more ASCII digits.
fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// The quantity version of a money parse error's code.
fn qty_code(error: ParseMoneyError) -> String {
    match error {
        ParseMoneyError::Empty => "qty_empty",
        ParseMoneyError::NotANumber => "qty_not_a_number",
        ParseMoneyError::TooManyDecimals { .. } => "qty_too_many_decimals",
        ParseMoneyError::TooLarge => "qty_too_large",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_04_excel_day_numbers_around_the_1900_mistake() {
        let date = |year, month, day| NaiveDate::from_ymd_opt(year, month, day);
        assert_eq!(excel_day(59), date(1900, 2, 28));
        assert_eq!(excel_day(60), None);
        assert_eq!(excel_day(61), date(1900, 3, 1));
        assert_eq!(excel_day(45_658), date(2025, 1, 1));
    }

    #[test]
    fn io_04_huge_quantities_are_too_large_not_wrapped() {
        assert_eq!(
            parse_qty("9223372036854776", 0),
            Err("qty_too_large".to_string())
        );
    }

    #[test]
    fn io_04_dates_with_letters_after_the_point_are_refused() {
        assert_eq!(parse_date("46082.x"), Err("date_not_a_date".to_string()));
        assert_eq!(parse_date("-5"), Err("date_not_a_date".to_string()));
    }
}
