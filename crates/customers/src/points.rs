//! Loyalty points: the maths (pure functions) and the points ledger (rule CUS-04, PAY-05).

use pos_core::{AppError, Id, Money};
use rusqlite::{Transaction, named_params};

use crate::rows::{load_balance, save_points};
use crate::{CustomersError, LedgerCtx, LedgerRef, LoyaltySettingsView, PointsKind};

/// The part of a sale that earns points: what was paid, less what was paid with points
/// or customer credit (rule CUS-04). Example: 1,250.00 due, 50.00 paid with points → 1,200.00.
pub fn eligible_for_points(
    total_due: Money,
    paid_by_points: Money,
    paid_by_credit: Money,
) -> Result<Money, CustomersError> {
    let eligible = total_due
        .checked_sub(paid_by_points)?
        .checked_sub(paid_by_credit)?;
    Ok(eligible.max(Money::zero()))
}

/// Points earned on an eligible amount: whole spend units only, rounded down.
/// Example E16: 1 point per 100.00 on 1,250.00 earns 12 points.
pub fn points_earned(eligible: Money, settings: &LoyaltySettingsView) -> i64 {
    if !settings.enabled || settings.spend_unit.0 <= 0 || eligible.0 <= 0 {
        return 0;
    }
    let whole_units = eligible.0 / settings.spend_unit.0;
    whole_units.saturating_mul(settings.points_per_unit.max(0))
}

/// What a number of points is worth as payment. Example: 1,200 points at 0.10 = 120.00.
pub fn points_value(points: i64, settings: &LoyaltySettingsView) -> Result<Money, CustomersError> {
    points
        .checked_mul(settings.value_per_point.0)
        .map(Money)
        .ok_or_else(|| CustomersError::App(AppError::internal("points value overflow")))
}

/// The most points that may pay towards `amount_due`, in whole points (rule PAY-05).
/// Nothing if loyalty is off or the balance is under the minimum to redeem.
/// Example: 1,200 points at 0.10 with 500.00 due → 1,200 points (120.00).
pub fn max_points_to_redeem(
    balance: i64,
    amount_due: Money,
    settings: &LoyaltySettingsView,
) -> i64 {
    if !settings.enabled || balance <= 0 || balance < settings.min_redeem {
        return 0;
    }
    if settings.value_per_point.0 <= 0 || amount_due.0 <= 0 {
        return 0;
    }
    // Round down, so points never pay more than what is due.
    let points_for_amount_due = amount_due.0 / settings.value_per_point.0;
    balance.min(points_for_amount_due)
}

/// Adds a points ledger row and gives back the customer's new points balance.
/// Earning is refused for customers who opted out. Redeeming and adjusting can't take the
/// balance below zero; a void or return (`Reverse`) can. Adjustments need a reason (CUS-04).
pub fn add_points(
    tx: &Transaction,
    customer: &Id,
    delta: i64,
    kind: PointsKind,
    ref_: Option<&LedgerRef>,
    reason: Option<&str>,
    ctx: &LedgerCtx,
) -> Result<i64, CustomersError> {
    kind.check_sign(delta)?;
    let reason = reason.map(str::trim).filter(|text| !text.is_empty());
    if kind == PointsKind::Adjust && reason.is_none() {
        return Err(CustomersError::ReasonMissing);
    }
    let balance = load_balance(tx, customer)?;
    if kind == PointsKind::Earn && balance.loyalty_opt_out {
        return Err(CustomersError::LoyaltyOptedOut);
    }
    let new_balance = add_to_balance(balance.points, delta)?;
    let may_go_below_zero = kind == PointsKind::Reverse;
    if new_balance < 0 && !may_go_below_zero {
        return Err(CustomersError::PointsNotEnough {
            balance: balance.points,
        });
    }
    insert_points_row(tx, customer, delta, kind, ref_, reason, ctx)?;
    save_points(tx, customer, new_balance)?;
    Ok(new_balance)
}

/// Adds two points numbers, refusing an overflow.
pub(crate) fn add_to_balance(balance: i64, delta: i64) -> Result<i64, CustomersError> {
    balance
        .checked_add(delta)
        .ok_or_else(|| CustomersError::App(AppError::internal("points balance overflow")))
}

/// Writes one row to the append-only points ledger. Callers update the balance.
pub(crate) fn insert_points_row(
    tx: &Transaction,
    customer: &Id,
    delta: i64,
    kind: PointsKind,
    ref_: Option<&LedgerRef>,
    reason: Option<&str>,
    ctx: &LedgerCtx,
) -> Result<(), CustomersError> {
    tx.execute(
        "INSERT INTO customers_points_ledger
           (id, customer_id, points_delta, kind, ref_type, ref_id, reason, at, user_id)
         VALUES (:id, :customer_id, :delta, :kind, :ref_type, :ref_id, :reason, :at, :user_id)",
        named_params! {
            ":id": Id::new().as_str(),
            ":customer_id": customer.as_str(),
            ":delta": delta,
            ":kind": kind.as_db(),
            ":ref_type": ref_.map(|r| r.ref_type.as_str()),
            ":ref_id": ref_.map(|r| r.ref_id.as_str()),
            ":reason": reason,
            ":at": ctx.at.0,
            ":user_id": ctx.user_id.as_ref().map(Id::as_str),
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_point_per_100() -> LoyaltySettingsView {
        LoyaltySettingsView {
            enabled: true,
            spend_unit: Money(10_000),
            points_per_unit: 1,
            value_per_point: Money(10),
            min_redeem: 100,
        }
    }

    #[test]
    fn cus_04_e16_1250_cash_earns_12_points() {
        assert_eq!(points_earned(Money(125_000), &one_point_per_100()), 12);
    }

    #[test]
    fn cus_04_earned_points_exclude_points_and_credit_paid_amounts() {
        // 1,250.00 due: 100.00 paid with points and 200.00 on customer credit.
        let eligible = eligible_for_points(Money(125_000), Money(10_000), Money(20_000)).unwrap();
        assert_eq!(eligible, Money(95_000));
        assert_eq!(points_earned(eligible, &one_point_per_100()), 9);
    }

    #[test]
    fn cus_04_no_points_when_loyalty_is_off_or_nothing_is_eligible() {
        let mut off = one_point_per_100();
        off.enabled = false;
        assert_eq!(points_earned(Money(125_000), &off), 0);
        assert_eq!(points_earned(Money(9_999), &one_point_per_100()), 0);
        assert_eq!(points_earned(Money(-50_000), &one_point_per_100()), 0);
        let all_on_points = eligible_for_points(Money(100), Money(100), Money(50)).unwrap();
        assert_eq!(all_on_points, Money::zero());
    }

    #[test]
    fn pay_05_1200_points_at_0_10_pay_up_to_120() {
        let settings = one_point_per_100();
        let points = max_points_to_redeem(1_200, Money(50_000), &settings);
        assert_eq!(points, 1_200);
        assert_eq!(points_value(points, &settings).unwrap(), Money(12_000));
    }

    #[test]
    fn pay_05_points_never_pay_more_than_is_due_and_respect_the_minimum() {
        let settings = one_point_per_100();
        // 45.55 due: 455 whole points (45.50); the last 0.05 is paid another way.
        assert_eq!(max_points_to_redeem(1_200, Money(4_555), &settings), 455);
        assert_eq!(max_points_to_redeem(99, Money(50_000), &settings), 0);
    }

    #[test]
    fn points_value_overflow_is_an_error() {
        assert!(points_value(i64::MAX, &one_point_per_100()).is_err());
    }
}
