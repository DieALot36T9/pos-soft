//! The data this module takes in and gives back. The UI sees these through ts-rs.

use pos_core::{Bps, Id, Money};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The tax result for one sale line. Always `net + tax == gross`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LineTax {
    /// The amount without tax.
    pub net: Money,
    /// The tax on the line.
    pub tax: Money,
    /// What the customer pays for the line.
    pub gross: Money,
}

/// A named part of a rate, shown on receipts and the tax report (rule TAX-03).
/// Example: "CGST" 9% inside "GST 18%".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RateComponent {
    pub name: String,
    pub rate_bps: Bps,
}

/// A tax rate as saved, with its components in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RateWithComponents {
    pub id: Id,
    /// Example: "Standard 15%".
    pub name: String,
    /// The whole rate. Example: `Bps(1500)` is 15%.
    pub rate_bps: Bps,
    /// False once deactivated. A deactivated rate can't be chosen for new products.
    pub active: bool,
    /// Lists show smaller numbers first.
    pub sort: i64,
    /// Empty, or parts that add up to `rate_bps`.
    pub components: Vec<RateComponent>,
}

/// What an admin enters to add a rate (rule TAX-02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NewRate {
    pub name: String,
    /// From 0 to 10000 (0% to 100%, with up to 2 decimals).
    pub rate_bps: Bps,
    /// Leave empty for a rate with no parts.
    pub components: Vec<RateComponent>,
    pub sort: i64,
}

/// What an admin enters to change a rate. It replaces the name, rate, parts and sort.
/// Rule TAX-07: past sales keep the rate they stored, so this affects only future sales.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RateUpdate {
    pub name: String,
    pub rate_bps: Bps,
    pub components: Vec<RateComponent>,
    pub sort: i64,
}
