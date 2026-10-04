//! How much a return refunds for one sale line (RET-02, Money Tax and Rounding "Returns").
//! A pure function: no database. Refunds are split from the amounts stored on the
//! sale line, never from today's prices, so every refund matches what was paid.

use pos_core::{AppError, Money, Qty, round_half_away};

/// The money amounts of a line, or of the part of it being refunded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineAmounts {
    pub net: Money,
    pub tax: Money,
    pub gross: Money,
    pub cost: Money,
}

/// The refund for returning `returning` units of a line sold as `sold` units.
///
/// - The return of the last units refunds exactly what is left of each amount,
///   so all the returns of a line add up to the line exactly.
/// - Otherwise each amount is round(stored × returning ÷ sold). Example (E14): a line of
///   100.00 for 3 units, returned one at a time, refunds 33.33, 33.33, then 33.34.
/// - A refund never goes past what is left, and net is gross − tax, so every refund
///   keeps net + tax = gross and nothing is ever refunded twice.
pub(crate) fn split_refund(
    stored: LineAmounts,
    sold: Qty,
    returned_before: Qty,
    refunded_before: LineAmounts,
    returning: Qty,
) -> Result<LineAmounts, AppError> {
    let returned_after = returned_before
        .0
        .checked_add(returning.0)
        .ok_or_else(|| AppError::internal("refund quantity overflowed"))?;
    if returning.0 <= 0 || returned_before.0 < 0 || returned_after > sold.0 {
        return Err(AppError::internal(format!(
            "refund of {} after {} of {} sold",
            returning.0, returned_before.0, sold.0
        )));
    }
    let left = LineAmounts {
        net: stored.net.checked_sub(refunded_before.net)?,
        tax: stored.tax.checked_sub(refunded_before.tax)?,
        gross: stored.gross.checked_sub(refunded_before.gross)?,
        cost: stored.cost.checked_sub(refunded_before.cost)?,
    };
    if returned_after == sold.0 {
        return Ok(left);
    }
    let gross = share(stored.gross, returning, sold)?
        .min(left.gross)
        .max(Money::zero());
    // Keep tax inside what's left of both tax and net, so net never runs out first.
    let lowest_tax = gross.checked_sub(left.net)?.max(Money::zero());
    let highest_tax = left.tax.min(gross);
    let tax = share(stored.tax, returning, sold)?
        .max(lowest_tax)
        .min(highest_tax);
    let cost = share(stored.cost, returning, sold)?
        .min(left.cost)
        .max(Money::zero());
    Ok(LineAmounts {
        net: gross.checked_sub(tax)?,
        tax,
        gross,
        cost,
    })
}

/// round(amount × part ÷ whole), half away from zero.
fn share(amount: Money, part: Qty, whole: Qty) -> Result<Money, AppError> {
    let exact = i128::from(amount.0) * i128::from(part.0);
    round_half_away(exact, i128::from(whole.0)).map(Money)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn amounts(net: i64, tax: i64, cost: i64) -> LineAmounts {
        LineAmounts {
            net: Money(net),
            tax: Money(tax),
            gross: Money(net + tax),
            cost: Money(cost),
        }
    }

    fn add(left: LineAmounts, right: LineAmounts) -> LineAmounts {
        LineAmounts {
            net: Money(left.net.0 + right.net.0),
            tax: Money(left.tax.0 + right.tax.0),
            gross: Money(left.gross.0 + right.gross.0),
            cost: Money(left.cost.0 + right.cost.0),
        }
    }

    #[test]
    fn e14_ret_02_three_single_returns_refund_the_line_exactly() {
        // Line gross 100.00 for 3 units (inclusive 15%: tax 13.04, net 86.96).
        let stored = amounts(8_696, 1_304, 6_000);
        let mut refunded = LineAmounts::default();
        let mut grosses = Vec::new();
        for returned_before in [0, 1_000, 2_000] {
            let refund = split_refund(
                stored,
                Qty(3_000),
                Qty(returned_before),
                refunded,
                Qty(1_000),
            )
            .unwrap();
            grosses.push(refund.gross.0);
            refunded = add(refunded, refund);
        }
        assert_eq!(grosses, vec![3_333, 3_333, 3_334]);
        assert_eq!(refunded, stored);
    }

    #[test]
    fn ret_02_returning_everything_at_once_refunds_the_whole_line() {
        let stored = amounts(10_000, 1_500, 7_000);
        let refund = split_refund(
            stored,
            Qty(2_000),
            Qty(0),
            LineAmounts::default(),
            Qty(2_000),
        )
        .unwrap();
        assert_eq!(refund, stored);
    }

    #[test]
    fn ret_01_returning_more_than_was_sold_is_a_bug() {
        let stored = amounts(100, 0, 0);
        assert!(
            split_refund(
                stored,
                Qty(1_000),
                Qty(500),
                LineAmounts::default(),
                Qty(600)
            )
            .is_err()
        );
        assert!(split_refund(stored, Qty(1_000), Qty(0), LineAmounts::default(), Qty(0)).is_err());
    }

    #[test]
    fn ret_02_many_small_returns_never_refund_more_than_the_line() {
        // 5 minor units over 7 units: plain rounding would give 1 each time (7 > 5).
        let stored = amounts(5, 0, 0);
        let mut refunded = LineAmounts::default();
        for returned_before in 0..7 {
            let refund = split_refund(
                stored,
                Qty(7_000),
                Qty(returned_before * 1_000),
                refunded,
                Qty(1_000),
            )
            .unwrap();
            assert!(refund.gross.0 >= 0);
            refunded = add(refunded, refund);
        }
        assert_eq!(refunded, stored);
    }

    /// Splits `sold` into 1 to 6 returns of random sizes that add up to `sold`.
    fn return_sizes() -> impl Strategy<Value = (i64, Vec<i64>)> {
        (1_i64..20_000, prop::collection::vec(1_i64..1_000, 1..6)).prop_map(|(sold, weights)| {
            let weight_sum: i64 = weights.iter().sum();
            let mut sizes = Vec::new();
            let mut given = 0;
            for weight in &weights {
                let size = sold * weight / weight_sum;
                if size > 0 {
                    sizes.push(size);
                    given += size;
                }
            }
            if given < sold {
                sizes.push(sold - given);
            }
            (sold, sizes)
        })
    }

    proptest! {
        #[test]
        fn ret_02_refunds_of_a_line_always_add_up_to_the_line(
            net in 0_i64..10_000_000,
            tax in 0_i64..2_000_000,
            cost in 0_i64..10_000_000,
            (sold, sizes) in return_sizes(),
        ) {
            let stored = amounts(net, tax, cost);
            let mut refunded = LineAmounts::default();
            let mut returned = 0;
            for size in sizes {
                let refund = split_refund(stored, Qty(sold), Qty(returned), refunded, Qty(size)).unwrap();
                prop_assert!(refund.net.0 >= 0 && refund.tax.0 >= 0 && refund.cost.0 >= 0);
                prop_assert_eq!(refund.net.0 + refund.tax.0, refund.gross.0);
                refunded = add(refunded, refund);
                returned += size;
            }
            prop_assert_eq!(refunded, stored);
        }
    }
}
