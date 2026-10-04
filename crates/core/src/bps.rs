//! Rates in basis points (hundredths of a percent).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A rate in hundredths of a percent. `Bps(1500)` is 15%, `Bps(750)` is 7.5%.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS,
)]
#[serde(transparent)]
#[ts(export)]
pub struct Bps(pub u32);
