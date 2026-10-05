//! The one error type of the printing module.

use std::collections::BTreeMap;

use pos_core::AppError;

/// Everything that can go wrong while drawing or encoding a printout.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PrintError {
    /// The shop logo file isn't a PNG or JPEG picture we can read.
    #[error("the logo file can't be read: {0}")]
    LogoUnreadable(String),
    /// The QR code text is longer than a QR code can hold.
    #[error("the QR code text is too long")]
    QrTooLong,
    /// The printout would be too big to draw (a bug in the data passed in).
    #[error("the picture is too big to draw: {width} x {height} dots")]
    TooBig { width: u32, height: u32 },
    /// A picture's `bits` don't match its width and height (a bug in the caller).
    #[error("the picture's data doesn't match its size")]
    BadPicture,
    /// The PNG encoder failed.
    #[error("PNG encoding failed: {0}")]
    Png(String),
    /// The `escpos` library refused a command.
    #[error("ESC/POS encoding failed: {0}")]
    EscPos(String),
}

impl From<PrintError> for AppError {
    fn from(error: PrintError) -> AppError {
        match error {
            // The admin picked a file that isn't a picture: they can choose another one.
            PrintError::LogoUnreadable(_) => AppError::Validation {
                field: Some("logo".to_string()),
                code: "logo_unreadable".to_string(),
                params: BTreeMap::new(),
            },
            PrintError::QrTooLong => AppError::validation("qr_text_too_long"),
            other => AppError::internal(other.to_string()),
        }
    }
}
