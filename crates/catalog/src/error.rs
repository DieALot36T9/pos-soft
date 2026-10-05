//! The catalog's one error type. It turns into `AppError` for the UI.

use std::collections::BTreeMap;

use pos_core::AppError;

/// Everything that can go wrong in the catalog. The `code` values are stable:
/// the UI turns each one into a plain-English message.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum CatalogError {
    /// The user can fix what they typed. Example: field "name", code "name_missing".
    #[error("invalid {field}: {code}")]
    Invalid {
        field: &'static str,
        code: &'static str,
    },
    /// Rule CAT-04: the barcode already belongs to another product.
    /// The UI shows "Already used by Milk 1L (SKU P000123)".
    #[error("barcode {code} is already used by {product_name} ({sku})")]
    BarcodeInUse {
        code: String,
        product_name: String,
        sku: String,
    },
    /// The action clashes with existing data. Example: code "sku_in_use", value "P000123".
    #[error("conflict {code}: {value}")]
    Conflict { code: &'static str, value: String },
    /// A unit, category, brand or product with this id doesn't exist (or is archived
    /// where an active one is needed).
    #[error("{entity} {key} was not found")]
    NotFound { entity: &'static str, key: String },
    /// A database error. This is a bug, so the user sees only a reference number.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    /// Reading or writing an image file failed.
    #[error("file error: {0}")]
    File(String),
}

impl From<CatalogError> for AppError {
    fn from(error: CatalogError) -> AppError {
        match error {
            CatalogError::Invalid { field, code } => AppError::Validation {
                field: Some(field.to_string()),
                code: code.to_string(),
                params: BTreeMap::new(),
            },
            CatalogError::BarcodeInUse {
                code,
                product_name,
                sku,
            } => AppError::Conflict {
                code: "barcode_in_use".to_string(),
                params: BTreeMap::from([
                    ("code".to_string(), code),
                    ("product_name".to_string(), product_name),
                    ("sku".to_string(), sku),
                ]),
            },
            CatalogError::Conflict { code, value } => AppError::Conflict {
                code: code.to_string(),
                params: BTreeMap::from([("value".to_string(), value)]),
            },
            CatalogError::NotFound { entity, key } => AppError::NotFound {
                entity: entity.to_string(),
                key,
            },
            CatalogError::Database(error) => AppError::internal(format!("catalog: {error}")),
            CatalogError::File(detail) => AppError::internal(format!("catalog file: {detail}")),
        }
    }
}

/// A shortcut for "the user typed something wrong in this field".
pub(crate) fn invalid(field: &'static str, code: &'static str) -> CatalogError {
    CatalogError::Invalid { field, code }
}

/// A shortcut for "this clashes with existing data".
pub(crate) fn conflict(code: &'static str, value: impl Into<String>) -> CatalogError {
    CatalogError::Conflict {
        code,
        value: value.into(),
    }
}

/// A shortcut for "there is no such row".
pub(crate) fn not_found(entity: &'static str, key: impl Into<String>) -> CatalogError {
    CatalogError::NotFound {
        entity,
        key: key.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_errors_hide_their_details() {
        let error = CatalogError::Database(rusqlite::Error::QueryReturnedNoRows);
        assert!(matches!(AppError::from(error), AppError::Internal { .. }));
    }

    #[test]
    fn conflicts_pass_their_value_to_the_ui() {
        let AppError::Conflict { code, params } = AppError::from(conflict("sku_in_use", "P1"))
        else {
            panic!("expected a conflict");
        };
        assert_eq!(code, "sku_in_use");
        assert_eq!(params["value"], "P1");
    }
}
