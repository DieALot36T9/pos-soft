//! Working out payments and change from what the cashier entered (PAY-01, PAY-03).
//! A pure function: no database. The payment screen and `complete` both rely on it.

use pos_core::{Id, Money};

use crate::{NewPayment, PaymentKind, SalesError};

/// One payment as the cashier typed it on the payment screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnteredPayment {
    pub method_id: Id,
    pub kind: PaymentKind,
    /// For cash: the notes handed over. For other kinds: the amount charged.
    pub offered: Money,
    pub reference: Option<String>,
}

/// The payments ready for `complete`, with what is still due and the change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settlement {
    pub payments: Vec<NewPayment>,
    /// Σ payment amounts. Never more than the total due.
    pub paid: Money,
    /// What is still to pay. The sale completes only when this is zero.
    pub remaining: Money,
    /// Cash to give back.
    pub change: Money,
}

/// Applies the entered payments to the amount due, in the order entered.
/// Only cash may be more than what's left; the extra is change (PAY-03).
/// Example (E13): due 1,000.00, wallet 400.00, then cash 700.00 → change 100.00.
pub fn settle_payments(
    total_due: Money,
    entered: &[EnteredPayment],
) -> Result<Settlement, SalesError> {
    let mut payments = Vec::new();
    let mut remaining = total_due;
    let mut change = Money::zero();
    for payment in entered {
        if payment.offered.0 <= 0 {
            return Err(SalesError::invalid("payment_amount_invalid"));
        }
        if remaining.0 <= 0 {
            return Err(SalesError::invalid("payment_not_needed"));
        }
        let applied = if payment.kind == PaymentKind::Cash {
            payment.offered.min(remaining)
        } else if payment.offered > remaining {
            return Err(SalesError::invalid_with(
                "payment_over_remaining",
                "remaining_minor",
                remaining.0.to_string(),
            ));
        } else {
            payment.offered
        };
        let tendered = if payment.kind == PaymentKind::Cash {
            change = change.checked_add(payment.offered.checked_sub(applied)?)?;
            Some(payment.offered)
        } else {
            None
        };
        remaining = remaining.checked_sub(applied)?;
        payments.push(NewPayment {
            method_id: payment.method_id.clone(),
            amount: applied,
            tendered,
            reference: payment.reference.clone(),
        });
    }
    let paid = total_due.checked_sub(remaining)?;
    Ok(Settlement {
        payments,
        paid,
        remaining,
        change,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cash(offered: i64) -> EnteredPayment {
        entered(PaymentKind::Cash, offered)
    }

    fn entered(kind: PaymentKind, offered: i64) -> EnteredPayment {
        EnteredPayment {
            method_id: Id::new(),
            kind,
            offered: Money(offered),
            reference: None,
        }
    }

    #[test]
    fn e12_pay_01_cash_change() {
        // Due 470.00, cash 500.00 → change 30.00.
        let settled = settle_payments(Money(47_000), &[cash(50_000)]).unwrap();
        assert_eq!(settled.change, Money(3_000));
        assert_eq!(settled.remaining, Money(0));
        assert_eq!(settled.payments[0].amount, Money(47_000));
        assert_eq!(settled.payments[0].tendered, Some(Money(50_000)));
    }

    #[test]
    fn e13_pay_03_split_wallet_then_cash() {
        // Due 1,000.00: wallet 400.00, cash 700.00 → change 100.00.
        let settled = settle_payments(
            Money(100_000),
            &[entered(PaymentKind::Wallet, 40_000), cash(70_000)],
        )
        .unwrap();
        assert_eq!(settled.change, Money(10_000));
        assert_eq!(settled.paid, Money(100_000));
        assert_eq!(settled.payments[0].amount, Money(40_000));
        assert_eq!(settled.payments[0].tendered, None);
        assert_eq!(settled.payments[1].amount, Money(60_000));
    }

    #[test]
    fn pay_03_non_cash_over_the_remaining_amount_is_refused() {
        let error = settle_payments(
            Money(100_000),
            &[cash(50_000), entered(PaymentKind::Card, 60_000)],
        )
        .unwrap_err();
        assert_eq!(error.code(), Some("payment_over_remaining"));
    }

    #[test]
    fn pay_03_short_payment_leaves_an_amount_remaining() {
        let settled =
            settle_payments(Money(100_000), &[entered(PaymentKind::Card, 30_000)]).unwrap();
        assert_eq!(settled.remaining, Money(70_000));
        assert_eq!(settled.change, Money(0));
    }

    #[test]
    fn pay_03_a_payment_after_everything_is_paid_is_refused() {
        let error = settle_payments(Money(1_000), &[cash(1_000), cash(500)]).unwrap_err();
        assert_eq!(error.code(), Some("payment_not_needed"));
    }

    #[test]
    fn pay_03_zero_or_negative_payments_are_refused() {
        let error = settle_payments(Money(1_000), &[cash(0)]).unwrap_err();
        assert_eq!(error.code(), Some("payment_amount_invalid"));
    }

    #[test]
    fn pay_03_two_cash_payments_add_up() {
        let settled = settle_payments(Money(1_000), &[cash(300), cash(1_000)]).unwrap();
        assert_eq!(settled.payments[1].amount, Money(700));
        assert_eq!(settled.change, Money(300));
    }
}
