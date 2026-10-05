//! Completed sales: the snapshot the workflow sends in, and the sale read back.
//! A sale copies every name, price, tax and cost it used (ADR-011), so reprints,
//! returns and reports never depend on today's catalog.

use pos_core::{Bps, BusinessDate, Id, Money, Qty, Timestamp};
use serde::{Deserialize, Serialize};

use crate::PaymentKind;

/// Whether shelf prices include tax or have tax added on top (ADR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxMode {
    /// Prices include tax: gross = amount, net = gross − tax.
    Inclusive,
    /// Tax is added: net = amount, gross = net + tax.
    Exclusive,
}

impl TaxMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            TaxMode::Inclusive => "inclusive",
            TaxMode::Exclusive => "exclusive",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<TaxMode> {
        match text {
            "inclusive" => Some(TaxMode::Inclusive),
            "exclusive" => Some(TaxMode::Exclusive),
            _ => None,
        }
    }
}

/// One part of a split tax, such as CGST in GST 18% = CGST 9% + SGST 9%.
/// Stored in `tax_components_json` as `[{"name":"CGST","bps":900,"tax_minor":652}]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxPart {
    pub name: String,
    pub bps: Bps,
    #[serde(rename = "tax_minor")]
    pub tax: Money,
}

/// The promotion that gave a line its discount (ADR-019: at most one per line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionSnapshot {
    pub id: Id,
    /// Shown on the receipt. Example: "Buy 2 get 1".
    pub name: String,
}

/// The customer as they were when the sale was made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomerSnapshot {
    pub id: Id,
    pub name: String,
    pub phone: String,
}

/// One fully priced sale line, worked out by pricing and tax in the workflow.
/// Sales checks that its amounts add up, but never calculates a price or tax itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineSnapshot {
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    /// Example: "kg", "pc".
    pub unit_symbol: String,
    pub category_id: Id,
    pub brand_id: Option<Id>,
    pub qty: Qty,
    /// The price for one unit that was used, after any price list or override.
    pub unit_price: Money,
    /// The normal shelf price for one unit, for "you saved" and reports.
    pub retail_price: Money,
    /// unit price × quantity, or the sticker price for price-embedded labels.
    pub base: Money,
    pub promotion: Option<PromotionSnapshot>,
    pub promo_discount: Money,
    pub tier_discount: Money,
    pub manual_discount: Money,
    /// This line's share of the discount on the whole invoice.
    pub invoice_discount: Money,
    pub net: Money,
    pub tax: Money,
    pub gross: Money,
    pub tax_rate_id: Id,
    /// Example: "VAT 15%".
    pub tax_name: String,
    pub tax_bps: Bps,
    /// Empty when the tax has no parts.
    pub tax_components: Vec<TaxPart>,
    /// The product's cost for one unit at the time of sale (moving average).
    pub unit_cost: Money,
    pub price_overridden: bool,
}

/// The sale's totals. Each one must equal the sum of the same amount over the lines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SaleTotals {
    pub base: Money,
    pub promo_discount: Money,
    pub tier_discount: Money,
    pub manual_discount: Money,
    pub invoice_discount: Money,
    pub net: Money,
    pub tax: Money,
    pub gross: Money,
    /// Cash rounding (SET-06). total = gross + rounding. Not taxed.
    pub rounding: Money,
    /// The amount due.
    pub total: Money,
    /// Cash given back to the customer.
    pub change: Money,
    /// Σ round(unit cost × quantity) over the lines.
    pub cost: Money,
}

/// One payment on a sale or a refund on a return, as it will be stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPayment {
    pub method_id: Id,
    /// The part of the total this payment covers.
    pub amount: Money,
    /// Cash only: what the customer handed over. Change = tendered − amount.
    pub tendered: Option<Money>,
    /// A transaction id or similar (PAY-02). Never a card number.
    pub reference: Option<String>,
}

/// Everything `complete` needs: the header, the priced lines and the payments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedSale {
    /// The cart being completed. It is deleted in the same transaction.
    pub cart_id: Id,
    pub till_id: Id,
    /// The till's short code, used in the invoice number. Example: "T1".
    pub till_code: String,
    pub shift_id: Id,
    pub user_id: Id,
    pub business_date: BusinessDate,
    pub completed_at: Timestamp,
    pub customer: Option<CustomerSnapshot>,
    pub price_list_id: Option<Id>,
    pub tax_mode: TaxMode,
    /// The return this sale is the other half of, in an exchange (RET-05).
    pub exchange_return_id: Option<Id>,
    pub totals: SaleTotals,
    /// In the same order as the cart's lines.
    pub lines: Vec<LineSnapshot>,
    pub payments: Vec<NewPayment>,
}

/// The id and invoice number of a sale that was just saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaleRef {
    pub id: Id,
    /// Example: "T1-000123".
    pub invoice_no: String,
}

/// Whether a sale stands or was voided (RET-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaleStatus {
    Completed,
    Voided,
}

impl SaleStatus {
    pub(crate) fn parse(text: &str) -> Option<SaleStatus> {
        match text {
            "completed" => Some(SaleStatus::Completed),
            "voided" => Some(SaleStatus::Voided),
            _ => None,
        }
    }
}

/// Who voided a sale, when and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoidInfo {
    pub at: Timestamp,
    pub by: Id,
    pub approved_by: Option<Id>,
    pub reason: String,
}

/// A completed sale read back from the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sale {
    pub id: Id,
    pub invoice_no: String,
    pub till_id: Id,
    pub shift_id: Id,
    pub user_id: Id,
    pub business_date: BusinessDate,
    pub completed_at: Timestamp,
    pub customer: Option<CustomerSnapshot>,
    pub price_list_id: Option<Id>,
    pub status: SaleStatus,
    pub tax_mode: TaxMode,
    pub exchange_return_id: Option<Id>,
    pub totals: SaleTotals,
    pub void: Option<VoidInfo>,
    pub lines: Vec<SaleLine>,
    pub payments: Vec<Payment>,
}

/// A stored sale line: the snapshot plus its id, position and cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaleLine {
    pub id: Id,
    pub seq: u32,
    pub snapshot: LineSnapshot,
    /// round(unit cost × quantity).
    pub cost: Money,
}

/// A stored payment or refund.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payment {
    pub id: Id,
    pub method_id: Id,
    /// Copied from the method when the payment was made.
    pub kind: PaymentKind,
    pub amount: Money,
    pub tendered: Option<Money>,
    pub reference: Option<String>,
    pub at: Timestamp,
}
