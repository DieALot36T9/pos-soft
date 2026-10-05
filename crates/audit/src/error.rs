//! What can go wrong while recording or reading audit events.

use pos_core::AppError;

/// Everything that can go wrong in the audit module.
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    /// SQLite refused something. The details go to the log.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// An action name isn't lower-case words joined by dots, like `sale.void`. A bug in the caller.
    #[error("audit action {0:?} is not a valid name")]
    BadAction(String),
    /// `details` must be a JSON object, like `{"reason": "wrong item"}`. A bug in the caller.
    #[error("audit details must be a JSON object")]
    DetailsNotObject,
    /// `details` has a key that sounds like a secret. Only the key is kept, never the value.
    #[error("audit details must not hold secrets, but have the key {0:?}")]
    SecretInDetails(String),
    /// The page size is 0 or larger than `AuditPage::MAX_LIMIT`.
    #[error("page limit {0} is out of range")]
    BadPageLimit(u32),
    /// A stored row couldn't be read back, for example a broken id.
    #[error("audit row {id} can't be read: {detail}")]
    BadStoredRow { id: String, detail: String },
}

impl From<AuditError> for AppError {
    fn from(error: AuditError) -> AppError {
        match error {
            // The screen sends the page size, so this is the only error a user could cause.
            AuditError::BadPageLimit(_) => AppError::validation("page_limit_invalid"),
            other => AppError::internal(other.to_string()),
        }
    }
}
