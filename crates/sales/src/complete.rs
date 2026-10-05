//! Completing a sale: check it, give it the next invoice number, save it, delete the cart.
//! All in the caller's transaction (PAY-07), so a failure anywhere saves nothing.

use pos_core::{AppError, Id, Money, Timestamp};
use rusqlite::{Transaction, named_params};

use crate::cart::{CartStatus, get_cart};
use crate::cart_hold::delete_cart;
use crate::check::check_sale;
use crate::numbers::{DocKind, format_number, take_next_number};
use crate::{CompletedSale, LineSnapshot, NewPayment, SaleRef, SalesError, payment_methods};

/// Saves a finished sale and returns its id and invoice number (PAY-07, PAY-08).
/// The lines must be fully priced by the workflow and in the cart's order; sales only
/// checks that everything adds up and refuses the sale if it doesn't.
pub fn complete(tx: &Transaction, sale: CompletedSale) -> Result<SaleRef, SalesError> {
    check_matches_cart(tx, &sale)?;
    let methods = payment_methods(tx)?;
    let line_costs = check_sale(&sale, &methods)?;

    let number = take_next_number(tx, &sale.till_id, DocKind::Sale)?;
    let sale_ref = SaleRef {
        id: Id::new(),
        invoice_no: format_number(sale.till_code.trim(), DocKind::Sale, number),
    };
    insert_header(tx, &sale_ref, &sale)?;
    for (index, line) in sale.lines.iter().enumerate() {
        let seq =
            u32::try_from(index + 1).map_err(|_| AppError::internal("too many sale lines"))?;
        insert_line(tx, &sale_ref.id, seq, line, line_costs[index])?;
    }
    for payment in &sale.payments {
        insert_payment(tx, "sale", &sale_ref.id, payment, sale.completed_at)?;
    }
    delete_cart(tx, &sale.cart_id)?;
    Ok(sale_ref)
}

/// The sale must come from an open cart on the same till, with the same products
/// and quantities in the same order. This catches a workflow completing the wrong cart.
fn check_matches_cart(tx: &Transaction, sale: &CompletedSale) -> Result<(), SalesError> {
    let cart = get_cart(tx, &sale.cart_id)?;
    if cart.status != CartStatus::Open {
        return Err(SalesError::conflict("cart_not_open"));
    }
    if cart.till_id != sale.till_id {
        return Err(SalesError::TotalsDontAddUp(
            "the cart belongs to another till".into(),
        ));
    }
    let same_lines = cart.lines.len() == sale.lines.len()
        && cart
            .lines
            .iter()
            .zip(&sale.lines)
            .all(|(cart_line, sale_line)| {
                cart_line.product_id == sale_line.product_id && cart_line.qty == sale_line.qty
            });
    if !same_lines {
        return Err(SalesError::TotalsDontAddUp(
            "the lines don't match the cart".into(),
        ));
    }
    Ok(())
}

fn insert_header(
    tx: &Transaction,
    sale_ref: &SaleRef,
    sale: &CompletedSale,
) -> Result<(), SalesError> {
    let totals = &sale.totals;
    let customer = sale.customer.as_ref();
    tx.execute(
        "INSERT INTO sales_sales (id, invoice_no, till_id, shift_id, user_id, business_date, completed_at,
            customer_id, customer_name, customer_phone, price_list_id, status,
            base_minor, promo_discount_minor, tier_discount_minor, manual_discount_minor, invoice_discount_minor,
            net_minor, tax_minor, gross_minor, rounding_minor, total_minor, change_minor, cost_minor,
            tax_mode, exchange_return_id)
         VALUES (:id, :invoice_no, :till, :shift, :user, :date, :at,
            :customer_id, :customer_name, :customer_phone, :price_list, 'completed',
            :base, :promo, :tier, :manual, :invoice,
            :net, :tax, :gross, :rounding, :total, :change, :cost,
            :tax_mode, :exchange_return)",
        named_params! {
            ":id": sale_ref.id.as_str(), ":invoice_no": sale_ref.invoice_no,
            ":till": sale.till_id.as_str(), ":shift": sale.shift_id.as_str(), ":user": sale.user_id.as_str(),
            ":date": sale.business_date.to_string(), ":at": sale.completed_at.0,
            ":customer_id": customer.map(|value| value.id.as_str()),
            ":customer_name": customer.map(|value| value.name.as_str()),
            ":customer_phone": customer.map(|value| value.phone.as_str()),
            ":price_list": sale.price_list_id.as_ref().map(Id::as_str),
            ":base": totals.base.0, ":promo": totals.promo_discount.0, ":tier": totals.tier_discount.0,
            ":manual": totals.manual_discount.0, ":invoice": totals.invoice_discount.0,
            ":net": totals.net.0, ":tax": totals.tax.0, ":gross": totals.gross.0,
            ":rounding": totals.rounding.0, ":total": totals.total.0, ":change": totals.change.0,
            ":cost": totals.cost.0, ":tax_mode": sale.tax_mode.as_str(),
            ":exchange_return": sale.exchange_return_id.as_ref().map(Id::as_str),
        },
    )?;
    Ok(())
}

fn insert_line(
    tx: &Transaction,
    sale_id: &Id,
    seq: u32,
    line: &LineSnapshot,
    cost: Money,
) -> Result<(), SalesError> {
    let components = serde_json::to_string(&line.tax_components)
        .map_err(|error| AppError::internal(format!("tax parts to JSON: {error}")))?;
    let promotion = line.promotion.as_ref();
    tx.execute(
        "INSERT INTO sales_lines (id, sale_id, seq, product_id, name, sku, unit_symbol, category_id, brand_id,
            qty_milli, unit_price_minor, retail_price_minor, base_minor, promotion_id, promotion_name,
            promo_discount_minor, tier_discount_minor, manual_discount_minor, invoice_discount_minor,
            net_minor, tax_minor, gross_minor, tax_rate_id, tax_name, tax_bps, tax_components_json,
            unit_cost_minor, cost_minor, price_overridden)
         VALUES (:id, :sale, :seq, :product, :name, :sku, :unit, :category, :brand,
            :qty, :unit_price, :retail_price, :base, :promotion_id, :promotion_name,
            :promo, :tier, :manual, :invoice,
            :net, :tax, :gross, :tax_rate, :tax_name, :tax_bps, :components,
            :unit_cost, :cost, :overridden)",
        named_params! {
            ":id": Id::new().as_str(), ":sale": sale_id.as_str(), ":seq": seq,
            ":product": line.product_id.as_str(), ":name": line.name, ":sku": line.sku,
            ":unit": line.unit_symbol, ":category": line.category_id.as_str(),
            ":brand": line.brand_id.as_ref().map(Id::as_str),
            ":qty": line.qty.0, ":unit_price": line.unit_price.0, ":retail_price": line.retail_price.0,
            ":base": line.base.0, ":promotion_id": promotion.map(|value| value.id.as_str()),
            ":promotion_name": promotion.map(|value| value.name.as_str()),
            ":promo": line.promo_discount.0, ":tier": line.tier_discount.0,
            ":manual": line.manual_discount.0, ":invoice": line.invoice_discount.0,
            ":net": line.net.0, ":tax": line.tax.0, ":gross": line.gross.0,
            ":tax_rate": line.tax_rate_id.as_str(), ":tax_name": line.tax_name, ":tax_bps": line.tax_bps.0,
            ":components": components, ":unit_cost": line.unit_cost.0, ":cost": cost.0,
            ":overridden": line.price_overridden,
        },
    )?;
    Ok(())
}

/// Saves one payment (on a sale) or refund (on a return). The kind is copied from its method.
pub(crate) fn insert_payment(
    tx: &Transaction,
    doc_type: &str,
    doc_id: &Id,
    payment: &NewPayment,
    at: Timestamp,
) -> Result<(), SalesError> {
    let reference = payment
        .reference
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty());
    tx.execute(
        "INSERT INTO sales_payments (id, doc_type, doc_id, method_id, kind, amount_minor, tendered_minor, reference, at)
         VALUES (:id, :doc_type, :doc_id, :method,
                 (SELECT kind FROM sales_payment_methods WHERE id = :method),
                 :amount, :tendered, :reference, :at)",
        named_params! {
            ":id": Id::new().as_str(), ":doc_type": doc_type, ":doc_id": doc_id.as_str(),
            ":method": payment.method_id.as_str(), ":amount": payment.amount.0,
            ":tendered": payment.tendered.map(|value| value.0), ":reference": reference, ":at": at.0,
        },
    )?;
    Ok(())
}
