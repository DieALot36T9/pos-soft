//! Numbers as printed: money (through `pos_core::format_money`), quantities and
//! percentages. Only formatting; every amount is already worked out by the caller.

use pos_core::{Bps, CurrencyFormat, Money, Qty, format_money};

/// Money with the currency symbol. Example: "৳1,805.00".
pub(crate) fn money_with_symbol(amount: Money, currency: &CurrencyFormat) -> String {
    format_money(amount, currency)
}

/// Money without the symbol, for columns of amounts. Example: "1,805.00".
pub(crate) fn money(amount: Money, currency: &CurrencyFormat) -> String {
    let mut plain = currency.clone();
    plain.symbol = String::new();
    format_money(amount, &plain)
}

/// A quantity with no extra zeros, and its unit if it has one.
/// Examples: Qty(2000) → "2", Qty(1234) and "kg" → "1.234 kg", Qty(1500) → "1.5".
pub(crate) fn quantity(qty: Qty, unit: &str, decimal_sep: &str) -> String {
    let number = without_trailing_zeros(qty.0, 3, decimal_sep);
    if unit.is_empty() {
        number
    } else {
        format!("{number} {unit}")
    }
}

/// A rate as a percentage with no extra zeros. Examples: Bps(1500) → "15%", Bps(1250) → "12.5%".
pub(crate) fn percent(rate: Bps, decimal_sep: &str) -> String {
    format!(
        "{}%",
        without_trailing_zeros(i64::from(rate.0), 2, decimal_sep)
    )
}

/// Writes `value / 10^digits` with only the decimals it needs.
fn without_trailing_zeros(value: i64, digits: u32, decimal_sep: &str) -> String {
    let divisor = 10u64.pow(digits);
    let size = value.unsigned_abs();
    let sign = if value < 0 { "-" } else { "" };
    let whole = size / divisor;
    let fraction = size % divisor;
    if fraction == 0 {
        return format!("{sign}{whole}");
    }
    let decimals = format!("{fraction:0width$}", width = digits as usize);
    format!(
        "{sign}{whole}{decimal_sep}{}",
        decimals.trim_end_matches('0')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantities_drop_extra_zeros() {
        assert_eq!(quantity(Qty(2000), "", "."), "2");
        assert_eq!(quantity(Qty(1234), "kg", "."), "1.234 kg");
        assert_eq!(quantity(Qty(1500), "", ","), "1,5");
        assert_eq!(quantity(Qty(5), "g", "."), "0.005 g");
        assert_eq!(quantity(Qty(-1000), "", "."), "-1");
    }

    #[test]
    fn percentages_drop_extra_zeros() {
        assert_eq!(percent(Bps(1500), "."), "15%");
        assert_eq!(percent(Bps(1250), "."), "12.5%");
        assert_eq!(percent(Bps(1234), "."), "12.34%");
        assert_eq!(percent(Bps(0), "."), "0%");
    }
}
