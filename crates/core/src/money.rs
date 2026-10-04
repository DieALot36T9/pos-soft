//! Money: whole minor units (paisa, cents) in an i64. Never a floating-point number.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{AppError, Bps, Qty, round_half_away};

/// An amount of money in minor units. With 2 currency decimals, `Money(12345)` is 123.45.
/// All arithmetic is checked: an overflow is an `AppError::Internal`, never a wrap.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS,
)]
#[serde(transparent)]
#[ts(export)]
pub struct Money(pub i64);

impl Money {
    /// No money at all.
    pub const fn zero() -> Money {
        Money(0)
    }

    /// Adds two amounts.
    pub fn checked_add(self, other: Money) -> Result<Money, AppError> {
        self.0
            .checked_add(other.0)
            .map(Money)
            .ok_or_else(|| overflow("add", self, other))
    }

    /// Takes `other` away from this amount.
    pub fn checked_sub(self, other: Money) -> Result<Money, AppError> {
        self.0
            .checked_sub(other.0)
            .map(Money)
            .ok_or_else(|| overflow("subtract", self, other))
    }

    /// Flips the sign: 5.00 becomes −5.00.
    pub fn checked_neg(self) -> Result<Money, AppError> {
        self.0
            .checked_neg()
            .map(Money)
            .ok_or_else(|| overflow("negate", self, Money::zero()))
    }

    /// Adds up a list of amounts.
    pub fn sum(amounts: &[Money]) -> Result<Money, AppError> {
        let mut total = Money::zero();
        for amount in amounts {
            total = total.checked_add(*amount)?;
        }
        Ok(total)
    }
}

/// Price for a quantity: round(price × qty / 1000). Example: 120.00/kg × 1.234 kg = 148.08.
pub fn mul_qty(price: Money, qty: Qty) -> Result<Money, AppError> {
    let exact = i128::from(price.0) * i128::from(qty.0);
    round_half_away(exact, 1000).map(Money)
}

/// A percentage of an amount: round(amount × bps / 10000). Example: 7.5% of 99.99 = 7.50.
pub fn percent_of(amount: Money, rate: Bps) -> Result<Money, AppError> {
    let exact = i128::from(amount.0) * i128::from(rate.0);
    round_half_away(exact, 10_000).map(Money)
}

fn overflow(action: &str, left: Money, right: Money) -> AppError {
    AppError::internal(format!(
        "money overflow: {action} {} and {}",
        left.0, right.0
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_past_the_limit_is_an_internal_error() {
        assert!(Money(i64::MAX).checked_add(Money(1)).is_err());
        assert!(Money(i64::MIN).checked_sub(Money(1)).is_err());
        assert!(Money(i64::MIN).checked_neg().is_err());
    }

    #[test]
    fn sum_of_nothing_is_zero() {
        assert_eq!(Money::sum(&[]).unwrap(), Money::zero());
    }

    #[test]
    fn mul_qty_handles_negative_quantities() {
        // A return line: −1.5 kg at 10.01 = −15.015, which rounds to −15.02.
        assert_eq!(mul_qty(Money(1001), Qty(-1500)).unwrap(), Money(-1502));
    }

    #[test]
    fn mul_qty_too_large_is_an_internal_error() {
        assert!(mul_qty(Money(i64::MAX), Qty(2000)).is_err());
    }

    #[test]
    fn money_is_sent_to_the_ui_as_a_plain_number() {
        assert_eq!(serde_json::to_string(&Money(12345)).unwrap(), "12345");
    }
}
