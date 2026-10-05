//! The plain data shapes this module takes and gives back.

use pos_core::{BusinessDate, Id, Money, Timestamp};

use crate::ShiftsError;

/// The payment method kind that goes in and out of the cash drawer.
/// Only movements with this method count toward expected cash.
pub const CASH: &str = "cash";

/// Whether a shift is still running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftStatus {
    Open,
    Closed,
}

impl ShiftStatus {
    /// The text stored in the `status` column.
    pub fn as_str(self) -> &'static str {
        match self {
            ShiftStatus::Open => "open",
            ShiftStatus::Closed => "closed",
        }
    }

    pub(crate) fn parse(text: &str) -> Result<ShiftStatus, ShiftsError> {
        match text {
            "open" => Ok(ShiftStatus::Open),
            "closed" => Ok(ShiftStatus::Closed),
            other => Err(stored_value_invalid("status", other)),
        }
    }
}

/// Why money moved. Each kind has a fixed sign: + into the till, − out of it.
/// The order here is the order totals are listed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MovementKind {
    /// The opening float (+). Only `open` writes it.
    Float,
    /// A customer paid for a sale (+). Record the amount kept, after change.
    SalePayment,
    /// Money paid back for a return (−).
    Refund,
    /// Cash put into the drawer, for example extra change (+). Needs a reason.
    CashIn,
    /// Cash paid out of the drawer, for example to a delivery person (−). Needs a reason.
    CashOut,
    /// Cash taken to the safe (−). Needs a reason.
    CashDrop,
    /// A customer paid off some of their debt (+).
    CustomerPayment,
    /// A supplier was paid from the till (−).
    SupplierPayment,
    /// Undoes the movements of a voided document. Only `reverse_ref` writes it.
    VoidReversal,
}

impl MovementKind {
    /// The text stored in the `kind` column.
    pub fn as_str(self) -> &'static str {
        match self {
            MovementKind::Float => "float",
            MovementKind::SalePayment => "sale_payment",
            MovementKind::Refund => "refund",
            MovementKind::CashIn => "cash_in",
            MovementKind::CashOut => "cash_out",
            MovementKind::CashDrop => "cash_drop",
            MovementKind::CustomerPayment => "customer_payment",
            MovementKind::SupplierPayment => "supplier_payment",
            MovementKind::VoidReversal => "void_reversal",
        }
    }

    pub(crate) fn parse(text: &str) -> Result<MovementKind, ShiftsError> {
        let kind = match text {
            "float" => MovementKind::Float,
            "sale_payment" => MovementKind::SalePayment,
            "refund" => MovementKind::Refund,
            "cash_in" => MovementKind::CashIn,
            "cash_out" => MovementKind::CashOut,
            "cash_drop" => MovementKind::CashDrop,
            "customer_payment" => MovementKind::CustomerPayment,
            "supplier_payment" => MovementKind::SupplierPayment,
            "void_reversal" => MovementKind::VoidReversal,
            other => return Err(stored_value_invalid("kind", other)),
        };
        Ok(kind)
    }
}

/// One movement of money in a shift. Amounts are signed: + into the till, − out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftMovement {
    pub kind: MovementKind,
    /// The payment method from settings, if there is one. Example: the bKash method.
    pub method_id: Option<Id>,
    /// "cash", "card", "wallet", … Only "cash" counts toward expected cash.
    pub method_kind: String,
    pub amount: Money,
    /// The document this belongs to, for example "sale" or "return". Used by `reverse_ref`.
    pub ref_type: Option<String>,
    pub ref_id: Option<Id>,
    /// Why the money moved. Required for cash in, cash out and cash drops (SHF-03).
    pub reason: Option<String>,
    pub at: Timestamp,
    pub user_id: Id,
}

/// One shift as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shift {
    pub id: Id,
    pub till_id: Id,
    /// The cashier who owns the shift.
    pub user_id: Id,
    pub business_date: BusinessDate,
    pub status: ShiftStatus,
    pub opened_at: Timestamp,
    pub opening_float: Money,
    pub closed_at: Option<Timestamp>,
    pub closed_by: Option<Id>,
    /// True when an admin closed the shift for someone else (SHF-07).
    pub forced: bool,
    /// The cash figures saved at close. Empty while the shift is open.
    pub expected_cash: Option<Money>,
    pub counted_cash: Option<Money>,
    /// Counted − expected. A shortage is negative. Example: −50.00.
    pub difference: Option<Money>,
    pub close_note: Option<String>,
}

/// How many notes or coins of one value were counted. Example: 12 × 1,000.00.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DenominationCount {
    pub denomination: Money,
    pub count: u32,
}

/// What `close` worked out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseResult {
    pub expected_cash: Money,
    /// Empty when a force close had no count.
    pub counted_cash: Option<Money>,
    /// Counted − expected. A shortage is negative.
    pub difference: Option<Money>,
}

/// The total of one movement kind paid with one method kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovementTotal {
    pub kind: MovementKind,
    pub method_kind: String,
    /// How many movements were added up.
    pub count: u32,
    pub total: Money,
}

/// Payments for one method: sales − refunds, after voids (SHF-08).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodTotal {
    pub method_kind: String,
    pub method_id: Option<Id>,
    pub total: Money,
}

/// Everything the X and Z reports need from this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftSummary {
    pub shift: Shift,
    /// Totals by kind, then method kind. Every kind and every method is included.
    pub totals: Vec<MovementTotal>,
    /// Sale payments, refunds and void reversals, by method.
    pub payments_by_method: Vec<MethodTotal>,
    /// Worked out from the movements, so it is up to date while the shift is open.
    pub expected_cash: Money,
    /// Smallest denomination first. Empty if nothing was counted.
    pub open_counts: Vec<DenominationCount>,
    pub close_counts: Vec<DenominationCount>,
}

/// A stored value this module never writes. Something outside the module changed the row.
pub(crate) fn stored_value_invalid(column: &str, value: &str) -> ShiftsError {
    ShiftsError::Core(pos_core::AppError::internal(format!(
        "shifts: stored {column} {value:?} is not valid"
    )))
}
