//! The plain data that goes in and out of the inventory module: stock changes,
//! ledger rows and paging.

use pos_core::{BusinessDate, Id, Money, Qty, Timestamp};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::InventoryError;

/// Why stock changed. Stored in `inventory_ledger.kind` as lower snake case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum MoveKind {
    /// Sold at the till. Stock goes out.
    Sale,
    /// A sale was voided. Stock comes back on the same batches.
    SaleVoid,
    /// A customer brought items back. Stock comes in.
    Return,
    /// Goods received from a supplier. Stock comes in.
    Receive,
    /// Goods sent back to a supplier. Stock goes out.
    SupplierReturn,
    /// A manual correction with a reason. Either way.
    Adjust,
    /// A stock count found a difference. Either way.
    Count,
    /// Opening stock, when the shop starts using the app. Stock comes in.
    Opening,
}

impl MoveKind {
    /// The text stored in the database. Example: `SaleVoid` → "sale_void".
    pub fn as_str(self) -> &'static str {
        match self {
            MoveKind::Sale => "sale",
            MoveKind::SaleVoid => "sale_void",
            MoveKind::Return => "return",
            MoveKind::Receive => "receive",
            MoveKind::SupplierReturn => "supplier_return",
            MoveKind::Adjust => "adjust",
            MoveKind::Count => "count",
            MoveKind::Opening => "opening",
        }
    }

    /// Reads the text stored in the database back into a kind.
    pub fn parse(text: &str) -> Result<MoveKind, InventoryError> {
        let all = [
            MoveKind::Sale,
            MoveKind::SaleVoid,
            MoveKind::Return,
            MoveKind::Receive,
            MoveKind::SupplierReturn,
            MoveKind::Adjust,
            MoveKind::Count,
            MoveKind::Opening,
        ];
        all.into_iter()
            .find(|kind| kind.as_str() == text)
            .ok_or_else(|| InventoryError::BadData(format!("unknown stock kind '{text}'")))
    }

    /// Whether a change of this size fits the kind. A sale always takes stock out,
    /// a receive always brings it in; adjustments and counts go either way.
    pub fn allows(self, qty_delta: Qty) -> bool {
        match self {
            MoveKind::Sale | MoveKind::SupplierReturn => qty_delta.0 < 0,
            MoveKind::SaleVoid | MoveKind::Return | MoveKind::Receive | MoveKind::Opening => {
                qty_delta.0 > 0
            }
            MoveKind::Adjust | MoveKind::Count => qty_delta.0 != 0,
        }
    }
}

/// Which batch a stock change uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchChoice {
    /// First expiry, first out (INV-05). Only for stock going out. May split into several rows.
    Fefo,
    /// This exact batch.
    Specific(Id),
    /// No batch. For products that don't use batches.
    None,
}

/// Where and when a change happens and who makes it. `app` fills this in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveCtx {
    pub at: Timestamp,
    pub business_date: BusinessDate,
    pub till_id: Id,
    pub user_id: Option<Id>,
}

/// One stock change, as asked for by a workflow. `record` turns it into ledger rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Movement {
    pub product_id: Id,
    /// Signed: negative takes stock out. Example: `Qty(-3000)` is 3 pieces out.
    pub qty_delta: Qty,
    pub kind: MoveKind,
    pub batch: BatchChoice,
    /// Needed for `Adjust`. Must be an active reason that fits the direction.
    pub reason_code: Option<String>,
    pub note: Option<String>,
    /// The document behind the change, e.g. ("sale", invoice id).
    pub ref_: Option<(String, Id)>,
    /// Cost of one unit, copied onto every row.
    pub unit_cost: Money,
    /// The product needs an expiry date on stock coming in (from catalog, via `app`).
    pub tracks_expiry: bool,
    pub ctx: MoveCtx,
}

impl Movement {
    /// A change with no batch, reason, note, document or cost. Set other fields with
    /// `Movement { batch: BatchChoice::Fefo, ..Movement::new(…) }`.
    pub fn new(product_id: &Id, qty_delta: Qty, kind: MoveKind, ctx: MoveCtx) -> Movement {
        Movement {
            product_id: product_id.clone(),
            qty_delta,
            kind,
            batch: BatchChoice::None,
            reason_code: None,
            note: None,
            ref_: None,
            unit_cost: Money::zero(),
            tracks_expiry: false,
            ctx,
        }
    }
}

/// One ledger row that `record` or `reverse` wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedRow {
    pub id: Id,
    pub batch_id: Option<Id>,
    pub qty_delta: Qty,
}

/// A full row of `inventory_ledger`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerRow {
    pub id: Id,
    pub product_id: Id,
    pub batch_id: Option<Id>,
    pub qty_delta: Qty,
    pub kind: MoveKind,
    pub reason_code: Option<String>,
    pub note: Option<String>,
    pub ref_: Option<(String, Id)>,
    pub unit_cost: Money,
    pub at: Timestamp,
    pub business_date: BusinessDate,
    pub till_id: Id,
    pub user_id: Option<Id>,
}

/// Which part of a long list to return. Example: `Page { offset: 50, limit: 50 }` is page 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub offset: u32,
    pub limit: u32,
}

impl Page {
    /// The first `limit` rows.
    pub fn first(limit: u32) -> Page {
        Page { offset: 0, limit }
    }
}
