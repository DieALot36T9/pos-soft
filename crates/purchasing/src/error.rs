//! The purchasing module's one error type. It turns into `AppError` for the UI.

use std::collections::BTreeMap;

use pos_core::AppError;

/// Everything that can go wrong in purchasing. Codes are snake_case words the UI
/// turns into plain English. Example: `supplier_name_missing`.
#[derive(Debug, thiserror::Error)]
pub enum PurchasingError {
    /// The user can fix what they typed. `field` names the form field.
    #[error("{field}: {code}")]
    Invalid {
        field: &'static str,
        code: &'static str,
    },
    /// A supplier, order or GRN with this id doesn't exist.
    #[error("{entity} {key} was not found")]
    NotFound { entity: &'static str, key: String },
    /// The action doesn't fit the current state. Example: `order_not_draft`.
    #[error("conflict: {code}")]
    Conflict { code: &'static str },
    /// A money sum overflowed, or another bug found by `core`.
    #[error(transparent)]
    Core(#[from] AppError),
    /// SQLite failed. This is a bug or a broken database, never the user's fault.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
}

impl PurchasingError {
    /// A "not found" error for a row id.
    pub(crate) fn not_found(entity: &'static str, key: &pos_core::Id) -> PurchasingError {
        PurchasingError::NotFound {
            entity,
            key: key.as_str().to_string(),
        }
    }
}

impl From<PurchasingError> for AppError {
    fn from(error: PurchasingError) -> AppError {
        match error {
            PurchasingError::Invalid { field, code } => AppError::Validation {
                field: Some(field.to_string()),
                code: code.to_string(),
                params: BTreeMap::new(),
            },
            PurchasingError::NotFound { entity, key } => AppError::NotFound {
                entity: entity.to_string(),
                key,
            },
            PurchasingError::Conflict { code } => AppError::Conflict {
                code: code.to_string(),
                params: BTreeMap::new(),
            },
            PurchasingError::Core(inner) => inner,
            PurchasingError::Database(inner) => AppError::internal(format!("purchasing: {inner}")),
        }
    }
}
