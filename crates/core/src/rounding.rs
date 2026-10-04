//! Rounding: the one rounding rule (half away from zero) and cash rounding.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{AppError, Money};

/// Divides `numer` by `denom` and rounds half away from zero. Examples: 2.5 → 3, −2.5 → −3.
pub fn round_half_away(numer: i128, denom: i128) -> Result<i64, AppError> {
    // checked_div and checked_rem catch division by zero and the one overflow case.
    let (Some(quotient), Some(remainder)) = (numer.checked_div(denom), numer.checked_rem(denom))
    else {
        return Err(AppError::internal(format!(
            "round_half_away cannot divide {numer} by {denom}"
        )));
    };
    let rest = remainder.unsigned_abs();
    let divisor = denom.unsigned_abs();
    // Move one step away from zero when the rest is at least half the divisor.
    let rounded = if rest != 0 && rest >= divisor - rest {
        let answer_is_negative = (numer < 0) != (denom < 0);
        if answer_is_negative {
            quotient - 1
        } else {
            quotient + 1
        }
    } else {
        quotient
    };
    i64::try_from(rounded)
        .map_err(|_| AppError::internal(format!("round_half_away result {rounded} is too large")))
}

/// Which way cash rounding goes. Up and Down are about size, so they treat refunds
/// (negative amounts) the same way as sales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RoundingMode {
    /// To the nearest step; exactly half goes away from zero. 123.50 → 124.00
    Nearest,
    /// Towards zero (the customer pays less). 123.99 → 123.00
    Down,
    /// Away from zero (the customer pays more). 123.01 → 124.00
    Up,
}

/// Which payments cash rounding applies to. The sales module uses this; `cash_round` doesn't.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RoundingAppliesTo {
    /// Every sale is rounded.
    AllPayments,
    /// Only rounded when the rest of the amount is paid in cash (rule PAY-06).
    CashOnly,
}

/// The shop's cash rounding setting (rule SET-06).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RoundingRule {
    /// The step to round to, in minor units. 100 = one whole unit. 0 = rounding is off.
    pub increment_minor: Money,
    /// Which way to round.
    pub mode: RoundingMode,
    /// Which payments the rounding applies to.
    pub applies_to: RoundingAppliesTo,
}

/// Rounds an amount to the shop's cash step and returns the rounded amount.
/// Example: 123.45 with step 1.00 and mode Nearest gives 123.00.
pub fn cash_round(amount: Money, rule: &RoundingRule) -> Result<Money, AppError> {
    let step = rule.increment_minor.0;
    if step == 0 {
        return Ok(amount);
    }
    if step < 0 {
        return Err(AppError::internal(format!(
            "cash rounding step {step} is negative"
        )));
    }
    let steps = match rule.mode {
        RoundingMode::Nearest => round_half_away(i128::from(amount.0), i128::from(step))?,
        // Integer division in Rust already goes towards zero.
        RoundingMode::Down => amount.0 / step,
        RoundingMode::Up => {
            let whole_steps = amount.0 / step;
            if amount.0 % step == 0 {
                whole_steps
            } else if amount.0 < 0 {
                whole_steps - 1
            } else {
                whole_steps + 1
            }
        }
    };
    steps
        .checked_mul(step)
        .map(Money)
        .ok_or_else(|| AppError::internal(format!("cash rounding of {} overflowed", amount.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(step: i64, mode: RoundingMode) -> RoundingRule {
        RoundingRule {
            increment_minor: Money(step),
            mode,
            applies_to: RoundingAppliesTo::AllPayments,
        }
    }

    #[test]
    fn half_goes_away_from_zero() {
        assert_eq!(round_half_away(5, 2).unwrap(), 3);
        assert_eq!(round_half_away(-5, 2).unwrap(), -3);
        assert_eq!(round_half_away(5, -2).unwrap(), -3);
        assert_eq!(round_half_away(-5, -2).unwrap(), 3);
    }

    #[test]
    fn below_half_goes_towards_zero() {
        assert_eq!(round_half_away(4, 3).unwrap(), 1);
        assert_eq!(round_half_away(-4, 3).unwrap(), -1);
        assert_eq!(round_half_away(0, 7).unwrap(), 0);
    }

    #[test]
    fn division_by_zero_is_an_internal_error() {
        assert!(matches!(
            round_half_away(1, 0),
            Err(AppError::Internal { .. })
        ));
    }

    #[test]
    fn result_too_large_for_money_is_an_internal_error() {
        assert!(matches!(
            round_half_away(i128::MAX, 1),
            Err(AppError::Internal { .. })
        ));
        assert!(matches!(
            round_half_away(i128::MIN, -1),
            Err(AppError::Internal { .. })
        ));
    }

    #[test]
    fn set_06_rounding_off_keeps_the_amount() {
        assert_eq!(
            cash_round(Money(12345), &rule(0, RoundingMode::Nearest)).unwrap(),
            Money(12345)
        );
    }

    #[test]
    fn set_06_down_and_up_round_by_size() {
        assert_eq!(
            cash_round(Money(12399), &rule(100, RoundingMode::Down)).unwrap(),
            Money(12300)
        );
        assert_eq!(
            cash_round(Money(12301), &rule(100, RoundingMode::Up)).unwrap(),
            Money(12400)
        );
        assert_eq!(
            cash_round(Money(-12399), &rule(100, RoundingMode::Down)).unwrap(),
            Money(-12300)
        );
        assert_eq!(
            cash_round(Money(-12301), &rule(100, RoundingMode::Up)).unwrap(),
            Money(-12400)
        );
        assert_eq!(
            cash_round(Money(12300), &rule(100, RoundingMode::Up)).unwrap(),
            Money(12300)
        );
    }

    #[test]
    fn set_06_step_of_fifty() {
        assert_eq!(
            cash_round(Money(12324), &rule(50, RoundingMode::Nearest)).unwrap(),
            Money(12300)
        );
        assert_eq!(
            cash_round(Money(12325), &rule(50, RoundingMode::Nearest)).unwrap(),
            Money(12350)
        );
    }

    #[test]
    fn negative_step_is_an_internal_error() {
        assert!(cash_round(Money(100), &rule(-5, RoundingMode::Nearest)).is_err());
    }
}
