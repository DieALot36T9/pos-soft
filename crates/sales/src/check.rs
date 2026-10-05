//! Checks that a finished sale adds up before it is saved. Pure functions: no database.
//! The workflow does the pricing; these checks catch a workflow bug before it
//! becomes a wrong receipt. Never loosen them.

use pos_core::{Money, TaxMode, mul_qty};

use crate::{CompletedSale, LineSnapshot, NewPayment, PaymentKind, PaymentMethod, SalesError};

/// Checks the whole sale and returns each line's cost, in line order.
pub(crate) fn check_sale(
    sale: &CompletedSale,
    methods: &[PaymentMethod],
) -> Result<Vec<Money>, SalesError> {
    if sale.lines.is_empty() {
        return Err(SalesError::invalid("cart_empty"));
    }
    if sale.till_code.trim().is_empty() {
        return Err(SalesError::invalid("till_code_missing"));
    }
    let mut costs = Vec::new();
    for (index, line) in sale.lines.iter().enumerate() {
        costs.push(check_line(index + 1, line, sale.tax_mode)?);
    }
    check_totals(sale, &costs)?;
    check_sale_payments(sale, methods)?;
    Ok(costs)
}

/// Checks one line's amounts and returns its cost: round(unit cost × quantity).
pub(crate) fn check_line(
    number: usize,
    line: &LineSnapshot,
    tax_mode: TaxMode,
) -> Result<Money, SalesError> {
    let wrong = |what: &str| SalesError::TotalsDontAddUp(format!("line {number}: {what}"));
    if line.qty.0 <= 0 {
        return Err(wrong("quantity must be more than zero"));
    }
    let amounts = [
        line.unit_price,
        line.retail_price,
        line.base,
        line.promo_discount,
        line.tier_discount,
        line.manual_discount,
        line.invoice_discount,
        line.net,
        line.tax,
        line.gross,
        line.unit_cost,
    ];
    if amounts.iter().any(|amount| amount.0 < 0) {
        return Err(wrong("an amount is negative"));
    }
    if line.net.checked_add(line.tax)? != line.gross {
        return Err(wrong("net + tax is not gross"));
    }
    // The line amount after discounts (Money Tax and Rounding, step 6).
    let discounts = Money::sum(&[
        line.promo_discount,
        line.tier_discount,
        line.manual_discount,
        line.invoice_discount,
    ])?;
    let amount = line.base.checked_sub(discounts)?;
    let expected = match tax_mode {
        TaxMode::Inclusive => line.gross,
        TaxMode::Exclusive => line.net,
    };
    if amount != expected {
        return Err(wrong("base − discounts doesn't match the taxed amount"));
    }
    if !line.tax_components.is_empty() {
        let parts: Vec<Money> = line.tax_components.iter().map(|part| part.tax).collect();
        if Money::sum(&parts)? != line.tax {
            return Err(wrong("tax parts don't add up to the tax"));
        }
    }
    Ok(mul_qty(line.unit_cost, line.qty)?)
}

/// Each header total must be the sum of the lines; total = gross + rounding.
fn check_totals(sale: &CompletedSale, costs: &[Money]) -> Result<(), SalesError> {
    let totals = &sale.totals;
    let sum_of = |pick: fn(&LineSnapshot) -> Money| -> Result<Money, SalesError> {
        let amounts: Vec<Money> = sale.lines.iter().map(pick).collect();
        Ok(Money::sum(&amounts)?)
    };
    let checks: [(&str, Money, Money); 9] = [
        ("base", totals.base, sum_of(|line| line.base)?),
        (
            "promo_discount",
            totals.promo_discount,
            sum_of(|line| line.promo_discount)?,
        ),
        (
            "tier_discount",
            totals.tier_discount,
            sum_of(|line| line.tier_discount)?,
        ),
        (
            "manual_discount",
            totals.manual_discount,
            sum_of(|line| line.manual_discount)?,
        ),
        (
            "invoice_discount",
            totals.invoice_discount,
            sum_of(|line| line.invoice_discount)?,
        ),
        ("net", totals.net, sum_of(|line| line.net)?),
        ("tax", totals.tax, sum_of(|line| line.tax)?),
        ("gross", totals.gross, sum_of(|line| line.gross)?),
        ("cost", totals.cost, Money::sum(costs)?),
    ];
    for (name, header, lines) in checks {
        if header != lines {
            return Err(SalesError::TotalsDontAddUp(format!(
                "{name}: header {} but lines add up to {}",
                header.0, lines.0
            )));
        }
    }
    if totals.gross.checked_add(totals.rounding)? != totals.total {
        return Err(SalesError::TotalsDontAddUp(
            "total is not gross + rounding".into(),
        ));
    }
    if totals.total.0 < 0 || totals.change.0 < 0 {
        return Err(SalesError::TotalsDontAddUp(
            "total or change is negative".into(),
        ));
    }
    Ok(())
}

/// Payments must cover the total exactly; only cash may hand over more (the change).
fn check_sale_payments(sale: &CompletedSale, methods: &[PaymentMethod]) -> Result<(), SalesError> {
    let mut paid = Money::zero();
    let mut change = Money::zero();
    for payment in &sale.payments {
        let method = check_method(payment, methods)?;
        paid = paid.checked_add(payment.amount)?;
        match (method.kind, payment.tendered) {
            (PaymentKind::Cash, Some(tendered)) if tendered >= payment.amount => {
                change = change.checked_add(tendered.checked_sub(payment.amount)?)?;
            }
            (PaymentKind::Cash, None) => {}
            (_, None) => {}
            _ => {
                return Err(SalesError::TotalsDontAddUp(
                    "only cash may be tendered, and never less than its amount".into(),
                ));
            }
        }
    }
    if paid < sale.totals.total {
        let remaining = sale.totals.total.checked_sub(paid)?;
        return Err(SalesError::invalid_with(
            "payment_not_enough",
            "remaining_minor",
            remaining.0.to_string(),
        ));
    }
    if paid > sale.totals.total {
        return Err(SalesError::TotalsDontAddUp(
            "payments are more than the total".into(),
        ));
    }
    if change != sale.totals.change {
        return Err(SalesError::TotalsDontAddUp(
            "change doesn't match the cash tendered".into(),
        ));
    }
    Ok(())
}

/// Refund payments on a return: they must add up to the total exactly, with no change.
pub(crate) fn check_refund_payments(
    total: Money,
    payments: &[NewPayment],
    methods: &[PaymentMethod],
) -> Result<(), SalesError> {
    let mut paid = Money::zero();
    for payment in payments {
        check_method(payment, methods)?;
        if payment.tendered.is_some() {
            return Err(SalesError::TotalsDontAddUp(
                "a refund has no tendered amount".into(),
            ));
        }
        paid = paid.checked_add(payment.amount)?;
    }
    if paid != total {
        return Err(SalesError::TotalsDontAddUp(format!(
            "refunds add up to {} but the return total is {}",
            paid.0, total.0
        )));
    }
    Ok(())
}

/// Checks one payment against its method: positive amount, active method, reference if needed.
fn check_method<'a>(
    payment: &NewPayment,
    methods: &'a [PaymentMethod],
) -> Result<&'a PaymentMethod, SalesError> {
    let Some(method) = methods.iter().find(|method| method.id == payment.method_id) else {
        return Err(SalesError::not_found(
            "payment_method",
            payment.method_id.as_str(),
        ));
    };
    if payment.amount.0 <= 0 {
        return Err(SalesError::invalid("payment_amount_invalid"));
    }
    if !method.active {
        return Err(SalesError::invalid("payment_method_inactive"));
    }
    let has_reference = payment
        .reference
        .as_deref()
        .is_some_and(|text| !text.trim().is_empty());
    if method.requires_reference && !has_reference {
        return Err(SalesError::invalid("payment_reference_missing"));
    }
    Ok(method)
}
