//! What can go wrong in the tax module, and how the UI hears about it.

use std::collections::BTreeMap;

use pos_core::{AppError, Id};

/// The result of every fallible function in this module.
pub type Result<T> = std::result::Result<T, TaxError>;

/// Everything that can go wrong in the tax module.
#[derive(Debug, thiserror::Error)]
pub enum TaxError {
    /// The rate's name is empty.
    #[error("the tax rate needs a name")]
    NameMissing,
    /// The rate is above 100% (10000 bps). Rule TAX-02.
    #[error("the tax rate must be from 0% to 100%")]
    RateOutOfRange,
    /// A component's name is empty. Rule TAX-03.
    #[error("every part of a tax rate needs a name")]
    ComponentNameMissing,
    /// The components don't add up to the rate. Rule TAX-03.
    #[error("the parts of a tax rate must add up to the rate")]
    ComponentsDoNotAddUp,
    /// Another rate already has this name (capitals ignored).
    #[error("another tax rate already has this name")]
    NameTaken,
    /// No rate has this id.
    #[error("tax rate {id} was not found")]
    NotFound { id: Id },
    /// SQLite refused something. The details go to the log.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// A money helper from `core` failed, for example an amount too large to hold.
    #[error(transparent)]
    Core(#[from] AppError),
}

impl From<TaxError> for AppError {
    fn from(error: TaxError) -> AppError {
        match error {
            TaxError::NameMissing => validation("name", "tax_rate_name_missing"),
            TaxError::RateOutOfRange => validation("rate_bps", "tax_rate_out_of_range"),
            TaxError::ComponentNameMissing => {
                validation("components", "tax_component_name_missing")
            }
            TaxError::ComponentsDoNotAddUp => {
                validation("components", "tax_components_do_not_add_up")
            }
            TaxError::NameTaken => AppError::Conflict {
                code: "tax_rate_name_taken".to_string(),
                params: BTreeMap::new(),
            },
            TaxError::NotFound { id } => AppError::NotFound {
                entity: "tax_rate".to_string(),
                key: id.to_string(),
            },
            TaxError::Sqlite(sqlite_error) => {
                AppError::internal(format!("database error: {sqlite_error}"))
            }
            TaxError::Core(app_error) => app_error,
        }
    }
}

fn validation(field: &str, code: &str) -> AppError {
    AppError::Validation {
        field: Some(field.to_string()),
        code: code.to_string(),
        params: BTreeMap::new(),
    }
}
