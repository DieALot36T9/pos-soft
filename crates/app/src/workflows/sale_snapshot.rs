//! The completed sale exactly as `sales::complete` wants it (ADR-011): the header, one
//! snapshot per cart line with every name, price, tax and cost it used, and the payments.

use pos_core::{AppError, Clock, Id, Money, business_date, mul_qty};
use rusqlite::{Connection, Transaction};
use sales::{
    CompletedSale, CustomerSnapshot, LineSnapshot, PromotionSnapshot, SaleTotals, Settlement,
    TaxPart,
};

use crate::workflows::cart_view::{PricedCart, PricedLine};
use crate::workflows::checkout::Selling;

/// The sale exactly as `sales::complete` wants it: header, snapshot lines and payments.
pub(crate) fn completed_sale(
    tx: &Transaction,
    clock: &dyn Clock,
    selling: &Selling,
    user: &Id,
    priced: &PricedCart,
    settled: &Settlement,
    due: Money,
) -> Result<CompletedSale, AppError> {
    let mut lines = Vec::with_capacity(priced.lines.len());
    let mut totals = SaleTotals::default();
    for line in &priced.lines {
        let snapshot = snapshot(tx, line)?;
        add_to_totals(&mut totals, &snapshot)?;
        lines.push(snapshot);
    }
    totals.total = due;
    totals.rounding = due.checked_sub(totals.gross)?;
    totals.change = settled.change;
    let now = clock.now();
    Ok(CompletedSale {
        cart_id: priced.cart.id.clone(),
        till_id: selling.till_id.clone(),
        till_code: selling.till_code.clone(),
        shift_id: selling.shift.id.clone(),
        user_id: user.clone(),
        business_date: business_date(now, clock.local_offset_minutes(), selling.cutoff_hour)?,
        completed_at: now,
        customer: priced.customer.as_ref().map(|customer| CustomerSnapshot {
            id: customer.customer.id.clone(),
            name: customer.customer.name.clone(),
            phone: customer.customer.phone.clone(),
        }),
        price_list_id: priced.cart.price_list_id.clone(),
        tax_mode: priced.tax.mode,
        exchange_return_id: priced.cart.exchange_return_id.clone(),
        totals,
        lines,
        payments: settled.payments.clone(),
    })
}

/// One priced line copied with every name, price, tax and cost it used (ADR-011).
fn snapshot(conn: &Connection, line: &PricedLine) -> Result<LineSnapshot, AppError> {
    let price = &line.price;
    let tax_components = if line.rate.components.is_empty() {
        Vec::new()
    } else {
        tax::split_components(line.tax.tax, &line.rate)?
            .into_iter()
            .zip(&line.rate.components)
            .map(|((name, tax), component)| TaxPart {
                name,
                bps: component.rate_bps,
                tax,
            })
            .collect()
    };
    Ok(LineSnapshot {
        product_id: line.product.id.clone(),
        name: line.product.name.clone(),
        sku: line.product.sku.clone(),
        unit_symbol: line.unit.symbol.clone(),
        category_id: line.product.category_id.clone(),
        brand_id: line.product.brand_id.clone(),
        qty: line.line.qty,
        unit_price: price.unit_price,
        retail_price: line.retail_price,
        base: price.base,
        promotion: price.promo.as_ref().map(|promo| PromotionSnapshot {
            id: promo.promotion_id.clone(),
            name: promo.name.clone(),
        }),
        promo_discount: price.promo_discount(),
        tier_discount: price.tier_discount,
        manual_discount: price.manual_discount,
        invoice_discount: price.invoice_share,
        net: line.tax.net,
        tax: line.tax.tax,
        gross: line.tax.gross,
        tax_rate_id: line.rate.id.clone(),
        tax_name: line.rate.name.clone(),
        tax_bps: line.rate.rate_bps,
        tax_components,
        unit_cost: unit_cost(conn, &line.product)?,
        price_overridden: line.line.price_override.is_some(),
    })
}

/// The cost of one unit: the product's cost, or for a kit the cost of its parts.
fn unit_cost(conn: &Connection, product: &catalog::Product) -> Result<Money, AppError> {
    if product.kind != catalog::ProductKind::Kit {
        return Ok(product.cost_price);
    }
    let mut cost = Money::zero();
    for (part_id, part_qty) in catalog::kit_components(conn, &product.id)? {
        let part = catalog::get(conn, &part_id)?;
        cost = cost.checked_add(mul_qty(part.cost_price, part_qty)?)?;
    }
    Ok(cost)
}

fn add_to_totals(totals: &mut SaleTotals, line: &LineSnapshot) -> Result<(), AppError> {
    totals.base = totals.base.checked_add(line.base)?;
    totals.promo_discount = totals.promo_discount.checked_add(line.promo_discount)?;
    totals.tier_discount = totals.tier_discount.checked_add(line.tier_discount)?;
    totals.manual_discount = totals.manual_discount.checked_add(line.manual_discount)?;
    totals.invoice_discount = totals.invoice_discount.checked_add(line.invoice_discount)?;
    totals.net = totals.net.checked_add(line.net)?;
    totals.tax = totals.tax.checked_add(line.tax)?;
    totals.gross = totals.gross.checked_add(line.gross)?;
    totals.cost = totals
        .cost
        .checked_add(mul_qty(line.unit_cost, line.qty)?)?;
    Ok(())
}
