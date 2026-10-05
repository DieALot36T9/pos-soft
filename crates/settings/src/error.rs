//! The settings module's one error type, which turns into `AppError` for the UI.

use pos_core::AppError;

/// Everything that can go wrong in this module.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// The value breaks a rule, so it was not saved. Holds the `AppError::Validation`.
    #[error("invalid setting: {0}")]
    Invalid(AppError),
    /// The database refused a query.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    /// A stored value could not be read back. Example: the JSON has the wrong shape.
    #[error("setting {key} could not be read: {detail}")]
    Unreadable { key: String, detail: String },
}

/// Shorthand for results of this module.
pub type Result<T> = std::result::Result<T, SettingsError>;

impl From<SettingsError> for AppError {
    fn from(error: SettingsError) -> AppError {
        match error {
            SettingsError::Invalid(app_error) => app_error,
            other => AppError::internal(other.to_string()),
        }
    }
}
