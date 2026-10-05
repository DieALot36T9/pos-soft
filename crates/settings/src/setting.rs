//! The `Setting` trait: what every typed setting has, and small helpers for `validate`.

use std::collections::BTreeMap;

use pos_core::AppError;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// One typed setting, stored as one row of `settings_values`.
pub trait Setting: Serialize + DeserializeOwned {
    /// The row's key. Example: "till".
    const KEY: &'static str;

    /// The value used until the shop saves its own. `CurrencyFormat` and `RoundingRule`
    /// live in `core` without a `Default`, so this is a method instead of `Default`.
    fn default_value() -> Self;

    /// Checks the value before it is saved. Returns an `AppError::Validation` naming the field.
    fn validate(&self) -> Result<(), AppError>;
}

/// A validation error for one field. Example: `invalid("till_code", "till_code_invalid")`.
pub(crate) fn invalid(field: &str, code: &str) -> AppError {
    AppError::Validation {
        field: Some(field.to_string()),
        code: code.to_string(),
        params: BTreeMap::new(),
    }
}

/// True when the text has something other than spaces in it.
pub(crate) fn is_filled(text: &str) -> bool {
    !text.trim().is_empty()
}
