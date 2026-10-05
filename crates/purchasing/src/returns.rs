//! PUR-04 Supplier returns: goods sent back, which lowers what the shop owes.
//! It does not change stock: `app` calls inventory (`supplier_return`) for that.

use pos_core::{Id, Money, Qty};
use rusqlite::{OptionalExtension, Transaction, named_params};

use crate::ledger::{LedgerEntry, LedgerKind, append};
use crate::sql::{
    check_line, clean_text, lines_value, next_number, read_id, require_lines, supplier_facts,
};
use crate::{PurchasingCtx, PurchasingError};

/// One line sent back. `unit_cost` is the value per unit (default: current cost, from `app`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewReturnLine {
    pub product_id: Id,
    pub product_name: String,
    pub qty: Qty,
    pub unit_cost: Money,
    pub inventory_batch_id: Option<Id>,
}

/// A supplier return as the user enters it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSupplierReturn {
    pub supplier_id: Id,
    /// The GRN the goods came in on, if the user picked one.
    pub grn_id: Option<Id>,
    pub reason: Option<String>,
    pub lines: Vec<NewReturnLine>,
}

/// What `record_return` saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnRef {
    pub id: Id,
    /// Example: "SR-000001".
    pub number: String,
    /// The value sent back. The supplier's balance went down by this amount.
    pub total: Money,
}

/// Saves a supplier return with the next SR number and adds a `return_credit` row
/// to the payables ledger. Returns to archived suppliers are allowed.
pub fn record_return(
    tx: &Transaction,
    sent_back: &NewSupplierReturn,
    ctx: &PurchasingCtx,
) -> Result<ReturnRef, PurchasingError> {
    supplier_facts(tx, &sent_back.supplier_id)?;
    require_lines(sent_back.lines.len())?;
    for line in &sent_back.lines {
        check_line(line.qty, line.unit_cost)?;
    }
    if let Some(grn) = &sent_back.grn_id {
        check_grn_supplier(tx, grn, &sent_back.supplier_id)?;
    }
    let priced: Vec<(Qty, Money)> = sent_back
        .lines
        .iter()
        .map(|line| (line.qty, line.unit_cost))
        .collect();
    let total = lines_value(&priced)?;

    let id = Id::new();
    let number = next_number(tx, "sr", "SR")?;
    insert_return(tx, &id, &number, sent_back, total, ctx)?;

    let entry = LedgerEntry {
        supplier: &sent_back.supplier_id,
        amount: total.checked_neg()?,
        kind: LedgerKind::ReturnCredit,
        method_kind: None,
        reference: Some(&number),
        ref_type: Some("supplier_return"),
        ref_id: Some(&id),
        due_date: None,
    };
    append(tx, &entry, ctx)?;
    Ok(ReturnRef { id, number, total })
}

fn check_grn_supplier(tx: &Transaction, grn: &Id, supplier: &Id) -> Result<(), PurchasingError> {
    let grn_supplier = tx
        .query_row(
            "SELECT supplier_id FROM purchasing_grns WHERE id = :id",
            named_params! { ":id": grn.as_str() },
            |row| read_id(row, "supplier_id"),
        )
        .optional()?
        .ok_or_else(|| PurchasingError::not_found("grn", grn))?;
    if grn_supplier != *supplier {
        return Err(PurchasingError::Invalid {
            field: "grn_id",
            code: "grn_other_supplier",
        });
    }
    Ok(())
}

fn insert_return(
    tx: &Transaction,
    id: &Id,
    number: &str,
    sent_back: &NewSupplierReturn,
    total: Money,
    ctx: &PurchasingCtx,
) -> Result<(), PurchasingError> {
    tx.execute(
        "INSERT INTO purchasing_returns (id, number, supplier_id, grn_id, total_minor, reason,
           at, business_date, user_id)
         VALUES (:id, :number, :supplier_id, :grn_id, :total, :reason, :at, :business_date,
           :user_id)",
        named_params! {
            ":id": id.as_str(),
            ":number": number,
            ":supplier_id": sent_back.supplier_id.as_str(),
            ":grn_id": sent_back.grn_id.as_ref().map(Id::as_str),
            ":total": total.0,
            ":reason": clean_text(sent_back.reason.as_deref()),
            ":at": ctx.at.0,
            ":business_date": ctx.business_date.to_string(),
            ":user_id": ctx.user_id.as_str(),
        },
    )?;
    let mut statement = tx.prepare(
        "INSERT INTO purchasing_return_lines (id, return_id, product_id, product_name, qty_milli,
           unit_cost_minor, inventory_batch_id)
         VALUES (:id, :return_id, :product_id, :product_name, :qty, :unit_cost, :batch)",
    )?;
    for line in &sent_back.lines {
        statement.execute(named_params! {
            ":id": Id::new().as_str(),
            ":return_id": id.as_str(),
            ":product_id": line.product_id.as_str(),
            ":product_name": line.product_name,
            ":qty": line.qty.0,
            ":unit_cost": line.unit_cost.0,
            ":batch": line.inventory_batch_id.as_ref().map(Id::as_str),
        })?;
    }
    Ok(())
}
