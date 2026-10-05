//! The shift rules as plain functions with no database, so they are easy to test.

use std::collections::{BTreeMap, BTreeSet};

use pos_core::{AppError, Id, Money, Timestamp};

use crate::ShiftsError;
use crate::model::{CASH, MethodTotal, MovementKind, MovementTotal, ShiftMovement};

/// The opening float can't be negative. If notes were counted, they must add up to it.
pub(crate) fn check_float(float: Money, counts: &[(Money, u32)]) -> Result<(), ShiftsError> {
    if float.0 < 0 {
        return Err(ShiftsError::Invalid("float_negative"));
    }
    if !counts.is_empty() && count_total(counts)? != float {
        return Err(ShiftsError::Invalid("float_count_mismatch"));
    }
    Ok(())
}

/// The value of a denomination count. Example: 12 × 1,000.00 + 4 × 100.00 = 12,400.00.
/// Each denomination must be above zero and appear once.
pub(crate) fn count_total(counts: &[(Money, u32)]) -> Result<Money, ShiftsError> {
    let mut seen = BTreeSet::new();
    let mut total = Money::zero();
    for &(denomination, count) in counts {
        if denomination.0 <= 0 {
            return Err(ShiftsError::Invalid("denomination_invalid"));
        }
        if !seen.insert(denomination.0) {
            return Err(ShiftsError::Invalid("denomination_repeated"));
        }
        let value = i128::from(denomination.0) * i128::from(count);
        let value = i64::try_from(value)
            .map_err(|_| AppError::internal("shifts: denomination count overflowed"))?;
        total = total.checked_add(Money(value))?;
    }
    Ok(total)
}

/// Checks a movement that a workflow wants to record.
/// Float and void reversal rows are written only by `open` and `reverse_ref`.
pub(crate) fn check_movement(movement: &ShiftMovement) -> Result<(), ShiftsError> {
    let amount = movement.amount.0;
    let sign_ok = match movement.kind {
        MovementKind::Float | MovementKind::VoidReversal => {
            return Err(ShiftsError::Invalid("movement_kind_not_allowed"));
        }
        MovementKind::SalePayment | MovementKind::CashIn | MovementKind::CustomerPayment => {
            amount > 0
        }
        MovementKind::Refund
        | MovementKind::CashOut
        | MovementKind::CashDrop
        | MovementKind::SupplierPayment => amount < 0,
    };
    if !sign_ok {
        return Err(ShiftsError::Invalid("amount_wrong_sign"));
    }
    if movement.method_kind.trim().is_empty() {
        return Err(ShiftsError::Invalid("method_kind_missing"));
    }
    if needs_reason(movement.kind) && !has_text(&movement.reason) {
        return Err(ShiftsError::Invalid("reason_missing"));
    }
    Ok(())
}

/// Rule SHF-03: cash in, cash out and cash drops each need a reason.
fn needs_reason(kind: MovementKind) -> bool {
    matches!(
        kind,
        MovementKind::CashIn | MovementKind::CashOut | MovementKind::CashDrop
    )
}

fn has_text(text: &Option<String>) -> bool {
    text.as_deref().is_some_and(|text| !text.trim().is_empty())
}

/// Rule SHF-06: expected cash is the float, plus cash sales, minus cash refunds, plus
/// cash in, minus cash out, plus cash customer payments (and the other cash kinds).
/// Every amount already carries its sign, so this adds up the cash movements.
pub(crate) fn expected_cash(movements: &[ShiftMovement]) -> Result<Money, ShiftsError> {
    let mut total = Money::zero();
    for movement in movements {
        if movement.method_kind == CASH {
            total = total.checked_add(movement.amount)?;
        }
    }
    Ok(total)
}

/// Counted − expected. A shortage is negative.
pub(crate) fn difference(counted: Money, expected: Money) -> Result<Money, ShiftsError> {
    Ok(counted.checked_sub(expected)?)
}

/// Adds up the movements by kind and method kind, in kind order.
pub(crate) fn totals(movements: &[ShiftMovement]) -> Result<Vec<MovementTotal>, ShiftsError> {
    let mut groups: BTreeMap<(MovementKind, &str), (u32, Money)> = BTreeMap::new();
    for movement in movements {
        let key = (movement.kind, movement.method_kind.as_str());
        let (count, total) = groups.entry(key).or_insert((0, Money::zero()));
        *count += 1;
        *total = total.checked_add(movement.amount)?;
    }
    let mut result = Vec::new();
    for ((kind, method_kind), (count, total)) in groups {
        result.push(MovementTotal {
            kind,
            method_kind: method_kind.to_string(),
            count,
            total,
        });
    }
    Ok(result)
}

/// Rule SHF-08: payments by method are sales − refunds. Void reversals count too,
/// because they undo sale payments and refunds.
pub(crate) fn payments_by_method(
    movements: &[ShiftMovement],
) -> Result<Vec<MethodTotal>, ShiftsError> {
    let mut groups: BTreeMap<(&str, Option<&Id>), Money> = BTreeMap::new();
    for movement in movements {
        let is_payment = matches!(
            movement.kind,
            MovementKind::SalePayment | MovementKind::Refund | MovementKind::VoidReversal
        );
        if is_payment {
            let key = (movement.method_kind.as_str(), movement.method_id.as_ref());
            let total = groups.entry(key).or_insert(Money::zero());
            *total = total.checked_add(movement.amount)?;
        }
    }
    let mut result = Vec::new();
    for ((method_kind, method_id), total) in groups {
        result.push(MethodTotal {
            method_kind: method_kind.to_string(),
            method_id: method_id.cloned(),
            total,
        });
    }
    Ok(result)
}

/// The rows that undo a voided document: each movement again with the opposite sign.
/// `movements` are the document's movements in this shift.
pub(crate) fn reversals(
    movements: &[ShiftMovement],
    ref_type: &str,
    ref_id: &Id,
    by: &Id,
    now: Timestamp,
) -> Result<Vec<ShiftMovement>, ShiftsError> {
    if movements
        .iter()
        .any(|movement| movement.kind == MovementKind::VoidReversal)
    {
        return Err(ShiftsError::AlreadyReversed {
            ref_type: ref_type.to_string(),
            ref_id: ref_id.clone(),
        });
    }
    if movements.is_empty() {
        return Err(ShiftsError::NothingToReverse {
            ref_type: ref_type.to_string(),
            ref_id: ref_id.clone(),
        });
    }
    let mut result = Vec::new();
    for movement in movements {
        result.push(ShiftMovement {
            kind: MovementKind::VoidReversal,
            method_id: movement.method_id.clone(),
            method_kind: movement.method_kind.clone(),
            amount: movement.amount.checked_neg()?,
            ref_type: Some(ref_type.to_string()),
            ref_id: Some(ref_id.clone()),
            reason: None,
            at: now,
            user_id: by.clone(),
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cash(kind: MovementKind, amount: i64) -> ShiftMovement {
        ShiftMovement {
            kind,
            method_id: None,
            method_kind: CASH.to_string(),
            amount: Money(amount),
            ref_type: None,
            ref_id: None,
            reason: Some("Reason".to_string()),
            at: Timestamp(0),
            user_id: Id::new(),
        }
    }

    #[test]
    fn shf_06_count_total_multiplies_and_adds() {
        let counts = [(Money(100_000), 12), (Money(10_000), 4)];
        assert_eq!(count_total(&counts).unwrap(), Money(1_240_000));
        assert_eq!(count_total(&[]).unwrap(), Money::zero());
    }

    #[test]
    fn shf_06_count_total_refuses_bad_denominations_and_overflow() {
        assert_eq!(
            count_total(&[(Money(0), 1)]),
            Err(ShiftsError::Invalid("denomination_invalid"))
        );
        assert_eq!(
            count_total(&[(Money(100), 1), (Money(100), 2)]),
            Err(ShiftsError::Invalid("denomination_repeated"))
        );
        let huge = [(Money(i64::MAX), 2)];
        assert!(matches!(count_total(&huge), Err(ShiftsError::Core(_))));
    }

    #[test]
    fn shf_01_a_zero_float_with_no_count_is_fine() {
        assert_eq!(check_float(Money::zero(), &[]), Ok(()));
    }

    #[test]
    fn shf_03_a_blank_reason_counts_as_missing() {
        let mut drop = cash(MovementKind::CashDrop, -100);
        drop.reason = Some("  ".to_string());
        assert_eq!(
            check_movement(&drop),
            Err(ShiftsError::Invalid("reason_missing"))
        );
    }

    #[test]
    fn sales_and_refunds_need_no_reason_but_need_a_method() {
        let mut sale = cash(MovementKind::SalePayment, 100);
        sale.reason = None;
        assert_eq!(check_movement(&sale), Ok(()));
        sale.method_kind = String::new();
        assert_eq!(
            check_movement(&sale),
            Err(ShiftsError::Invalid("method_kind_missing"))
        );
    }

    #[test]
    fn shf_06_expected_cash_overflow_is_an_error_not_a_wrong_number() {
        let rows = [
            cash(MovementKind::SalePayment, i64::MAX),
            cash(MovementKind::SalePayment, 1),
        ];
        assert!(matches!(expected_cash(&rows), Err(ShiftsError::Core(_))));
    }

    #[test]
    fn ret_04_reversals_flip_the_sign_and_keep_the_method() {
        let sale = Id::new();
        let by = Id::new();
        let mut card = cash(MovementKind::SalePayment, 500);
        card.method_kind = "card".to_string();
        let rows = reversals(&[card], "sale", &sale, &by, Timestamp(9)).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, MovementKind::VoidReversal);
        assert_eq!(rows[0].amount, Money(-500));
        assert_eq!(rows[0].method_kind, "card");
        assert_eq!(rows[0].ref_id, Some(sale));
        assert_eq!(rows[0].user_id, by);
        assert_eq!(rows[0].at, Timestamp(9));
    }
}
