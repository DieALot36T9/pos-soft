//! What can go wrong in the inventory module, and how each problem reaches the UI.

use std::collections::BTreeMap;

use pos_core::{AppError, Qty};

/// Everything that can go wrong in the inventory module.
#[derive(Debug, thiserror::Error)]
pub enum InventoryError {
    /// SQLite refused something. The details go to the log.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Something the user typed is wrong. `code` says what, e.g. `qty_zero`.
    #[error("validation failed: {0}")]
    Invalid(&'static str),
    /// A row asked for doesn't exist. Example: entity "batch".
    #[error("{entity} {key} was not found")]
    NotFound { entity: &'static str, key: String },
    /// Taking this much stock out would leave less than zero, and the shop doesn't allow that.
    #[error("not enough stock: {available:?} available")]
    NotEnoughStock { available: Qty },
    /// Like `NotEnoughStock`, but for the one batch that was chosen.
    #[error("not enough stock in batch: {available:?} available")]
    NotEnoughStockInBatch { available: Qty },
    /// The action clashes with what is already saved. Example: `already_reversed`.
    #[error("conflict: {0}")]
    Conflict(&'static str),
    /// A number grew too big to store, or the saved data can't be read back. A bug.
    #[error("bad data: {0}")]
    BadData(String),
}

impl From<InventoryError> for AppError {
    fn from(error: InventoryError) -> AppError {
        match error {
            InventoryError::Sqlite(sqlite) => {
                AppError::internal(format!("database error: {sqlite}"))
            }
            InventoryError::Invalid(code) => AppError::validation(code),
            InventoryError::NotFound { entity, key } => AppError::NotFound {
                entity: entity.to_string(),
                key,
            },
            InventoryError::NotEnoughStock { available } => {
                conflict_with_available("not_enough_stock", available)
            }
            InventoryError::NotEnoughStockInBatch { available } => {
                conflict_with_available("not_enough_stock_in_batch", available)
            }
            InventoryError::Conflict(code) => AppError::Conflict {
                code: code.to_string(),
                params: BTreeMap::new(),
            },
            InventoryError::BadData(detail) => AppError::internal(detail),
        }
    }
}

/// A conflict that tells the UI how much is available, in thousandths ("3000" is 3 pieces).
fn conflict_with_available(code: &str, available: Qty) -> AppError {
    let mut params = BTreeMap::new();
    params.insert("available_milli".to_string(), available.0.to_string());
    AppError::Conflict {
        code: code.to_string(),
        params,
    }
}
