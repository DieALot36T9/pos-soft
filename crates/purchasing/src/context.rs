//! Who did something, and when.

use pos_core::{BusinessDate, Id, Timestamp};

/// Who recorded a document or ledger row, and when. `app` fills it from the session and clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchasingCtx {
    /// The moment it happened.
    pub at: Timestamp,
    /// The trading day it belongs to (from `core::business_date`).
    pub business_date: BusinessDate,
    /// The signed-in user.
    pub user_id: Id,
}
