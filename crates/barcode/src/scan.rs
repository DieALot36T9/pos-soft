//! Reading a scanned code (Rule CHK-08): is it an ordinary barcode or a scale sticker?

use pos_core::{Money, Qty, round_half_away};

use crate::gtin::is_valid_gtin;
use crate::rule::{Pattern, ScaleKind, ScaleRule, Slot, read_pattern};

/// What a scanned code turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scanned {
    /// An ordinary code. Look it up as a product barcode or SKU.
    Plain(String),
    /// A weight sticker: find the product by its scale item code and sell this quantity.
    Weighed { item_code: String, qty: Qty },
    /// A price sticker: find the product by its scale item code and charge exactly this price.
    Priced { item_code: String, price: Money },
}

/// Works out what a scanned code means, using the shop's scale-label rules in order.
/// A sticker with a wrong check digit comes back as `Plain`, so it is later rejected
/// like any unknown code. Spaces and line endings that scanners add are removed.
pub fn parse(code: &str, rules: &[ScaleRule], currency_minor_digits: u8) -> Scanned {
    let code = code.trim();
    for rule in rules {
        // A broken rule is skipped: one bad setting must not stop the till.
        let Ok(pattern) = read_pattern(&rule.pattern) else {
            continue;
        };
        if let Some(scanned) = read_sticker(code, rule.kind, &pattern, currency_minor_digits) {
            return scanned;
        }
    }
    Scanned::Plain(code.to_string())
}

/// Reads the code with one rule. `None` if the rule doesn't fit the code.
fn read_sticker(
    code: &str,
    kind: ScaleKind,
    pattern: &Pattern,
    currency_minor_digits: u8,
) -> Option<Scanned> {
    // The pattern covers every digit except the last one, which is the check digit.
    if code.len() != pattern.slots.len() + 1 || !is_valid_gtin(code) {
        return None;
    }
    let mut item_code = String::new();
    let mut value: i64 = 0;
    for (slot, byte) in pattern.slots.iter().zip(code.bytes()) {
        let digit = byte - b'0';
        match slot {
            Slot::Fixed(wanted) if *wanted != digit => return None,
            Slot::Fixed(_) => {}
            Slot::Item => item_code.push(char::from(byte)),
            Slot::Value => value = value.checked_mul(10)?.checked_add(i64::from(digit))?,
        }
    }
    match kind {
        ScaleKind::Weight => {
            // Qty counts thousandths of the unit (ADR-007).
            let thousandths = change_decimals(value, pattern.decimals, 3)?;
            Some(Scanned::Weighed {
                item_code,
                qty: Qty(thousandths),
            })
        }
        ScaleKind::Price => {
            let minor_units =
                change_decimals(value, pattern.decimals, u32::from(currency_minor_digits))?;
            Some(Scanned::Priced {
                item_code,
                price: Money(minor_units),
            })
        }
    }
}

/// Moves a whole number with `from` decimals to one with `to` decimals.
/// Example: 15000 with 2 decimals is 150 with 0 decimals. Dropped digits are
/// rounded half away from zero, the project's normal rounding.
fn change_decimals(value: i64, from: u32, to: u32) -> Option<i64> {
    if to >= from {
        value.checked_mul(10_i64.checked_pow(to - from)?)
    } else {
        let divisor = 10_i128.checked_pow(from - to)?;
        round_half_away(i128::from(value), divisor).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chk08_extra_decimals_are_rounded_half_away_from_zero() {
        assert_eq!(change_decimals(12345, 4, 3), Some(1235));
        assert_eq!(change_decimals(12344, 4, 3), Some(1234));
        assert_eq!(change_decimals(15_000, 2, 0), Some(150));
    }

    #[test]
    fn chk08_huge_decimal_shift_is_refused_instead_of_overflowing() {
        assert_eq!(change_decimals(1, 0, 255), None);
    }
}
