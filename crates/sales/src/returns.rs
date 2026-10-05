//! Returns (RET-01 to RET-03, RET-05): working out refunds and saving the return.

use std::collections::HashSet;

use pos_core::{AppError, Bps, BusinessDate, Id, Money, Qty, Timestamp, mul_qty};
use rusqlite::{Connection, Transaction, named_params};

use crate::check::check_refund_payments;
use crate::complete::insert_payment;
use crate::numbers::{DocKind, format_number, take_next_number};
use crate::refund::{LineAmounts, split_refund};
use crate::sale_read::{get_sale_status, sale_lines};
use crate::{NewPayment, SaleLine, SaleStatus, SalesError, payment_methods};

/// A quantity to return from one sale line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnQty {
    pub sale_line_id: Id,
    pub qty: Qty,
}

/// What returning part of a sale line refunds, worked out from the stored line (RET-02).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineRefund {
    pub sale_line_id: Id,
    pub product_id: Id,
    pub name: String,
    pub qty: Qty,
    pub amounts: LineAmounts,
    pub tax_rate_id: Id,
    pub tax_bps: Bps,
    pub unit_cost: Money,
}

/// A line of a return with a receipt: which sale line, how much, and whether it goes back on the shelf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnedSaleLine {
    pub sale_line_id: Id,
    pub qty: Qty,
    /// True: back into stock. False: written off as damaged.
    pub restock: bool,
}

/// A line of a return without a receipt (RET-03), priced by the workflow at today's price.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnedProduct {
    pub product_id: Id,
    pub name: String,
    pub qty: Qty,
    pub net: Money,
    pub tax: Money,
    pub gross: Money,
    pub tax_rate_id: Id,
    pub tax_bps: Bps,
    pub unit_cost: Money,
    pub restock: bool,
}

/// One line of a new return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewReturnLine {
    FromSale(ReturnedSaleLine),
    WithoutReceipt(ReturnedProduct),
}

/// Everything `record_return` needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewReturn {
    /// The original sale. `None` for a return without a receipt; then every line is `WithoutReceipt`.
    pub sale_id: Option<Id>,
    pub till_id: Id,
    /// Used in the return number. Example: "T1" gives "T1-R000045".
    pub till_code: String,
    pub shift_id: Id,
    pub user_id: Id,
    pub approved_by: Option<Id>,
    pub business_date: BusinessDate,
    pub at: Timestamp,
    pub customer_id: Option<Id>,
    pub reason: Option<String>,
    pub lines: Vec<NewReturnLine>,
    /// Cash rounding of the refund, worked out by the workflow like a sale's. total = gross + rounding.
    pub rounding: Money,
    /// How the refund is paid out. Must add up to the total exactly.
    pub payments: Vec<NewPayment>,
}

/// The id and number of a return that was just saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnRef {
    pub id: Id,
    /// Example: "T1-R000045".
    pub return_no: String,
}

/// The refund for each requested line, without saving anything. The return screen shows
/// these; `record_return` works them out again inside its transaction.
/// Refused if the sale is void, a line is not on the sale, or a quantity is more than
/// is left to return (`return_qty_too_large`, with `max_milli`).
pub fn refund_amounts(
    conn: &Connection,
    sale_id: &Id,
    wanted: &[ReturnQty],
) -> Result<Vec<LineRefund>, SalesError> {
    if get_sale_status(conn, sale_id)? == SaleStatus::Voided {
        return Err(SalesError::conflict("sale_voided"));
    }
    let lines = sale_lines(conn, sale_id)?;
    let mut seen = HashSet::new();
    let mut refunds = Vec::new();
    for request in wanted {
        if !seen.insert(request.sale_line_id.clone()) {
            return Err(SalesError::invalid("return_line_repeated"));
        }
        let Some(line) = lines.iter().find(|line| line.id == request.sale_line_id) else {
            return Err(SalesError::not_found(
                "sale_line",
                request.sale_line_id.as_str(),
            ));
        };
        refunds.push(refund_for_line(conn, line, request.qty)?);
    }
    Ok(refunds)
}

/// The refund for returning `qty` of one stored sale line, after earlier returns.
fn refund_for_line(conn: &Connection, line: &SaleLine, qty: Qty) -> Result<LineRefund, SalesError> {
    if qty.0 <= 0 {
        return Err(SalesError::invalid("qty_not_positive"));
    }
    let (returned_before, refunded_before) = returned_so_far(conn, &line.id)?;
    let left = line.snapshot.qty.0 - returned_before.0;
    if qty.0 > left {
        return Err(SalesError::invalid_with(
            "return_qty_too_large",
            "max_milli",
            left.to_string(),
        ));
    }
    let stored = LineAmounts {
        net: line.snapshot.net,
        tax: line.snapshot.tax,
        gross: line.snapshot.gross,
        cost: line.cost,
    };
    let amounts = split_refund(
        stored,
        line.snapshot.qty,
        returned_before,
        refunded_before,
        qty,
    )?;
    Ok(LineRefund {
        sale_line_id: line.id.clone(),
        product_id: line.snapshot.product_id.clone(),
        name: line.snapshot.name.clone(),
        qty,
        amounts,
        tax_rate_id: line.snapshot.tax_rate_id.clone(),
        tax_bps: line.snapshot.tax_bps,
        unit_cost: line.snapshot.unit_cost,
    })
}

/// Saves a return with the next return number for the till (RET-01, RET-03, RET-06).
/// Lines from a sale are refunded from the stored sale line, never from what the caller
/// says. The refund payments must add up to gross + rounding exactly.
pub fn record_return(tx: &Transaction, new_return: NewReturn) -> Result<ReturnRef, SalesError> {
    if new_return.lines.is_empty() {
        return Err(SalesError::invalid("return_empty"));
    }
    if new_return.till_code.trim().is_empty() {
        return Err(SalesError::invalid("till_code_missing"));
    }
    let lines = priced_lines(tx, &new_return)?;
    let mut sum = LineAmounts::default();
    for line in &lines {
        sum = add_amounts(sum, line.amounts)?;
    }
    let total = sum.gross.checked_add(new_return.rounding)?;
    if total.0 < 0 {
        return Err(SalesError::TotalsDontAddUp(
            "the refund total is negative".into(),
        ));
    }
    check_refund_payments(total, &new_return.payments, &payment_methods(tx)?)?;

    let number = take_next_number(tx, &new_return.till_id, DocKind::Return)?;
    let return_ref = ReturnRef {
        id: Id::new(),
        return_no: format_number(new_return.till_code.trim(), DocKind::Return, number),
    };
    insert_header(tx, &return_ref, &new_return, sum, total)?;
    for line in &lines {
        insert_line(tx, &return_ref.id, line)?;
    }
    for payment in &new_return.payments {
        insert_payment(tx, "return", &return_ref.id, payment, new_return.at)?;
    }
    Ok(return_ref)
}

/// A return line with its refund worked out, ready to save.
struct PricedLine {
    /// `None` for a return without a receipt.
    sale_line_id: Option<Id>,
    product_id: Id,
    name: String,
    qty: Qty,
    amounts: LineAmounts,
    tax_rate_id: Id,
    tax_bps: Bps,
    unit_cost: Money,
    restock: bool,
}

impl PricedLine {
    fn from_refund(refund: LineRefund, restock: bool) -> PricedLine {
        PricedLine {
            sale_line_id: Some(refund.sale_line_id),
            product_id: refund.product_id,
            name: refund.name,
            qty: refund.qty,
            amounts: refund.amounts,
            tax_rate_id: refund.tax_rate_id,
            tax_bps: refund.tax_bps,
            unit_cost: refund.unit_cost,
            restock,
        }
    }
}

fn priced_lines(tx: &Transaction, new_return: &NewReturn) -> Result<Vec<PricedLine>, SalesError> {
    let mut wanted = Vec::new();
    let mut restock_flags = Vec::new();
    let mut without_receipt = Vec::new();
    for line in &new_return.lines {
        match line {
            NewReturnLine::FromSale(line) => {
                wanted.push(ReturnQty {
                    sale_line_id: line.sale_line_id.clone(),
                    qty: line.qty,
                });
                restock_flags.push(line.restock);
            }
            NewReturnLine::WithoutReceipt(product) => without_receipt.push(product),
        }
    }
    let mixed = match new_return.sale_id {
        Some(_) => !without_receipt.is_empty(),
        None => !wanted.is_empty(),
    };
    if mixed {
        return Err(SalesError::invalid("return_lines_mixed"));
    }
    let mut priced = Vec::new();
    if let Some(sale_id) = &new_return.sale_id {
        let refunds = refund_amounts(tx, sale_id, &wanted)?;
        for (refund, restock) in refunds.into_iter().zip(restock_flags) {
            priced.push(PricedLine::from_refund(refund, restock));
        }
    }
    for product in without_receipt {
        priced.push(unreceipted_line(product)?);
    }
    Ok(priced)
}

/// Checks a line returned without a receipt and works out its cost.
fn unreceipted_line(product: &ReturnedProduct) -> Result<PricedLine, SalesError> {
    if product.qty.0 <= 0 {
        return Err(SalesError::invalid("qty_not_positive"));
    }
    let amounts = [product.net, product.tax, product.gross, product.unit_cost];
    if amounts.iter().any(|amount| amount.0 < 0)
        || product.net.checked_add(product.tax)? != product.gross
    {
        return Err(SalesError::TotalsDontAddUp(format!(
            "return line {}: net + tax is not gross",
            product.name
        )));
    }
    Ok(PricedLine {
        sale_line_id: None,
        product_id: product.product_id.clone(),
        name: product.name.clone(),
        qty: product.qty,
        amounts: LineAmounts {
            net: product.net,
            tax: product.tax,
            gross: product.gross,
            cost: mul_qty(product.unit_cost, product.qty)?,
        },
        tax_rate_id: product.tax_rate_id.clone(),
        tax_bps: product.tax_bps,
        unit_cost: product.unit_cost,
        restock: product.restock,
    })
}

/// How much of a sale line was returned already, and what those returns refunded.
fn returned_so_far(conn: &Connection, sale_line_id: &Id) -> Result<(Qty, LineAmounts), SalesError> {
    let found = conn.query_row(
        "SELECT coalesce(sum(qty_milli), 0), coalesce(sum(net_minor), 0), coalesce(sum(tax_minor), 0),
                coalesce(sum(gross_minor), 0), coalesce(sum(cost_minor), 0)
         FROM sales_return_lines WHERE sale_line_id = :line",
        named_params! { ":line": sale_line_id.as_str() },
        |row| {
            Ok((
                Qty(row.get(0)?),
                LineAmounts { net: Money(row.get(1)?), tax: Money(row.get(2)?), gross: Money(row.get(3)?), cost: Money(row.get(4)?) },
            ))
        },
    )?;
    Ok(found)
}

fn add_amounts(left: LineAmounts, right: LineAmounts) -> Result<LineAmounts, AppError> {
    Ok(LineAmounts {
        net: left.net.checked_add(right.net)?,
        tax: left.tax.checked_add(right.tax)?,
        gross: left.gross.checked_add(right.gross)?,
        cost: left.cost.checked_add(right.cost)?,
    })
}

fn insert_header(
    tx: &Transaction,
    return_ref: &ReturnRef,
    new_return: &NewReturn,
    sum: LineAmounts,
    total: Money,
) -> Result<(), SalesError> {
    let reason = new_return
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty());
    tx.execute(
        "INSERT INTO sales_returns (id, return_no, sale_id, till_id, shift_id, user_id, approved_by,
            business_date, at, customer_id, reason, net_minor, tax_minor, gross_minor, rounding_minor,
            total_minor, cost_minor)
         VALUES (:id, :return_no, :sale, :till, :shift, :user, :approved_by,
            :date, :at, :customer, :reason, :net, :tax, :gross, :rounding, :total, :cost)",
        named_params! {
            ":id": return_ref.id.as_str(), ":return_no": return_ref.return_no,
            ":sale": new_return.sale_id.as_ref().map(Id::as_str), ":till": new_return.till_id.as_str(),
            ":shift": new_return.shift_id.as_str(), ":user": new_return.user_id.as_str(),
            ":approved_by": new_return.approved_by.as_ref().map(Id::as_str),
            ":date": new_return.business_date.to_string(), ":at": new_return.at.0,
            ":customer": new_return.customer_id.as_ref().map(Id::as_str), ":reason": reason,
            ":net": sum.net.0, ":tax": sum.tax.0, ":gross": sum.gross.0,
            ":rounding": new_return.rounding.0, ":total": total.0, ":cost": sum.cost.0,
        },
    )?;
    Ok(())
}

fn insert_line(tx: &Transaction, return_id: &Id, line: &PricedLine) -> Result<(), SalesError> {
    tx.execute(
        "INSERT INTO sales_return_lines (id, return_id, sale_line_id, product_id, name, qty_milli,
            net_minor, tax_minor, gross_minor, tax_rate_id, tax_bps, unit_cost_minor, cost_minor, restock)
         VALUES (:id, :return, :sale_line, :product, :name, :qty,
            :net, :tax, :gross, :tax_rate, :tax_bps, :unit_cost, :cost, :restock)",
        named_params! {
            ":id": Id::new().as_str(), ":return": return_id.as_str(), ":sale_line": line.sale_line_id.as_ref().map(Id::as_str),
            ":product": line.product_id.as_str(), ":name": line.name, ":qty": line.qty.0,
            ":net": line.amounts.net.0, ":tax": line.amounts.tax.0, ":gross": line.amounts.gross.0,
            ":tax_rate": line.tax_rate_id.as_str(), ":tax_bps": line.tax_bps.0,
            ":unit_cost": line.unit_cost.0, ":cost": line.amounts.cost.0, ":restock": line.restock,
        },
    )?;
    Ok(())
}
