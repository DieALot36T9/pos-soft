//! How a refund is paid out (RET-01, RET-07, Money Tax and Rounding "Returns").
//! Pure functions: the return workflow reads what the sale and its earlier returns
//! paid, and these split the refund and work out the points to take back.

use pos_core::{AppError, Money, RoundingAppliesTo, RoundingRule, cash_round};
use sales::PaymentKind;

/// What is left to give back of the customer's own money on a sale: the part paid on
/// credit and the part paid with points, less what earlier returns gave back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CustomerMoneyLeft {
    pub credit: Money,
    pub points: Money,
}

/// A refund split by how it goes back. total = credit + points + rest = gross + rounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RefundSplit {
    /// Back on the customer's account: off their debt first, the rest as store credit.
    pub credit: Money,
    /// Back as loyalty points, in whole points.
    pub points: Money,
    /// By the method the cashier chose (cash, card, store credit, exchange credit, …).
    pub rest: Money,
    /// Cash rounding of the refund (SET-06). total = gross + rounding.
    pub rounding: Money,
    pub total: Money,
}

/// Rule RET-07: a refund of a sale paid on credit goes off the customer's debt first,
/// then points paid go back as points (whole points only), and the rest goes by the
/// chosen method. Cash rounding works as on a sale: on everything with "all payments",
/// or on the cash part with "cash only".
pub(crate) fn split_refund(
    gross: Money,
    left: CustomerMoneyLeft,
    value_per_point: Money,
    rest_kind: PaymentKind,
    rule: &RoundingRule,
) -> Result<RefundSplit, AppError> {
    let gross = gross.max(Money::zero());
    let credit = left.credit.max(Money::zero()).min(gross);
    let after_credit = gross.checked_sub(credit)?;
    let points = if value_per_point.0 > 0 {
        let most = left.points.max(Money::zero()).min(after_credit);
        Money(most.0 / value_per_point.0 * value_per_point.0)
    } else {
        Money::zero()
    };
    let rest = after_credit.checked_sub(points)?;
    let customer_part = credit.checked_add(points)?;
    let total = match rule.applies_to {
        RoundingAppliesTo::AllPayments => cash_round(gross, rule)?,
        RoundingAppliesTo::CashOnly if rest_kind == PaymentKind::Cash => {
            customer_part.checked_add(cash_round(rest, rule)?)?
        }
        RoundingAppliesTo::CashOnly => gross,
    };
    // Rounding down can't take back money that already went to the customer's account.
    let total = if total < customer_part { gross } else { total };
    Ok(RefundSplit {
        credit,
        points,
        rest: total.checked_sub(customer_part)?,
        rounding: total.checked_sub(gross)?,
        total,
    })
}

/// Rule RET-07: the points earned on a sale are taken back in the share of the sale
/// returned. After the returns refunded `refunded_after` of the sale's `sale_gross`, the
/// customer keeps floor(earned × what is left ÷ sale gross). Returning everything takes
/// back exactly what was earned. Gives the points to take back with this return.
pub(crate) fn points_to_take_back(
    earned: i64,
    sale_gross: Money,
    refunded_after: Money,
    taken_back_before: i64,
) -> i64 {
    if earned <= 0 || sale_gross.0 <= 0 {
        return 0;
    }
    let left = (sale_gross.0 - refunded_after.0).clamp(0, sale_gross.0);
    let keep = i128::from(earned) * i128::from(left) / i128::from(sale_gross.0);
    let keep = i64::try_from(keep).unwrap_or(earned);
    (earned - keep - taken_back_before).max(0)
}

#[cfg(test)]
mod tests {
    use pos_core::RoundingMode;
    use proptest::prelude::*;

    use super::*;

    fn rule(applies_to: RoundingAppliesTo, increment: i64) -> RoundingRule {
        RoundingRule {
            increment_minor: Money(increment),
            mode: RoundingMode::Nearest,
            applies_to,
        }
    }

    const NO_ROUNDING: i64 = 1;

    #[test]
    fn ret_01_a_cash_sale_goes_back_by_the_chosen_method() {
        let split = split_refund(
            Money(12_345),
            CustomerMoneyLeft::default(),
            Money(100),
            PaymentKind::Card,
            &rule(RoundingAppliesTo::CashOnly, 100),
        )
        .unwrap();
        assert_eq!(split.rest, Money(12_345));
        assert_eq!(split.rounding, Money::zero());
    }

    #[test]
    fn ret_01_cash_refunds_round_like_sales() {
        let split = split_refund(
            Money(12_345),
            CustomerMoneyLeft::default(),
            Money(100),
            PaymentKind::Cash,
            &rule(RoundingAppliesTo::CashOnly, 100),
        )
        .unwrap();
        assert_eq!((split.rest, split.rounding), (Money(12_300), Money(-45)));
        assert_eq!(split.total, Money(12_300));
    }

    #[test]
    fn ret_07_a_credit_sale_refund_goes_off_the_debt_first() {
        let left = CustomerMoneyLeft {
            credit: Money(30_000),
            points: Money::zero(),
        };
        let rule = rule(RoundingAppliesTo::CashOnly, NO_ROUNDING);
        let split =
            split_refund(Money(50_000), left, Money(100), PaymentKind::Cash, &rule).unwrap();
        assert_eq!((split.credit, split.rest), (Money(30_000), Money(20_000)));
        let small =
            split_refund(Money(10_000), left, Money(100), PaymentKind::Cash, &rule).unwrap();
        assert_eq!((small.credit, small.rest), (Money(10_000), Money::zero()));
    }

    #[test]
    fn ret_07_points_paid_go_back_as_whole_points() {
        let left = CustomerMoneyLeft {
            credit: Money::zero(),
            points: Money(5_050),
        };
        let rule = rule(RoundingAppliesTo::CashOnly, NO_ROUNDING);
        let split =
            split_refund(Money(10_000), left, Money(100), PaymentKind::Cash, &rule).unwrap();
        assert_eq!((split.points, split.rest), (Money(5_000), Money(5_000)));
    }

    #[test]
    fn ret_07_points_earned_are_taken_back_in_share() {
        // 12 points earned on 1,200.00; return 400.00, then the rest.
        assert_eq!(points_to_take_back(12, Money(120_000), Money(40_000), 0), 4);
        assert_eq!(
            points_to_take_back(12, Money(120_000), Money(120_000), 4),
            8
        );
        // 5 points on 100.00 returned a third at a time: 2, 2, then the last 1.
        let first = points_to_take_back(5, Money(10_000), Money(3_333), 0);
        let second = points_to_take_back(5, Money(10_000), Money(6_666), first);
        let third = points_to_take_back(5, Money(10_000), Money(10_000), first + second);
        assert_eq!((first, second, third), (2, 2, 1));
        assert_eq!(points_to_take_back(0, Money(10_000), Money(10_000), 0), 0);
    }

    proptest! {
        /// The split always adds up to the total, and nothing goes back that wasn't paid.
        #[test]
        fn ret_02_a_split_always_adds_up(
            gross in 0i64..10_000_000,
            credit in 0i64..10_000_000,
            points in 0i64..10_000_000,
            value_per_point in 0i64..1_000,
            increment in prop::sample::select(vec![1i64, 50, 100, 500]),
            all in any::<bool>(),
            cash in any::<bool>(),
        ) {
            let applies_to = if all { RoundingAppliesTo::AllPayments } else { RoundingAppliesTo::CashOnly };
            let kind = if cash { PaymentKind::Cash } else { PaymentKind::Card };
            let left = CustomerMoneyLeft { credit: Money(credit), points: Money(points) };
            let split = split_refund(Money(gross), left, Money(value_per_point), kind, &rule(applies_to, increment)).unwrap();
            prop_assert_eq!(split.credit.0 + split.points.0 + split.rest.0, split.total.0);
            prop_assert_eq!(split.total.0, gross + split.rounding.0);
            prop_assert!(split.credit.0 <= credit && split.points.0 <= points);
            prop_assert!(split.rest.0 >= 0 && split.credit.0 >= 0 && split.points.0 >= 0);
            if value_per_point > 0 {
                prop_assert_eq!(split.points.0 % value_per_point, 0);
            }
            prop_assert!(split.rounding.0.abs() * 2 <= increment);
        }

        /// However a sale is returned, every point earned is taken back once, never more.
        #[test]
        fn ret_07_points_taken_back_add_up_to_the_points_earned(
            earned in 0i64..10_000,
            parts in prop::collection::vec(1i64..100_000, 1..8),
        ) {
            let sale_gross: i64 = parts.iter().sum();
            let mut refunded = 0;
            let mut taken = 0;
            for part in parts {
                refunded += part;
                let back = points_to_take_back(earned, Money(sale_gross), Money(refunded), taken);
                prop_assert!(back >= 0);
                taken += back;
                prop_assert!(taken <= earned);
            }
            prop_assert_eq!(taken, earned);
        }
    }
}
