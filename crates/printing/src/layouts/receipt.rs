//! Sale, void and return receipts (rules HW-02, HW-03, TAX-08, RET-04, RET-06, SET-14).

use pos_core::{CurrencyFormat, Money, Qty};

use super::receipt_data::{
    CustomerOnReceipt, PaymentLine, ReceiptData, ReceiptLayout, ReceiptLine, ReturnData,
    TaxSummaryLine,
};
use super::{BOTTOM_FEED_DOTS, barcode_with_digits, feed, row, rule, small_centered, text};
use crate::doc::{Align, Block, Doc, RuleStyle, TextSize};
use crate::labels::{ReceiptLabels, fill};
use crate::numbers::{money, money_with_symbol, quantity};

/// Size of the QR code on a receipt, in dots (about 20 mm).
const QR_DOTS: u32 = 160;

/// Lays out a sale receipt, or a void receipt when `void_reason` is set.
pub fn receipt(d: &ReceiptData, l: &ReceiptLayout, labels: &ReceiptLabels) -> Doc {
    let mut blocks = shop_header(l);
    if d.copy {
        blocks.push(feed(8));
        blocks.push(text(
            &labels.copy_heading,
            TextSize::Huge,
            true,
            Align::Center,
        ));
    }
    if let Some(reason) = &d.void_reason {
        blocks.push(feed(8));
        blocks.push(text(
            &labels.void_heading,
            TextSize::Huge,
            true,
            Align::Center,
        ));
        blocks.push(reason_line(labels, reason));
    }
    blocks.push(rule(RuleStyle::Single));
    blocks.push(detail(&labels.invoice_no, &d.invoice_no));
    blocks.push(detail(&labels.date, &d.printed_at));
    blocks.push(detail(&labels.till, &d.till));
    blocks.push(detail(&labels.cashier, &d.cashier));
    if let Some(customer) = &d.customer {
        blocks.push(detail(&labels.customer, &customer.name));
    }
    blocks.push(rule(RuleStyle::Single));
    push_item_lines(&mut blocks, &d.lines, &l.currency);

    blocks.push(rule(RuleStyle::Single));
    let currency = &l.currency;
    blocks.push(row(
        &labels.subtotal,
        money(d.subtotal, currency),
        TextSize::Normal,
        false,
    ));
    if d.discount_total != Money::zero() {
        blocks.push(row(
            &labels.discount,
            money(d.discount_total, currency),
            TextSize::Normal,
            false,
        ));
    }
    if d.rounding != Money::zero() {
        blocks.push(row(
            &labels.rounding,
            money(d.rounding, currency),
            TextSize::Normal,
            false,
        ));
    }
    push_big_total(&mut blocks, &labels.total, d.total, currency);
    push_payments(&mut blocks, &d.payments, labels, currency);
    if d.change != Money::zero() {
        blocks.push(row(
            &labels.change,
            money(d.change, currency),
            TextSize::Normal,
            false,
        ));
    }

    push_tax_part(
        &mut blocks,
        &d.tax_summary,
        &d.tax_label,
        d.prices_include_tax,
        l,
        labels,
    );
    if l.show_customer_points
        && let Some(customer) = &d.customer
    {
        push_customer_balances(&mut blocks, customer, labels, currency);
    }
    push_footer(&mut blocks, l);
    if l.show_invoice_barcode
        && let Some(barcode) = &d.invoice_barcode
    {
        blocks.push(feed(8));
        barcode_with_digits(&mut blocks, barcode, &d.invoice_no);
    }
    if let Some(qr_data) = &d.qr_data {
        blocks.push(feed(8));
        blocks.push(Block::Qr {
            data: qr_data.clone(),
            size_dots: QR_DOTS,
        });
    }
    blocks.push(feed(BOTTOM_FEED_DOTS));
    Doc {
        width_dots: l.paper.dots(),
        blocks,
    }
}

/// Lays out a return receipt: the return heading, both numbers, the returned
/// lines, and how much was refunded by which method (rule RET-06).
pub fn return_receipt(d: &ReturnData, l: &ReceiptLayout, labels: &ReceiptLabels) -> Doc {
    let mut blocks = shop_header(l);
    blocks.push(feed(8));
    blocks.push(text(
        &labels.return_heading,
        TextSize::Huge,
        true,
        Align::Center,
    ));
    blocks.push(rule(RuleStyle::Single));
    blocks.push(detail(&labels.return_no, &d.return_no));
    if let Some(invoice_no) = &d.original_invoice_no {
        blocks.push(detail(&labels.original_invoice, invoice_no));
    }
    blocks.push(detail(&labels.date, &d.printed_at));
    blocks.push(detail(&labels.till, &d.till));
    blocks.push(detail(&labels.cashier, &d.cashier));
    if let Some(customer) = &d.customer {
        blocks.push(detail(&labels.customer, &customer.name));
    }
    blocks.push(rule(RuleStyle::Single));
    push_item_lines(&mut blocks, &d.lines, &l.currency);

    push_big_total(
        &mut blocks,
        &labels.refund_total,
        d.refund_total,
        &l.currency,
    );
    push_payments(&mut blocks, &d.refunds, labels, &l.currency);
    if let Some(reason) = &d.reason {
        blocks.push(reason_line(labels, reason));
    }
    push_tax_part(
        &mut blocks,
        &d.tax_summary,
        &d.tax_label,
        d.prices_include_tax,
        l,
        labels,
    );
    if l.show_customer_points
        && let Some(customer) = &d.customer
    {
        push_customer_balances(&mut blocks, customer, labels, &l.currency);
    }
    push_footer(&mut blocks, l);
    blocks.push(feed(BOTTOM_FEED_DOTS));
    Doc {
        width_dots: l.paper.dots(),
        blocks,
    }
}

/// Logo, shop name, address, phone, tax number and header lines, centred.
pub(crate) fn shop_header(l: &ReceiptLayout) -> Vec<Block> {
    let mut blocks = Vec::new();
    if l.show_logo
        && let Some(logo) = &l.logo
    {
        blocks.push(Block::Image {
            bitmap: logo.clone(),
            align: Align::Center,
        });
        blocks.push(feed(8));
    }
    blocks.push(text(&l.shop.name, TextSize::Large, true, Align::Center));
    for line in &l.shop.address_lines {
        blocks.push(small_centered(line));
    }
    if !l.shop.phone.is_empty() {
        blocks.push(small_centered(&l.shop.phone));
    }
    if l.show_tax_reg_no && !l.shop.tax_reg_no.is_empty() {
        blocks.push(small_centered(format!(
            "{}: {}",
            l.shop.tax_reg_label, l.shop.tax_reg_no
        )));
    }
    for line in &l.header_lines {
        blocks.push(small_centered(line));
    }
    blocks
}

/// A small "Label      value" line under the header.
fn detail(label: &str, value: &str) -> Block {
    row(label, value, TextSize::Small, false)
}

/// "Reason: Wrong customer"
fn reason_line(labels: &ReceiptLabels, reason: &str) -> Block {
    text(
        format!("{}: {reason}", labels.reason),
        TextSize::Normal,
        false,
        Align::Center,
    )
}

/// Each item: name and amount; then "2 × 90.00" when it isn't one piece; then its discount.
fn push_item_lines(blocks: &mut Vec<Block>, lines: &[ReceiptLine], currency: &CurrencyFormat) {
    for line in lines {
        blocks.push(row(
            &line.name,
            money(line.amount, currency),
            TextSize::Normal,
            false,
        ));
        let one_piece = line.qty == Qty(1000) && line.unit.is_empty();
        if !one_piece {
            let qty = quantity(line.qty, &line.unit, &currency.decimal_sep);
            let each = money(line.unit_price, currency);
            blocks.push(text(
                format!("  {qty} × {each}"),
                TextSize::Small,
                false,
                Align::Left,
            ));
        }
        if let Some(discount) = &line.discount {
            blocks.push(row(
                format!("  {}", discount.label),
                money(discount.amount, currency),
                TextSize::Small,
                false,
            ));
        }
    }
}

/// The total (or refund) in large bold text with the currency symbol, between double lines.
fn push_big_total(blocks: &mut Vec<Block>, label: &str, amount: Money, currency: &CurrencyFormat) {
    blocks.push(rule(RuleStyle::Double));
    blocks.push(row(
        label,
        money_with_symbol(amount, currency),
        TextSize::Large,
        true,
    ));
    blocks.push(rule(RuleStyle::Double));
}

/// One line per payment or refund: "Card Ref 1234      500.00".
fn push_payments(
    blocks: &mut Vec<Block>,
    payments: &[PaymentLine],
    labels: &ReceiptLabels,
    currency: &CurrencyFormat,
) {
    for payment in payments {
        let method = match &payment.reference {
            Some(reference) => format!("{} {} {reference}", payment.method, labels.reference),
            None => payment.method.clone(),
        };
        blocks.push(row(
            method,
            money(payment.amount, currency),
            TextSize::Normal,
            false,
        ));
    }
}

/// The tax summary table (if the layout shows it) and "Prices include VAT" (rule TAX-08).
fn push_tax_part(
    blocks: &mut Vec<Block>,
    summary: &[TaxSummaryLine],
    tax_label: &str,
    prices_include_tax: bool,
    l: &ReceiptLayout,
    labels: &ReceiptLabels,
) {
    if l.show_tax_breakdown && !summary.is_empty() {
        blocks.push(rule(RuleStyle::Single));
        blocks.push(text(
            &labels.tax_summary,
            TextSize::Small,
            true,
            Align::Left,
        ));
        blocks.push(tax_row(["", &labels.tax_net, &labels.tax_amount], true));
        for line in summary {
            let net = money(line.net, &l.currency);
            let tax = money(line.tax, &l.currency);
            blocks.push(tax_row([&line.name, &net, &tax], false));
        }
    }
    if prices_include_tax {
        blocks.push(feed(4));
        blocks.push(small_centered(fill(
            &labels.prices_include_tax,
            &[("tax", tax_label)],
        )));
    }
}

/// A tax summary row: name, net, tax.
fn tax_row([name, net, tax]: [&str; 3], bold: bool) -> Block {
    Block::Columns {
        cells: vec![
            (name.to_string(), 2, Align::Left),
            (net.to_string(), 1, Align::Right),
            (tax.to_string(), 1, Align::Right),
        ],
        size: TextSize::Small,
        bold,
    }
}

/// The customer's points and credit balance after this sale.
fn push_customer_balances(
    blocks: &mut Vec<Block>,
    customer: &CustomerOnReceipt,
    labels: &ReceiptLabels,
    currency: &CurrencyFormat,
) {
    if let Some(points) = customer.points_balance {
        blocks.push(row(
            &labels.points,
            points.to_string(),
            TextSize::Small,
            false,
        ));
    }
    if let Some(balance) = customer.credit_balance {
        blocks.push(row(
            &labels.credit_balance,
            money(balance, currency),
            TextSize::Small,
            false,
        ));
    }
}

/// The shop's footer lines, centred.
fn push_footer(blocks: &mut Vec<Block>, l: &ReceiptLayout) {
    if l.footer_lines.is_empty() {
        return;
    }
    blocks.push(feed(8));
    for line in &l.footer_lines {
        blocks.push(small_centered(line));
    }
}
