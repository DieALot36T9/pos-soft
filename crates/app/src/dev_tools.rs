//! The dev-only fake scale panel (rule HW-10). Built into debug builds, and into
//! release builds only with the `dev-tools` feature (the Mac demo build).

use std::collections::BTreeMap;
use std::time::Duration;

use hardware::{Reading, Scale, WeightUnit};
use pos_core::{AppError, parse_money};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::devices::Devices;

/// True when the dev tools are built in: every debug build, and release builds made
/// with `--features dev-tools`.
pub const ON: bool = cfg!(any(debug_assertions, feature = "dev-tools"));

/// A weight is typed with up to three digits after the point: thousandths.
const WEIGHT_DIGITS: u8 = 3;

/// What the fake scale panel shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct FakeScaleView {
    /// The weight with three decimals. Example: "1.234".
    pub weight: String,
    pub unit: WeightUnit,
    /// False while the weight is "moving".
    pub stable: bool,
    /// False makes the scale act unplugged.
    pub connected: bool,
}

/// What the panel sends when the user changes something.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct FakeScaleChange {
    /// The weight as typed. Example: "1.234".
    pub weight: String,
    pub unit: WeightUnit,
    pub stable: bool,
    pub connected: bool,
}

/// What the fake scale holds right now.
pub fn fake_scale_view(devices: &Devices) -> FakeScaleView {
    let state = devices.fake_scale.state();
    FakeScaleView {
        weight: weight_text(state.value_milli),
        unit: state.unit,
        stable: state.stable,
        connected: state.connected,
    }
}

/// Changes the fake scale. A weight that isn't a number changes nothing.
pub fn fake_scale_set(
    devices: &Devices,
    change: &FakeScaleChange,
) -> Result<FakeScaleView, AppError> {
    let value_milli =
        parse_money(&change.weight, WEIGHT_DIGITS).map_err(|_| AppError::Validation {
            field: Some("weight".into()),
            code: "weight_invalid".into(),
            params: BTreeMap::new(),
        })?;
    let scale = &devices.fake_scale;
    scale.set_weight(value_milli.0, change.unit);
    scale.set_stable(change.stable);
    scale.set_connected(change.connected);
    Ok(fake_scale_view(devices))
}

/// Reads the fake scale the way checkout will read a real one: it waits up to
/// `limit` for a stable weight.
pub fn fake_scale_read(devices: &Devices, limit: Duration) -> Result<Reading, AppError> {
    Ok(devices.fake_scale.read(limit)?)
}

/// Thousandths as text with three decimals. Example: -50 → "-0.050".
fn weight_text(value_milli: i64) -> String {
    let sign = if value_milli < 0 { "-" } else { "" };
    let whole = value_milli.unsigned_abs() / 1000;
    let thousandths = value_milli.unsigned_abs() % 1000;
    format!("{sign}{whole}.{thousandths:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weight_text_has_three_decimals() {
        assert_eq!(weight_text(0), "0.000");
        assert_eq!(weight_text(1234), "1.234");
        assert_eq!(weight_text(-50), "-0.050");
        assert_eq!(weight_text(12_000), "12.000");
    }
}
