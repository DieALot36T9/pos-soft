//! The io module's one error type. Every problem here is something the user can
//! fix (choose another file, close the file in Excel), so each one becomes an
//! `AppError::Validation` with a code the UI turns into plain English.

use std::collections::BTreeMap;

use pos_core::AppError;

/// Everything that can go wrong while reading or writing a file.
#[derive(Debug, thiserror::Error)]
pub enum IoError {
    /// Only .csv and .xlsx files can be read and written.
    #[error("unknown file type: {extension}")]
    UnknownFileType { extension: String },
    /// The file is missing, locked or broken. `detail` goes to the log only.
    #[error("could not read the file: {detail}")]
    FileUnreadable { detail: String },
    /// A .csv file that is not saved as UTF-8. Example: Excel's plain "CSV" type on Windows.
    #[error("the file is not UTF-8 text")]
    FileNotUtf8,
    /// The file has no header row.
    #[error("the file is empty")]
    FileEmpty,
    /// The workbook has no sheet with this name.
    #[error("no sheet called {sheet}")]
    SheetNotFound { sheet: String },
    /// The file could not be saved. Example: it is open in Excel. `detail` goes to the log only.
    #[error("could not save the file: {detail}")]
    FileNotSaved { detail: String },
}

impl IoError {
    /// The stable code the UI turns into a message.
    pub fn code(&self) -> &'static str {
        match self {
            IoError::UnknownFileType { .. } => "io_unknown_file_type",
            IoError::FileUnreadable { .. } => "io_file_unreadable",
            IoError::FileNotUtf8 => "io_file_not_utf8",
            IoError::FileEmpty => "io_file_empty",
            IoError::SheetNotFound { .. } => "io_sheet_not_found",
            IoError::FileNotSaved { .. } => "io_file_not_saved",
        }
    }
}

impl From<IoError> for AppError {
    fn from(error: IoError) -> AppError {
        let mut params = BTreeMap::new();
        match &error {
            IoError::UnknownFileType { extension } => {
                params.insert("extension".to_string(), extension.clone());
            }
            IoError::SheetNotFound { sheet } => {
                params.insert("sheet".to_string(), sheet.clone());
            }
            IoError::FileUnreadable { .. }
            | IoError::FileNotUtf8
            | IoError::FileEmpty
            | IoError::FileNotSaved { .. } => {}
        }
        AppError::Validation {
            field: None,
            code: error.code().to_string(),
            params,
        }
    }
}

/// Short form used by every function in this crate.
pub type Result<T> = std::result::Result<T, IoError>;

/// Turns a library's read error into ours, keeping its text for the log.
pub(crate) fn unreadable(error: impl std::fmt::Display) -> IoError {
    IoError::FileUnreadable {
        detail: error.to_string(),
    }
}

/// Turns a library's write error into ours, keeping its text for the log.
pub(crate) fn not_saved(error: impl std::fmt::Display) -> IoError {
    IoError::FileNotSaved {
        detail: error.to_string(),
    }
}
