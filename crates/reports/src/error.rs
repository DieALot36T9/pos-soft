//! The one error type of this module. It turns into `AppError` for the UI.

use pos_core::{AppError, Id};

/// Everything that can go wrong while making a report.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ReportsError {
    /// The caller asked for something the rules don't allow. The text is the error code.
    /// Example: "date_range_backwards".
    #[error("not allowed: {0}")]
    Invalid(&'static str),
    /// No shift has this id.
    #[error("shift {0} was not found")]
    ShiftNotFound(Id),
    /// A money sum overflowed, or a stored value could not be read back.
    #[error(transparent)]
    Core(#[from] AppError),
    /// The database refused a query. This is a bug.
    #[error("database: {0}")]
    Database(#[from] rusqlite::Error),
}

impl From<ReportsError> for AppError {
    fn from(error: ReportsError) -> AppError {
        match error {
            ReportsError::Invalid(code) => AppError::validation(code),
            ReportsError::ShiftNotFound(shift) => AppError::NotFound {
                entity: "shift".to_string(),
                key: shift.to_string(),
            },
            ReportsError::Core(error) => error,
            ReportsError::Database(error) => AppError::internal(format!("reports: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_turn_into_the_right_app_error_kinds() {
        assert_eq!(
            AppError::from(ReportsError::Invalid("date_range_backwards")),
            AppError::validation("date_range_backwards")
        );
        let database = ReportsError::Database(rusqlite::Error::QueryReturnedNoRows);
        assert!(matches!(
            AppError::from(database),
            AppError::Internal { .. }
        ));
    }
}
