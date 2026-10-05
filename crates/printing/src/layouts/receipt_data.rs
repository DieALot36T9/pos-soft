//! The data a receipt prints. `app` fills these from sales, returns, customers
//! and settings; every amount is already worked out.

use pos_core::{Bitmap1, CurrencyFormat, Money, Qty};

use crate::doc::PaperWidth;

/// The shop's details at the top of every receipt (from the shop profile, rule SET-04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopHeader {
    pub name: String,
    pub address_lines: Vec<String>,
    /// Printed as it is. Empty means not printed.
    pub phone: String,
    /// The tax number's name. Example: "BIN", "VAT No", "GSTIN".
    pub tax_reg_label: String,
    /// Empty means not printed.
    pub tax_reg_no: String,
}

/// How receipts look: paper, currency, shop details and the options of rule SET-14.
/// Copies (1–3) are not here: `app` sends the same bytes again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptLayout {
    pub paper: PaperWidth,
    pub currency: CurrencyFormat,
    pub shop: ShopHeader,
    /// The logo, already black and white (see `logo_from_image`).
    pub logo: Option<Bitmap1>,
    pub header_lines: Vec<String>,
    pub footer_lines: Vec<String>,
    pub show_logo: bool,
    pub show_tax_breakdown: bool,
    pub show_tax_reg_no: bool,
    /// The invoice barcode lets a return find the sale by scanning (rule RET-01).
    pub show_invoice_barcode: bool,
    pub show_customer_points: bool,
}

/// One sold or returned item, with amounts already worked out by `sales`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptLine {
    pub name: String,
    pub qty: Qty,
    /// The unit for weighed items. Example: "kg". Empty for items sold by the piece.
    pub unit: String,
    /// Price for one unit. Example: Money(9000) → 90.00
    pub unit_price: Money,
    /// The line amount before the line discount.
    pub amount: Money,
    pub discount: Option<LineDiscount>,
}

/// A discount on one line, such as a promotion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDiscount {
    /// Example: "Buy 2 get 1"
    pub label: String,
    /// As it should print: negative for a saving. Example: Money(-5000) → -50.00
    pub amount: Money,
}

/// One row of the tax summary (rule TAX-08).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxSummaryLine {
    /// Example: "VAT 15%"
    pub name: String,
    pub net: Money,
    pub tax: Money,
}

/// One payment or refund.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentLine {
    /// Example: "Cash", "bKash"
    pub method: String,
    pub amount: Money,
    /// Card or wallet reference, if one was taken.
    pub reference: Option<String>,
}

/// The customer attached to the sale, and their balances after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomerOnReceipt {
    pub name: String,
    pub points_balance: Option<i64>,
    /// What the customer owes on credit, if they have a credit account.
    pub credit_balance: Option<Money>,
}

/// Everything a sale receipt prints. A void receipt is the same with `void_reason` set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptData {
    /// Example: "T1-000123"
    pub invoice_no: String,
    /// Date and time, already written the shop's way by `app`. Example: "2026-10-04 14:32"
    pub printed_at: String,
    pub till: String,
    pub cashier: String,
    pub customer: Option<CustomerOnReceipt>,
    pub lines: Vec<ReceiptLine>,
    pub subtotal: Money,
    /// Discounts on the whole sale, as they should print. Zero is left out.
    pub discount_total: Money,
    pub tax_summary: Vec<TaxSummaryLine>,
    /// The shop's tax name. Example: "VAT".
    pub tax_label: String,
    pub prices_include_tax: bool,
    /// Cash rounding. Zero is left out.
    pub rounding: Money,
    pub total: Money,
    pub payments: Vec<PaymentLine>,
    /// Zero is left out.
    pub change: Money,
    /// Set for a void receipt (rule RET-04): prints the void heading and this reason.
    pub void_reason: Option<String>,
    /// The invoice number as a barcode picture, from the `barcode` crate.
    pub invoice_barcode: Option<Bitmap1>,
    /// Text for a QR code at the bottom, if any. Example: a link to an e-receipt.
    pub qr_data: Option<String>,
}

/// Everything a return receipt prints (rule RET-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnData {
    /// Example: "T1-R000045"
    pub return_no: String,
    /// `None` for a return without a receipt (rule RET-03).
    pub original_invoice_no: Option<String>,
    pub printed_at: String,
    pub till: String,
    pub cashier: String,
    pub customer: Option<CustomerOnReceipt>,
    pub lines: Vec<ReceiptLine>,
    pub tax_summary: Vec<TaxSummaryLine>,
    pub tax_label: String,
    pub prices_include_tax: bool,
    pub refund_total: Money,
    pub refunds: Vec<PaymentLine>,
    pub reason: Option<String>,
}
