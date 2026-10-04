//! Voids: every points and credit row of a document gets an exact opposite row.

use pos_core::{Id, Money};
use rusqlite::{Transaction, named_params};

use crate::credit::{insert_credit_row, refresh_credit_cache};
use crate::points::{add_to_balance, insert_points_row};
use crate::rows::{load_balance, parse_id, save_points};
use crate::{CreditKind, CustomersError, LedgerCtx, LedgerRef, PointsKind};

/// The reason written on points rows made by a void. It marks the document as reversed.
const VOID_REASON: &str = "void";

/// Undoes a voided document: writes the opposite of each of its points and credit rows
/// and updates the balances. Refused if the document was already reversed. A document
/// with no customer rows (e.g. a cash sale with no customer) is fine and changes nothing.
pub fn reverse_ref(
    tx: &Transaction,
    ref_type: &str,
    ref_id: &Id,
    ctx: &LedgerCtx,
) -> Result<(), CustomersError> {
    if was_reversed(tx, ref_type, ref_id)? {
        return Err(CustomersError::AlreadyReversed {
            ref_type: ref_type.to_string(),
            ref_id: ref_id.to_string(),
        });
    }
    let document = LedgerRef {
        ref_type: ref_type.to_string(),
        ref_id: ref_id.clone(),
    };
    for (customer, delta) in rows_of(tx, "customers_points_ledger", "points_delta", &document)? {
        let opposite = -delta;
        let reason = Some(VOID_REASON);
        insert_points_row(
            tx,
            &customer,
            opposite,
            PointsKind::Reverse,
            Some(&document),
            reason,
            ctx,
        )?;
        let balance = load_balance(tx, &customer)?.points;
        save_points(tx, &customer, add_to_balance(balance, opposite)?)?;
    }
    for (customer, amount) in rows_of(tx, "customers_credit_ledger", "amount_minor", &document)? {
        let opposite = Money(amount).checked_neg()?;
        let kind = CreditKind::VoidReversal;
        insert_credit_row(tx, &customer, opposite, kind, None, Some(&document), ctx)?;
        let balance = load_balance(tx, &customer)?.credit.checked_add(opposite)?;
        refresh_credit_cache(tx, &customer, balance)?;
    }
    Ok(())
}

fn was_reversed(tx: &Transaction, ref_type: &str, ref_id: &Id) -> Result<bool, CustomersError> {
    let reversed = tx.query_row(
        "SELECT EXISTS (SELECT 1 FROM customers_points_ledger
                        WHERE ref_type = :ref_type AND ref_id = :ref_id
                          AND kind = 'reverse' AND reason = :void)
             OR EXISTS (SELECT 1 FROM customers_credit_ledger
                        WHERE ref_type = :ref_type AND ref_id = :ref_id
                          AND kind = 'void_reversal')",
        named_params! { ":ref_type": ref_type, ":ref_id": ref_id.as_str(), ":void": VOID_REASON },
        |row| row.get(0),
    )?;
    Ok(reversed)
}

/// The (customer, amount) of every row of one ledger that belongs to the document.
fn rows_of(
    tx: &Transaction,
    table: &str,
    amount_column: &str,
    document: &LedgerRef,
) -> Result<Vec<(Id, i64)>, CustomersError> {
    // `table` and `amount_column` are always one of the fixed names above, never user text.
    let sql = format!(
        "SELECT customer_id, {amount_column} FROM {table}
         WHERE ref_type = :ref_type AND ref_id = :ref_id ORDER BY rowid"
    );
    let mut statement = tx.prepare(&sql)?;
    let rows = statement.query_map(
        named_params! { ":ref_type": document.ref_type, ":ref_id": document.ref_id.as_str() },
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )?;
    let mut found = Vec::new();
    for row in rows {
        let (customer, amount) = row?;
        found.push((parse_id(&customer)?, amount));
    }
    Ok(found)
}
