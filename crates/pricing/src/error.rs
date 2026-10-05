//! What can go wrong in the pricing module.

use std::collections::BTreeMap;

use pos_core::{AppError, Id};

/// Everything that can go wrong in the pricing module. It turns into `AppError` for the UI.
#[derive(Debug, thiserror::Error)]
pub enum PricingError {
    /// SQLite refused something. The details go to the log.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// The user can fix what they typed. Example: field "name", code "tier_name_missing".
    #[error("{field} is not valid: {code}")]
    Invalid {
        field: &'static str,
        code: &'static str,
    },
    /// Another price list or tier already has this name.
    #[error("{code}: {name}")]
    NameTaken { code: &'static str, name: String },
    /// The price list, tier or promotion doesn't exist.
    #[error("{entity} {key} was not found")]
    NotFound { entity: &'static str, key: String },
    /// An error from a `core` helper, for example a money overflow.
    #[error(transparent)]
    App(#[from] AppError),
}

impl From<PricingError> for AppError {
    fn from(error: PricingError) -> AppError {
        match error {
            PricingError::Sqlite(sqlite) => AppError::internal(format!("database error: {sqlite}")),
            PricingError::Invalid { field, code } => AppError::Validation {
                field: Some(field.to_string()),
                code: code.to_string(),
                params: BTreeMap::new(),
            },
            PricingError::NameTaken { code, name } => AppError::Conflict {
                code: code.to_string(),
                params: BTreeMap::from([("name".to_string(), name)]),
            },
            PricingError::NotFound { entity, key } => AppError::NotFound {
                entity: entity.to_string(),
                key,
            },
            PricingError::App(app) => app,
        }
    }
}

/// A mistake in what the user typed, for the form field `field`.
pub(crate) fn invalid(field: &'static str, code: &'static str) -> PricingError {
    PricingError::Invalid { field, code }
}

/// The row with this id doesn't exist. `entity` is "price_list", "tier" or "promotion".
pub(crate) fn not_found(entity: &'static str, id: &Id) -> PricingError {
    PricingError::NotFound {
        entity,
        key: id.as_str().to_string(),
    }
}

/// Reads an id stored in one of our tables. A bad id there is a bug, not a user mistake.
pub(crate) fn stored_id(text: &str) -> Result<Id, PricingError> {
    Id::parse(text)
        .map_err(|_| AppError::internal(format!("pricing table holds a bad id: {text}")).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_turn_into_the_app_error_the_ui_understands() {
        let error = AppError::from(invalid("name", "tier_name_missing"));
        assert_eq!(
            error,
            AppError::Validation {
                field: Some("name".to_string()),
                code: "tier_name_missing".to_string(),
                params: BTreeMap::new(),
            }
        );

        let taken = AppError::from(PricingError::NameTaken {
            code: "tier_name_taken",
            name: "Gold".to_string(),
        });
        assert!(matches!(taken, AppError::Conflict { code, params }
            if code == "tier_name_taken" && params["name"] == "Gold"));

        let sqlite = AppError::from(PricingError::Sqlite(rusqlite::Error::InvalidQuery));
        assert!(matches!(sqlite, AppError::Internal { .. }));
    }

    #[test]
    fn a_bad_stored_id_is_an_internal_error() {
        let error = AppError::from(stored_id("nope").unwrap_err());
        assert!(matches!(error, AppError::Internal { .. }));
    }
}
