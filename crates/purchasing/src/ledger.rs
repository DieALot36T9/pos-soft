//! PUR-05 The payables ledger: what the shop owes each supplier (ADR-010).
//! Rows are only ever added. Each insert also updates `purchasing_balances` in the
//! same transaction, so the cached balance always equals the sum of the rows.

use chrono::NaiveDate;
use pos_core::{Id, Money};
use rusqlite::{Connection, Transaction, named_params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::sql::{clean_text, date_text, read_id, supplier_facts};
use crate::{PurchasingCtx, PurchasingError};

/// The kind of a ledger row. Positive amounts mean the shop owes more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LedgerKind {
    /// A delivery (GRN): positive.
    Invoice,
    /// Money paid to the supplier: negative.
    Payment,
    /// Goods sent back to the supplier: negative.
    ReturnCredit,
    /// A correction or an opening balance: either sign.
    Adjust,
}

impl LedgerKind {
    /// The value stored in `purchasing_ledger.kind`.
    pub fn as_str(self) -> &'static str {
        match self {
            LedgerKind::Invoice => "invoice",
            LedgerKind::Payment => "payment",
            LedgerKind::ReturnCredit => "return_credit",
            LedgerKind::Adjust => "adjust",
        }
    }

    pub(crate) fn parse(text: &str) -> rusqlite::Result<LedgerKind> {
        match text {
            "invoice" => Ok(LedgerKind::Invoice),
            "payment" => Ok(LedgerKind::Payment),
            "return_credit" => Ok(LedgerKind::ReturnCredit),
            "adjust" => Ok(LedgerKind::Adjust),
            other => Err(rusqlite::Error::InvalidColumnType(
                0,
                format!("kind {other}"),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

/// One new ledger row, before it is saved.
pub(crate) struct LedgerEntry<'a> {
    pub supplier: &'a Id,
    pub amount: Money,
    pub kind: LedgerKind,
    pub method_kind: Option<&'a str>,
    pub reference: Option<&'a str>,
    pub ref_type: Option<&'a str>,
    pub ref_id: Option<&'a Id>,
    pub due_date: Option<NaiveDate>,
}

/// Adds one row to the ledger and updates the supplier's cached balance.
pub(crate) fn append(
    tx: &Transaction,
    entry: &LedgerEntry,
    ctx: &PurchasingCtx,
) -> Result<Id, PurchasingError> {
    let id = Id::new();
    tx.execute(
        "INSERT INTO purchasing_ledger (id, supplier_id, amount_minor, kind, method_kind,
           reference, ref_type, ref_id, due_date, at, business_date, user_id)
         VALUES (:id, :supplier_id, :amount, :kind, :method_kind, :reference, :ref_type,
           :ref_id, :due_date, :at, :business_date, :user_id)",
        named_params! {
            ":id": id.as_str(),
            ":supplier_id": entry.supplier.as_str(),
            ":amount": entry.amount.0,
            ":kind": entry.kind.as_str(),
            ":method_kind": entry.method_kind,
            ":reference": entry.reference,
            ":ref_type": entry.ref_type,
            ":ref_id": entry.ref_id.map(Id::as_str),
            ":due_date": entry.due_date.map(date_text),
            ":at": ctx.at.0,
            ":business_date": ctx.business_date.to_string(),
            ":user_id": ctx.user_id.as_str(),
        },
    )?;
    let old_balance = cached_balance(tx, entry.supplier)?;
    let new_balance = old_balance.checked_add(entry.amount)?;
    tx.execute(
        "INSERT INTO purchasing_balances (supplier_id, balance_minor) VALUES (:supplier, :balance)
         ON CONFLICT (supplier_id) DO UPDATE SET balance_minor = :balance",
        named_params! { ":supplier": entry.supplier.as_str(), ":balance": new_balance.0 },
    )?;
    Ok(id)
}

/// Records money paid to a supplier. `amount` is what was paid (above zero).
/// `method_kind` is how: "cash", "bank", "card", … `app` records cash from the drawer on the shift.
pub fn record_payment(
    tx: &Transaction,
    supplier: &Id,
    amount: Money,
    method_kind: &str,
    reference: Option<&str>,
    ctx: &PurchasingCtx,
) -> Result<Id, PurchasingError> {
    supplier_facts(tx, supplier)?;
    if amount.0 <= 0 {
        return Err(PurchasingError::Invalid {
            field: "amount",
            code: "amount_not_positive",
        });
    }
    let method = clean_text(Some(method_kind)).ok_or(PurchasingError::Invalid {
        field: "method",
        code: "method_missing",
    })?;
    let reference = clean_text(reference);
    let entry = LedgerEntry {
        supplier,
        amount: amount.checked_neg()?,
        kind: LedgerKind::Payment,
        method_kind: Some(&method),
        reference: reference.as_deref(),
        ref_type: None,
        ref_id: None,
        due_date: None,
    };
    append(tx, &entry, ctx)
}

/// Records a correction to what the shop owes, with a reason. Positive raises the
/// balance (for example an opening balance), negative lowers it.
pub fn record_adjustment(
    tx: &Transaction,
    supplier: &Id,
    amount: Money,
    reason: &str,
    ctx: &PurchasingCtx,
) -> Result<Id, PurchasingError> {
    supplier_facts(tx, supplier)?;
    if amount.0 == 0 {
        return Err(PurchasingError::Invalid {
            field: "amount",
            code: "amount_zero",
        });
    }
    let reason = clean_text(Some(reason)).ok_or(PurchasingError::Invalid {
        field: "reason",
        code: "reason_missing",
    })?;
    // A positive adjustment is due straight away, so aging counts it from today.
    let due_date = (amount.0 > 0).then_some(ctx.business_date.0);
    let entry = LedgerEntry {
        supplier,
        amount,
        kind: LedgerKind::Adjust,
        method_kind: None,
        reference: Some(&reason),
        ref_type: None,
        ref_id: None,
        due_date,
    };
    append(tx, &entry, ctx)
}

/// What the shop owes a supplier now: invoices − payments − returns ± adjustments.
pub fn balance(connection: &Connection, supplier: &Id) -> Result<Money, PurchasingError> {
    supplier_facts(connection, supplier)?;
    cached_balance(connection, supplier)
}

/// Suppliers whose cached balance differs from the sum of their ledger rows.
/// Empty means everything matches (ADR-010 "Verify data").
pub fn unbalanced_suppliers(connection: &Connection) -> Result<Vec<Id>, PurchasingError> {
    let mut statement = connection.prepare(
        "SELECT s.id AS supplier_id FROM purchasing_suppliers s
         LEFT JOIN purchasing_balances b ON b.supplier_id = s.id
         LEFT JOIN (SELECT supplier_id, SUM(amount_minor) AS total FROM purchasing_ledger
                    GROUP BY supplier_id) l ON l.supplier_id = s.id
         WHERE COALESCE(b.balance_minor, 0) <> COALESCE(l.total, 0)
         ORDER BY s.id",
    )?;
    let rows = statement.query_map([], |row| read_id(row, "supplier_id"))?;
    let mut suppliers = Vec::new();
    for supplier in rows {
        suppliers.push(supplier?);
    }
    Ok(suppliers)
}

fn cached_balance(connection: &Connection, supplier: &Id) -> Result<Money, PurchasingError> {
    let balance: Option<i64> = connection.query_row(
        "SELECT (SELECT balance_minor FROM purchasing_balances WHERE supplier_id = :supplier)",
        named_params! { ":supplier": supplier.as_str() },
        |row| row.get(0),
    )?;
    Ok(Money(balance.unwrap_or(0)))
}
