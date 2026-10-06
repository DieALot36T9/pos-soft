//! Every word printed on receipts, reports, labels and shelf tags (ADR-022).
//! `app` fills this; layouts never type words of their own.

/// The printed words. A future language is a second set of these words.
///
/// Some are small templates with names in braces, filled in by the layout.
/// Example: `prices_include_tax` is "Prices include {tax}" → "Prices include VAT".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptLabels {
    /// "Invoice"
    pub invoice_no: String,
    /// "Date"
    pub date: String,
    /// "Till"
    pub till: String,
    /// "Cashier"
    pub cashier: String,
    /// "Customer"
    pub customer: String,
    /// "Points": the customer's loyalty points balance.
    pub points: String,
    /// "Balance due": what the customer owes on credit.
    pub credit_balance: String,
    /// "Subtotal"
    pub subtotal: String,
    /// "Discount"
    pub discount: String,
    /// "Rounding": the cash rounding amount.
    pub rounding: String,
    /// "Total"
    pub total: String,
    /// "Change": money given back to the customer.
    pub change: String,
    /// "Ref": the card or wallet payment reference.
    pub reference: String,
    /// "Tax summary"
    pub tax_summary: String,
    /// "Net": the amount before tax, in the tax summary.
    pub tax_net: String,
    /// "Tax": the tax amount, in the tax summary.
    pub tax_amount: String,
    /// "Prices include {tax}". `{tax}` becomes the shop's tax name, such as "VAT".
    pub prices_include_tax: String,
    /// "VOID": the heading of a void receipt.
    pub void_heading: String,
    /// "COPY": the heading of a reprinted receipt (rule PAY-10).
    pub copy_heading: String,
    /// "Reason": why a sale was voided or items were returned.
    pub reason: String,
    /// "RETURN": the heading of a return receipt.
    pub return_heading: String,
    /// "Return": the return number.
    pub return_no: String,
    /// "Original invoice"
    pub original_invoice: String,
    /// "Refund": the total paid back.
    pub refund_total: String,
    /// "Period": the dates a report covers.
    pub period: String,
    /// "{from} to {to}": a report's first and last day.
    pub date_range: String,
    /// "Packed": the packing date on a label.
    pub packed_on: String,
    /// "Expires": the expiry date on a label.
    pub expires_on: String,
    /// "{price} per {unit}". Example: "৳750.00 per kg".
    pub unit_price: String,
    /// "Was {price}": the normal price on a shelf tag during a promotion.
    pub regular_price: String,
    /// "Offer {from} to {to}": the promotion dates on a shelf tag.
    pub offer_dates: String,
}

/// Fills a template's `{name}` places. Example: ("Was {price}", [("price", "৳9.00")]) → "Was ৳9.00".
pub(crate) fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut text = template.to_string();
    for (name, value) in values {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_puts_values_in_their_places() {
        assert_eq!(
            fill(
                "Offer {from} to {to}",
                &[("from", "1 Oct"), ("to", "15 Oct")]
            ),
            "Offer 1 Oct to 15 Oct"
        );
        assert_eq!(fill("No places", &[("tax", "VAT")]), "No places");
    }
}
