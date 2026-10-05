//! Reading weights from a scale (rule HW-05). The scale is connected by serial or USB-serial.
//! A profile says how to ask the scale for a weight and how to find the weight in its answer.

mod fakes;
#[cfg(test)]
mod fixture_tests;
mod parse;
mod profiles;
mod serial_scale;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::HwError;

pub use fakes::{FakeScale, FakeScaleState, MockScale};
pub use parse::check_profile;
pub use profiles::builtin_scale_profiles;

/// Something that weighs. The real scale, the fake scale panel and test scales all are one.
pub trait Scale: Send + Sync {
    /// Waits up to `timeout` for a stable weight. If the weight is still moving when the
    /// time is up, returns the last weight with `stable: false`. If no weight came at all,
    /// returns an error (`Timeout`, or `NoWeight` if data came but had no weight in it).
    fn read(&self, timeout: Duration) -> Result<Reading, HwError>;
}

/// One weight from the scale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Reading {
    /// The weight in thousandths of `unit`. Example: 1.234 kg is 1234.
    pub value_milli: i64,
    pub unit: WeightUnit,
    /// True when the scale says the weight has settled.
    pub stable: bool,
    /// The line the scale sent, for troubleshooting in settings ("Read scale").
    pub raw: String,
}

/// The unit a scale reports its weight in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum WeightUnit {
    G,
    Kg,
    Lb,
}

/// How a scale talks, so its answers can be read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScaleProfile {
    pub mode: ScaleMode,
    /// A regular expression that finds the weight in one line from the scale. It must have a
    /// group named `value` (the number) and may have one named `unit` (g, kg or lb).
    /// Example: `(?P<value>[-+]?\d+\.\d+)\s*(?P<unit>kg|g|lb)`.
    pub pattern: String,
    /// The unit to use when the line has no unit in it.
    pub default_unit: WeightUnit,
    /// Text the scale puts in a line when the weight has settled. Example: "ST".
    /// `None` if the scale doesn't say; then `stable_repeats` decides.
    pub stable_marker: Option<String>,
    /// With no marker: the weight is stable once this many readings in a row are the same.
    /// Rule HW-05 uses 3.
    pub stable_repeats: u8,
}

/// Whether the scale sends weights by itself or must be asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum ScaleMode {
    /// The scale sends weight lines all the time.
    Continuous,
    /// The app sends `command` (e.g. "W", or the byte 0x05) and the scale answers with one line.
    Request { command: Vec<u8> },
}

/// The serial port settings from the scale's manual. Often 9600 baud, 8 data bits,
/// no parity, 1 stop bit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SerialSettings {
    /// Example: "COM3" on Windows, "/dev/cu.usbserial-1410" on macOS.
    pub port: String,
    pub baud: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DataBits {
    Seven,
    Eight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Parity {
    None,
    Odd,
    Even,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum StopBits {
    One,
    Two,
}

/// Makes the scale connection for these settings. The port is opened on each `read`,
/// so a scale that was unplugged and plugged in again works without restarting the app.
/// A wrong profile is reported by `read` as `BadSettings`; `check_profile` finds it earlier.
pub fn scale_from(port: &SerialSettings, profile: &ScaleProfile) -> Box<dyn Scale> {
    Box::new(serial_scale::SerialScale::new(
        port.clone(),
        profile.clone(),
    ))
}
