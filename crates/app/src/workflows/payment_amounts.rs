//! The amounts on the payment dialog (PAY-01, PAY-06): what is due for the payments
//! typed so far, with cash rounding, and the quick-cash note buttons. Pure functions.

use pos_core::{AppError, Money, RoundingAppliesTo, RoundingRule, cash_round};
use sales::{EnteredPayment, PaymentKind};

/// The most quick-cash buttons the dialog shows (`F1` to `F4`).
const QUICK_CASH_BUTTONS: usize = 4;

/// The amount due and what the cash row should default to.
pub(crate) struct Due {
    pub due: Money,
    pub cash_to_pay: Money,
}

/// Rule SET-06 / PAY-06: rounding on every sale, or only on the part paid in cash.
/// With "cash only", the rest after non-cash payments is rounded when cash pays it.
pub(crate) fn amount_due(
    gross: Money,
    entered: &[EnteredPayment],
    rule: &RoundingRule,
) -> Result<Due, AppError> {
    let mut non_cash = Money::zero();
    let mut has_cash = false;
    for payment in entered {
        if payment.kind == PaymentKind::Cash {
            has_cash = true;
        } else {
            non_cash = non_cash.checked_add(payment.offered)?;
        }
    }
    let rest = gross.checked_sub(non_cash)?.max(Money::zero());
    let rest_in_cash = cash_round(rest, rule)?;
    let due = match rule.applies_to {
        RoundingAppliesTo::AllPayments => cash_round(gross, rule)?,
        RoundingAppliesTo::CashOnly if has_cash => non_cash.checked_add(rest_in_cash)?,
        RoundingAppliesTo::CashOnly => gross,
    };
    Ok(Due {
        due,
        cash_to_pay: rest_in_cash,
    })
}

/// Rule PAY-01: the next useful notes above the amount. Each note size ≥ a tenth of the
/// amount rounds it up. Example: 1,377.00 → 1,400, 1,500, 2,000.
pub(crate) fn quick_cash(amount: Money, denominations: &[Money]) -> Vec<Money> {
    if amount.0 <= 0 {
        return Vec::new();
    }
    let mut amounts: Vec<Money> = Vec::new();
    for note in denominations {
        if note.0 <= 0 || note.0.saturating_mul(10) < amount.0 {
            continue;
        }
        let notes = (amount.0 + note.0 - 1) / note.0;
        let rounded = Money(notes.saturating_mul(note.0));
        if rounded > amount && !amounts.contains(&rounded) {
            amounts.push(rounded);
        }
    }
    amounts.sort();
    amounts.truncate(QUICK_CASH_BUTTONS);
    amounts
}

#[cfg(test)]
mod tests {
    use pos_core::{Id, RoundingMode};

    use super::*;

    fn rule(applies_to: RoundingAppliesTo) -> RoundingRule {
        RoundingRule {
            increment_minor: Money(100),
            mode: RoundingMode::Nearest,
            applies_to,
        }
    }

    fn paid(kind: PaymentKind, offered: i64) -> EnteredPayment {
        EnteredPayment {
            method_id: Id::new(),
            kind,
            offered: Money(offered),
            reference: None,
        }
    }

    #[test]
    fn pay_06_cash_only_rounding_applies_when_cash_pays_the_rest() {
        let cash_only = rule(RoundingAppliesTo::CashOnly);
        let nothing = amount_due(Money(137_750), &[], &cash_only).unwrap();
        assert_eq!(
            (nothing.due, nothing.cash_to_pay),
            (Money(137_750), Money(137_800))
        );
        let cash = amount_due(
            Money(137_750),
            &[paid(PaymentKind::Cash, 140_000)],
            &cash_only,
        )
        .unwrap();
        assert_eq!(cash.due, Money(137_800));
        let card = amount_due(
            Money(137_750),
            &[paid(PaymentKind::Card, 137_750)],
            &cash_only,
        )
        .unwrap();
        assert_eq!(card.due, Money(137_750));
        let split = [
            paid(PaymentKind::Card, 40_000),
            paid(PaymentKind::Cash, 100_000),
        ];
        let both = amount_due(Money(137_750), &split, &cash_only).unwrap();
        assert_eq!(both.due, Money(137_800));
    }

    #[test]
    fn set_06_rounding_for_all_payments_rounds_every_sale() {
        let all = rule(RoundingAppliesTo::AllPayments);
        let card = amount_due(Money(137_750), &[paid(PaymentKind::Card, 1)], &all).unwrap();
        assert_eq!(card.due, Money(137_800));
    }

    #[test]
    fn pay_01_quick_cash_shows_the_next_useful_notes() {
        let notes: Vec<Money> = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000]
            .iter()
            .map(|whole| Money(whole * 100))
            .collect();
        assert_eq!(
            quick_cash(Money(47_000), &notes),
            vec![Money(50_000), Money(60_000), Money(100_000)]
        );
        assert_eq!(
            quick_cash(Money(137_700), &notes),
            vec![Money(140_000), Money(150_000), Money(200_000)]
        );
        assert!(quick_cash(Money(0), &notes).is_empty());
    }
}
