//! The credit ledger: debt (baki) and store credit (rules CUS-05, CUS-06, PAY-04).
//! A positive balance means the customer owes the shop; a negative one is store credit.

use std::collections::VecDeque;

use pos_core::{Id, Money, Timestamp};
use rusqlite::{Transaction, named_params};

use crate::rows::{load_balance, save_credit};
use crate::{CreditKind, CreditSplit, CustomersError, LedgerCtx, LedgerRef, PaymentMethod};

/// Adds a credit ledger row and gives back the customer's new credit balance.
/// Store credit can't be overspent, and a payment can't be more than the debt.
/// The credit limit is checked by `app` before this (rule PAY-04).
pub fn add_credit(
    tx: &Transaction,
    customer: &Id,
    amount: Money,
    kind: CreditKind,
    method: Option<&PaymentMethod>,
    ref_: Option<&LedgerRef>,
    ctx: &LedgerCtx,
) -> Result<Money, CustomersError> {
    kind.check_sign(amount.0)?;
    let balance = load_balance(tx, customer)?.credit;
    let new_balance = balance.checked_add(amount)?;
    if kind == CreditKind::StoreCreditUsed && new_balance > Money::zero() {
        let available = balance.checked_neg()?.max(Money::zero());
        return Err(CustomersError::StoreCreditNotEnough { available });
    }
    if kind == CreditKind::Payment && new_balance < Money::zero() {
        let owed = balance.max(Money::zero());
        return Err(CustomersError::PaymentMoreThanDebt { owed });
    }
    insert_credit_row(tx, customer, amount, kind, method, ref_, ctx)?;
    refresh_credit_cache(tx, customer, new_balance)?;
    Ok(new_balance)
}

/// Pays part of a sale with "Customer credit": store credit first, then new debt (CUS-06).
/// Writes a `store_credit_used` row, a `credit_sale` row, or both.
pub fn pay_with_credit(
    tx: &Transaction,
    customer: &Id,
    amount: Money,
    ref_: Option<&LedgerRef>,
    ctx: &LedgerCtx,
) -> Result<CreditSplit, CustomersError> {
    CreditKind::CreditSale.check_sign(amount.0)?;
    let balance = load_balance(tx, customer)?.credit;
    let split = split_credit_payment(balance, amount)?;
    if split.store_credit_used > Money::zero() {
        let kind = CreditKind::StoreCreditUsed;
        add_credit(tx, customer, split.store_credit_used, kind, None, ref_, ctx)?;
    }
    if split.new_debt > Money::zero() {
        let kind = CreditKind::CreditSale;
        add_credit(tx, customer, split.new_debt, kind, None, ref_, ctx)?;
    }
    Ok(split)
}

/// How much of a "Customer credit" payment comes from store credit and how much is new debt.
/// `app` uses `new_debt` for the credit limit check. Example: −300.00 balance, pay 500.00
/// → 300.00 store credit used and 200.00 new debt.
pub fn split_credit_payment(balance: Money, amount: Money) -> Result<CreditSplit, CustomersError> {
    let store_credit = balance.checked_neg()?.max(Money::zero());
    let store_credit_used = store_credit.min(amount.max(Money::zero()));
    let new_debt = amount.checked_sub(store_credit_used)?;
    Ok(CreditSplit {
        store_credit_used,
        new_debt,
    })
}

/// When the oldest debt that is still unpaid was made, for the aging report (CUS-05).
/// Rows are (time, amount) in ledger order. Payments pay off the oldest debt first;
/// money paid beyond the debt is store credit and is used up by the next debts.
/// Nothing when nothing is owed.
pub fn oldest_unpaid_at(rows: &[(Timestamp, Money)]) -> Option<Timestamp> {
    let mut unpaid: VecDeque<(Timestamp, i128)> = VecDeque::new();
    let mut store_credit: i128 = 0;
    for (at, amount) in rows {
        let amount = i128::from(amount.0);
        if amount > 0 {
            let from_store_credit = store_credit.min(amount);
            store_credit -= from_store_credit;
            if amount > from_store_credit {
                unpaid.push_back((*at, amount - from_store_credit));
            }
        } else {
            let mut to_pay = -amount;
            while to_pay > 0 {
                let Some(oldest) = unpaid.front_mut() else {
                    break;
                };
                let paid = oldest.1.min(to_pay);
                oldest.1 -= paid;
                to_pay -= paid;
                if oldest.1 == 0 {
                    unpaid.pop_front();
                }
            }
            store_credit += to_pay;
        }
    }
    unpaid.front().map(|(at, _)| *at)
}

/// Saves the new balance and works out the oldest unpaid debt again from the ledger.
pub(crate) fn refresh_credit_cache(
    tx: &Transaction,
    customer: &Id,
    balance: Money,
) -> Result<(), CustomersError> {
    let mut statement = tx.prepare(
        "SELECT at, amount_minor FROM customers_credit_ledger
         WHERE customer_id = :customer_id ORDER BY at, rowid",
    )?;
    let rows = statement.query_map(named_params! { ":customer_id": customer.as_str() }, |row| {
        Ok((Timestamp(row.get(0)?), Money(row.get(1)?)))
    })?;
    let mut ledger = Vec::new();
    for row in rows {
        ledger.push(row?);
    }
    save_credit(tx, customer, balance, oldest_unpaid_at(&ledger))
}

/// Writes one row to the append-only credit ledger. Callers update the balance.
pub(crate) fn insert_credit_row(
    tx: &Transaction,
    customer: &Id,
    amount: Money,
    kind: CreditKind,
    method: Option<&PaymentMethod>,
    ref_: Option<&LedgerRef>,
    ctx: &LedgerCtx,
) -> Result<(), CustomersError> {
    tx.execute(
        "INSERT INTO customers_credit_ledger (id, customer_id, amount_minor, kind, method_kind,
           reference, ref_type, ref_id, at, business_date, user_id)
         VALUES (:id, :customer_id, :amount, :kind, :method_kind,
           :reference, :ref_type, :ref_id, :at, :business_date, :user_id)",
        named_params! {
            ":id": Id::new().as_str(),
            ":customer_id": customer.as_str(),
            ":amount": amount.0,
            ":kind": kind.as_db(),
            ":method_kind": method.map(|m| m.method_kind.as_str()),
            ":reference": method.and_then(|m| m.reference.as_deref()),
            ":ref_type": ref_.map(|r| r.ref_type.as_str()),
            ":ref_id": ref_.map(|r| r.ref_id.as_str()),
            ":at": ctx.at.0,
            ":business_date": ctx.business_date.to_string(),
            ":user_id": ctx.user_id.as_ref().map(Id::as_str),
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cus_06_store_credit_is_split_off_first() {
        let split = split_credit_payment(Money(-30_000), Money(50_000)).unwrap();
        assert_eq!(split.store_credit_used, Money(30_000));
        assert_eq!(split.new_debt, Money(20_000));

        let all_store_credit = split_credit_payment(Money(-30_000), Money(10_000)).unwrap();
        assert_eq!(all_store_credit.store_credit_used, Money(10_000));
        assert_eq!(all_store_credit.new_debt, Money::zero());

        let already_owes = split_credit_payment(Money(5_000), Money(10_000)).unwrap();
        assert_eq!(already_owes.store_credit_used, Money::zero());
        assert_eq!(already_owes.new_debt, Money(10_000));
    }

    #[test]
    fn cus_05_store_credit_pays_the_next_debt_before_it_counts_as_unpaid() {
        let rows = [
            (Timestamp(1), Money(-500)), // store credit 5.00
            (Timestamp(2), Money(300)),  // fully covered by store credit
            (Timestamp(3), Money(400)),  // 2.00 covered, 2.00 unpaid
        ];
        assert_eq!(oldest_unpaid_at(&rows), Some(Timestamp(3)));
        assert_eq!(oldest_unpaid_at(&rows[..2]), None);
    }

    #[test]
    fn cus_05_part_payments_keep_the_oldest_debt_until_it_is_cleared() {
        let rows = [
            (Timestamp(1), Money(1_000)),
            (Timestamp(2), Money(1_000)),
            (Timestamp(3), Money(-999)),
        ];
        assert_eq!(oldest_unpaid_at(&rows), Some(Timestamp(1)));
        assert_eq!(oldest_unpaid_at(&[]), None);
    }
}
