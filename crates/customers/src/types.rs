//! The plain data shapes that go in and out of this module.

use pos_core::{BusinessDate, Id, Money, Timestamp};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What the user types for a customer. Used to create and to edit one.
/// The phone is cleaned by `normalize_phone`; empty optional text is stored as nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CustomerDetails {
    pub phone: String,
    pub name: String,
    pub email: Option<String>,
    pub address: Option<String>,
    /// The pricing tier, owned by `pricing`. `app` checks it exists (rule CUS-07).
    pub tier_id: Option<Id>,
    pub credit_limit: Money,
    /// The customer doesn't want loyalty points.
    pub loyalty_opt_out: bool,
    pub notes: Option<String>,
}

impl CustomerDetails {
    /// Quick create at the till needs only a phone and a name (rule CUS-02).
    pub fn quick(phone: &str, name: &str) -> CustomerDetails {
        CustomerDetails {
            phone: phone.to_string(),
            name: name.to_string(),
            email: None,
            address: None,
            tier_id: None,
            credit_limit: Money::zero(),
            loyalty_opt_out: false,
            notes: None,
        }
    }
}

/// A saved customer record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Customer {
    pub id: Id,
    /// In E.164 form. Example: "+8801711123456".
    pub phone: String,
    pub name: String,
    pub email: Option<String>,
    pub address: Option<String>,
    pub tier_id: Option<Id>,
    pub credit_limit: Money,
    pub loyalty_opt_out: bool,
    pub notes: Option<String>,
    /// Set when the customer is archived. Archived customers are hidden at the till.
    pub archived_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// A customer with their current points and credit balance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CustomerWithBalances {
    pub customer: Customer,
    pub points: i64,
    /// Positive: the customer owes the shop. Negative: store credit the shop owes them.
    pub credit: Money,
    /// When the oldest debt that is still unpaid was made. Nothing when nothing is owed.
    pub oldest_unpaid_at: Option<Timestamp>,
}

/// One line in the till's customer search results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CustomerHit {
    pub id: Id,
    pub phone: String,
    pub name: String,
    pub tier_id: Option<Id>,
    pub points: i64,
    pub credit: Money,
}

/// Who did a ledger change, and when. `app` fills this in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerCtx {
    pub at: Timestamp,
    pub business_date: BusinessDate,
    pub user_id: Option<Id>,
}

/// The document a ledger row belongs to. Example: ref_type "sale" and the sale's id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerRef {
    pub ref_type: String,
    pub ref_id: Id,
}

/// How a customer paid off debt in the back office. Example: "bkash" and the transaction number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub method_kind: String,
    pub reference: Option<String>,
}

/// The kinds of points ledger rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PointsKind {
    /// Points earned on a sale. Always positive.
    Earn,
    /// Points spent as payment. Always negative.
    Redeem,
    /// Points given back or taken back by a return or void. Either sign.
    Reverse,
    /// An admin correction, with a reason. Either sign.
    Adjust,
}

/// The kinds of credit ledger rows. Positive means the customer owes more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CreditKind {
    /// A sale paid "on credit" (baki). Positive.
    CreditSale,
    /// The customer paid off debt. Negative.
    Payment,
    /// Returned goods taken off the customer's debt. Negative.
    Return,
    /// A refund given as store credit: the shop owes the customer. Negative.
    StoreCredit,
    /// Store credit spent on a sale. Positive.
    StoreCreditUsed,
    /// An admin correction. Either sign.
    Adjust,
    /// The opposite of a voided document's row. Either sign.
    VoidReversal,
}

/// The loyalty settings this module needs. `app` copies them from `settings`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoyaltySettingsView {
    pub enabled: bool,
    /// How much must be spent to earn points. Example: Money(10000) is 100.00.
    pub spend_unit: Money,
    /// Points earned per whole spend unit. Example: 1.
    pub points_per_unit: i64,
    /// What one point is worth when paying with points. Example: Money(10) is 0.10.
    pub value_per_point: Money,
    /// The smallest balance that can be used to pay. Example: 100 points.
    pub min_redeem: i64,
}

/// How a "Customer credit" payment is split (rule CUS-06): store credit first, then new debt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditSplit {
    pub store_credit_used: Money,
    pub new_debt: Money,
}

/// A customer's credit statement for a date range (rule CUS-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Statement {
    pub customer_id: Id,
    pub from: BusinessDate,
    pub to: BusinessDate,
    /// The balance before the first day.
    pub opening: Money,
    pub lines: Vec<StatementLine>,
    /// The balance at the end of the last day.
    pub closing: Money,
}

/// One credit ledger row on a statement, with the running balance after it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StatementLine {
    pub id: Id,
    pub at: Timestamp,
    pub business_date: BusinessDate,
    pub kind: CreditKind,
    pub amount: Money,
    pub method_kind: Option<String>,
    pub reference: Option<String>,
    pub ref_type: Option<String>,
    pub ref_id: Option<Id>,
    pub balance_after: Money,
}
