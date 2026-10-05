//! The sales module's one error type. It turns into `AppError` for the UI.

use std::collections::BTreeMap;

use pos_core::AppError;

/// Everything that can go wrong in the sales module.
#[derive(Debug, thiserror::Error)]
pub enum SalesError {
    /// SQLite failed. This is a bug or a broken database, never the user's fault.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    /// Money maths overflowed, or a stored value couldn't be read back.
    #[error(transparent)]
    Core(#[from] AppError),
    /// A cart, line, sale, return or payment method doesn't exist.
    #[error("{entity} {key} was not found")]
    NotFound { entity: &'static str, key: String },
    /// The user can fix this. Example: `return_qty_too_large`.
    #[error("not allowed: {code}")]
    Invalid {
        code: &'static str,
        params: BTreeMap<String, String>,
    },
    /// The action clashes with the state of the data. Example: `sale_already_voided`.
    #[error("conflict: {code}")]
    Conflict {
        code: &'static str,
        params: BTreeMap<String, String>,
    },
    /// The caller sent amounts that don't add up. This is a bug in the workflow.
    #[error("totals don't add up: {0}")]
    TotalsDontAddUp(String),
}

impl SalesError {
    /// A mistake the user can fix, with no extra details.
    pub(crate) fn invalid(code: &'static str) -> SalesError {
        SalesError::Invalid {
            code,
            params: BTreeMap::new(),
        }
    }

    /// A mistake the user can fix, with one detail for the message. Example: the maximum.
    pub(crate) fn invalid_with(code: &'static str, name: &str, value: String) -> SalesError {
        let mut params = BTreeMap::new();
        params.insert(name.to_string(), value);
        SalesError::Invalid { code, params }
    }

    /// The data is in a state that doesn't allow this action.
    pub(crate) fn conflict(code: &'static str) -> SalesError {
        SalesError::Conflict {
            code,
            params: BTreeMap::new(),
        }
    }

    /// Something with this id doesn't exist.
    pub(crate) fn not_found(entity: &'static str, key: &str) -> SalesError {
        SalesError::NotFound {
            entity,
            key: key.to_string(),
        }
    }

    /// The stable code the UI turns into a message, if this error has one.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            SalesError::Invalid { code, .. } | SalesError::Conflict { code, .. } => Some(code),
            _ => None,
        }
    }
}

impl From<SalesError> for AppError {
    fn from(error: SalesError) -> AppError {
        match error {
            SalesError::Core(inner) => inner,
            SalesError::NotFound { entity, key } => AppError::NotFound {
                entity: entity.to_string(),
                key,
            },
            SalesError::Invalid { code, params } => AppError::Validation {
                field: None,
                code: code.to_string(),
                params,
            },
            SalesError::Conflict { code, params } => AppError::Conflict {
                code: code.to_string(),
                params,
            },
            // Database failures and bad totals are bugs: the user sees only a reference id.
            SalesError::Database(inner) => AppError::internal(format!("sales: {inner}")),
            SalesError::TotalsDontAddUp(detail) => {
                AppError::internal(format!("sales: totals don't add up: {detail}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_mistakes_become_validation_errors_with_their_code() {
        let error: AppError =
            SalesError::invalid_with("return_qty_too_large", "max", "2000".into()).into();
        let AppError::Validation { code, params, .. } = error else {
            panic!("expected a validation error");
        };
        assert_eq!(code, "return_qty_too_large");
        assert_eq!(params.get("max").map(String::as_str), Some("2000"));
    }

    #[test]
    fn totals_that_dont_add_up_are_internal_errors() {
        let error: AppError = SalesError::TotalsDontAddUp("gross".into()).into();
        assert!(matches!(error, AppError::Internal { .. }));
    }

    #[test]
    fn conflicts_and_missing_rows_keep_their_meaning() {
        let error: AppError = SalesError::conflict("sale_already_voided").into();
        assert!(matches!(error, AppError::Conflict { code, .. } if code == "sale_already_voided"));
        let error: AppError = SalesError::not_found("sale", "T1-000001").into();
        assert!(matches!(error, AppError::NotFound { entity, .. } if entity == "sale"));
    }
}
