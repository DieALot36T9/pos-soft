//! PUR-03 Receive goods (GRN), from a purchase order or directly.
//! This saves the document, updates the order and adds the invoice to the ledger.
//! It does not change stock or product cost: `app` calls inventory and catalog for that.

use chrono::{Days, NaiveDate};
use pos_core::{Id, Money, Qty};
use rusqlite::{Transaction, named_params};

use crate::ledger::{LedgerEntry, LedgerKind, append};
use crate::orders::{OrderStatus, add_received, order_header, refresh_status};
use crate::sql::{
    check_line, clean_text, date_text, lines_value, next_number, require_active_supplier,
    require_lines,
};
use crate::{PurchasingCtx, PurchasingError};

/// One received line. Name is copied from the catalog by `app`. When the product tracks
/// expiry, `app` checks batch and expiry are filled in and passes the inventory batch id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGrnLine {
    /// The purchase order line this fills, when receiving against an order.
    pub order_line_id: Option<Id>,
    pub product_id: Id,
    pub product_name: String,
    pub qty: Qty,
    /// Cost for one unit, as on the supplier's invoice.
    pub unit_cost: Money,
    pub batch_no: Option<String>,
    pub expiry_date: Option<NaiveDate>,
    pub inventory_batch_id: Option<Id>,
}

/// A delivery as the user enters it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGrn {
    pub supplier_id: Id,
    /// The purchase order it fills, if any.
    pub order_id: Option<Id>,
    pub supplier_invoice_no: Option<String>,
    /// The total printed on the supplier's invoice. The balance goes up by this amount.
    pub invoice_total: Money,
    pub notes: Option<String>,
    pub lines: Vec<NewGrnLine>,
}

/// What `record_grn` saved, with the warnings the screen should show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrnRef {
    pub id: Id,
    /// Example: "GRN-000001".
    pub number: String,
    /// Sum of round(unit cost × quantity) over the lines.
    pub lines_total: Money,
    pub invoice_total: Money,
    /// True when the invoice total differs from the line total (show a warning).
    pub total_differs: bool,
    /// Order lines that now have more received than ordered (allowed, but show a warning).
    pub over_received: Vec<Id>,
    /// The order's new status, when the GRN filled an order.
    pub order_status: Option<OrderStatus>,
}

/// Saves a GRN with the next GRN number, adds the received quantities to the order
/// and sets its status, and adds an `invoice` row to the payables ledger.
pub fn record_grn(
    tx: &Transaction,
    grn: &NewGrn,
    ctx: &PurchasingCtx,
) -> Result<GrnRef, PurchasingError> {
    let supplier = require_active_supplier(tx, &grn.supplier_id)?;
    check_grn(tx, grn)?;
    let priced: Vec<(Qty, Money)> = grn
        .lines
        .iter()
        .map(|line| (line.qty, line.unit_cost))
        .collect();
    let lines_total = lines_value(&priced)?;

    let id = Id::new();
    let number = next_number(tx, "grn", "GRN")?;
    insert_grn(tx, &id, &number, grn, lines_total, ctx)?;

    let mut over_received = Vec::new();
    for line in &grn.lines {
        let (Some(order), Some(order_line)) = (&grn.order_id, &line.order_line_id) else {
            continue;
        };
        if add_received(tx, order, order_line, line.qty)? {
            over_received.push(order_line.clone());
        }
    }
    let order_status = match &grn.order_id {
        Some(order) => Some(refresh_status(tx, order)?),
        None => None,
    };

    // Rule PUR-05: the invoice is due after the supplier's payment terms.
    let due_date = ctx
        .business_date
        .0
        .checked_add_days(Days::new(u64::from(supplier.payment_terms_days)))
        .ok_or_else(|| pos_core::AppError::internal("grn due date out of range"))?;
    let invoice_no = clean_text(grn.supplier_invoice_no.as_deref());
    let entry = LedgerEntry {
        supplier: &grn.supplier_id,
        amount: grn.invoice_total,
        kind: LedgerKind::Invoice,
        method_kind: None,
        reference: invoice_no.as_deref(),
        ref_type: Some("grn"),
        ref_id: Some(&id),
        due_date: Some(due_date),
    };
    append(tx, &entry, ctx)?;

    Ok(GrnRef {
        id,
        number,
        lines_total,
        invoice_total: grn.invoice_total,
        total_differs: lines_total != grn.invoice_total,
        over_received,
        order_status,
    })
}

/// Checks lines, totals and (when given) that the order is this supplier's and still open.
fn check_grn(tx: &Transaction, grn: &NewGrn) -> Result<(), PurchasingError> {
    require_lines(grn.lines.len())?;
    for line in &grn.lines {
        check_line(line.qty, line.unit_cost)?;
    }
    if grn.invoice_total.0 < 0 {
        return Err(PurchasingError::Invalid {
            field: "invoice_total",
            code: "invoice_total_negative",
        });
    }
    let Some(order) = &grn.order_id else {
        if grn.lines.iter().any(|line| line.order_line_id.is_some()) {
            return Err(PurchasingError::Invalid {
                field: "order_id",
                code: "order_missing",
            });
        }
        return Ok(());
    };
    let (order_supplier, status) = order_header(tx, order)?;
    if order_supplier != grn.supplier_id {
        return Err(PurchasingError::Invalid {
            field: "order_id",
            code: "order_other_supplier",
        });
    }
    if matches!(status, OrderStatus::Cancelled | OrderStatus::Received) {
        return Err(PurchasingError::Conflict {
            code: "order_closed",
        });
    }
    Ok(())
}

fn insert_grn(
    tx: &Transaction,
    id: &Id,
    number: &str,
    grn: &NewGrn,
    lines_total: Money,
    ctx: &PurchasingCtx,
) -> Result<(), PurchasingError> {
    tx.execute(
        "INSERT INTO purchasing_grns (id, number, supplier_id, order_id, supplier_invoice_no,
           invoice_total_minor, lines_total_minor, received_at, business_date, user_id, notes)
         VALUES (:id, :number, :supplier_id, :order_id, :invoice_no, :invoice_total,
           :lines_total, :at, :business_date, :user_id, :notes)",
        named_params! {
            ":id": id.as_str(),
            ":number": number,
            ":supplier_id": grn.supplier_id.as_str(),
            ":order_id": grn.order_id.as_ref().map(Id::as_str),
            ":invoice_no": clean_text(grn.supplier_invoice_no.as_deref()),
            ":invoice_total": grn.invoice_total.0,
            ":lines_total": lines_total.0,
            ":at": ctx.at.0,
            ":business_date": ctx.business_date.to_string(),
            ":user_id": ctx.user_id.as_str(),
            ":notes": clean_text(grn.notes.as_deref()),
        },
    )?;
    let mut statement = tx.prepare(
        "INSERT INTO purchasing_grn_lines (id, grn_id, order_line_id, product_id, product_name,
           qty_milli, unit_cost_minor, batch_no, expiry_date, inventory_batch_id)
         VALUES (:id, :grn_id, :order_line_id, :product_id, :product_name, :qty, :unit_cost,
           :batch_no, :expiry_date, :inventory_batch_id)",
    )?;
    for line in &grn.lines {
        statement.execute(named_params! {
            ":id": Id::new().as_str(),
            ":grn_id": id.as_str(),
            ":order_line_id": line.order_line_id.as_ref().map(Id::as_str),
            ":product_id": line.product_id.as_str(),
            ":product_name": line.product_name,
            ":qty": line.qty.0,
            ":unit_cost": line.unit_cost.0,
            ":batch_no": clean_text(line.batch_no.as_deref()),
            ":expiry_date": line.expiry_date.map(date_text),
            ":inventory_batch_id": line.inventory_batch_id.as_ref().map(Id::as_str),
        })?;
    }
    Ok(())
}
