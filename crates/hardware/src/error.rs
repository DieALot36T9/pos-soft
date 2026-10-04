//! The one error type of this module. It turns into `AppError::Hardware`.

use pos_core::AppError;

/// Which kind of device an error came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    Printer,
    Scale,
}

impl Device {
    /// The name the UI receives, e.g. "printer".
    pub fn as_str(self) -> &'static str {
        match self {
            Device::Printer => "printer",
            Device::Scale => "scale",
        }
    }
}

impl std::fmt::Display for Device {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Everything that can go wrong when talking to a device.
/// The text after each variant goes to the log file; the UI only uses the code.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HwError {
    /// The device didn't answer in time. Code `timeout`.
    #[error("the {device} did not answer within {waited_ms} ms")]
    Timeout { device: Device, waited_ms: u64 },
    /// The device couldn't be reached at all. Example: wrong IP address or port. Code `not_connected`.
    #[error("could not connect to the {device}: {detail}")]
    NotConnected { device: Device, detail: String },
    /// The connection broke while sending or receiving. Code `send_failed`.
    #[error("sending to the {device} failed: {detail}")]
    SendFailed { device: Device, detail: String },
    /// The device (or Windows) refused the data. Example: a printer that can't take raw data. Code `refused`.
    #[error("the {device} refused the data: {detail}")]
    Refused { device: Device, detail: String },
    /// The scale sent data, but no weight could be found in it. `raw` is what it sent. Code `no_weight`.
    #[error("no weight found in the scale data: {raw:?}")]
    NoWeight { raw: String },
    /// The device settings can't work. Example: a scale pattern without a number. Code `bad_settings`.
    #[error("the {device} settings are wrong: {detail}")]
    BadSettings { device: Device, detail: String },
}

impl HwError {
    /// The device the error came from.
    pub fn device(&self) -> Device {
        match self {
            HwError::Timeout { device, .. }
            | HwError::NotConnected { device, .. }
            | HwError::SendFailed { device, .. }
            | HwError::Refused { device, .. }
            | HwError::BadSettings { device, .. } => *device,
            HwError::NoWeight { .. } => Device::Scale,
        }
    }

    /// The short code the UI turns into a message, e.g. "timeout".
    pub fn code(&self) -> &'static str {
        match self {
            HwError::Timeout { .. } => "timeout",
            HwError::NotConnected { .. } => "not_connected",
            HwError::SendFailed { .. } => "send_failed",
            HwError::Refused { .. } => "refused",
            HwError::NoWeight { .. } => "no_weight",
            HwError::BadSettings { .. } => "bad_settings",
        }
    }
}

impl From<HwError> for AppError {
    fn from(error: HwError) -> AppError {
        AppError::Hardware {
            device: error.device().as_str().to_string(),
            code: error.code().to_string(),
            detail: error.to_string(),
        }
    }
}
