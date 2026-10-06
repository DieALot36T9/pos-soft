//! Every word printed on receipts, reports, labels and shelf tags, in English
//! (ADR-022). The `printing` layouts never type words of their own; another
//! language later adds a second function like this one.

use printing::ReceiptLabels;

/// The printed words in English.
pub fn english() -> ReceiptLabels {
    ReceiptLabels {
        invoice_no: "Invoice".into(),
        date: "Date".into(),
        till: "Till".into(),
        cashier: "Cashier".into(),
        customer: "Customer".into(),
        points: "Points".into(),
        credit_balance: "Balance due".into(),
        subtotal: "Subtotal".into(),
        discount: "Discount".into(),
        rounding: "Rounding".into(),
        total: "Total".into(),
        change: "Change".into(),
        reference: "Ref".into(),
        tax_summary: "Tax summary".into(),
        tax_net: "Net".into(),
        tax_amount: "Tax".into(),
        prices_include_tax: "Prices include {tax}".into(),
        void_heading: "VOID".into(),
        copy_heading: "COPY".into(),
        reason: "Reason".into(),
        return_heading: "RETURN".into(),
        return_no: "Return".into(),
        original_invoice: "Original invoice".into(),
        refund_total: "Refund".into(),
        period: "Period".into(),
        date_range: "{from} to {to}".into(),
        packed_on: "Packed".into(),
        expires_on: "Expires".into(),
        unit_price: "{price} per {unit}".into(),
        regular_price: "Was {price}".into(),
        offer_dates: "Offer {from} to {to}".into(),
    }
}
