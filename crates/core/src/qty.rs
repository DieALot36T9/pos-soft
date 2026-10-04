//! Quantities in thousandths of the product's unit (ADR-007).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A quantity in thousandths of the product's unit. `Qty(1000)` is 1 piece, 1 kg or 1 litre.
/// `Qty(1234)` is 1.234 kg.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS,
)]
#[serde(transparent)]
#[ts(export)]
pub struct Qty(pub i64);
