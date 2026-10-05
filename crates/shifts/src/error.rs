//! The one error type of this module. It turns into `AppError` for the UI.

use std::collections::BTreeMap;

use pos_core::{AppError, Id};

/// Everything that can go wrong in the shifts module.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ShiftsError {
    /// The caller gave something the rules don't allow. The text is the error code.
    /// Example: "reason_missing".
    #[error("not allowed: {0}")]
    Invalid(&'static str),
    /// Rule SHF-02: this till already has an open shift.
    #[error("till {till} already has an open shift")]
    AlreadyOpen { till: Id },
    /// The shift is closed, so it can't change any more.
    #[error("shift {0} is closed")]
    Closed(Id),
    /// No shift has this id.
    #[error("shift {0} was not found")]
    NotFound(Id),
    /// `reverse_ref` found no movements for this document in the shift.
    #[error("nothing to reverse for {ref_type} {ref_id}")]
    NothingToReverse { ref_type: String, ref_id: Id },
    /// `reverse_ref` already ran for this document.
    #[error("{ref_type} {ref_id} was already reversed")]
    AlreadyReversed { ref_type: String, ref_id: Id },
    /// A money sum overflowed, or a stored value could not be read back.
    #[error(transparent)]
    Core(#[from] AppError),
    /// The database refused a query. This is a bug.
    #[error("database: {0}")]
    Database(#[from] rusqlite::Error),
}

impl From<ShiftsError> for AppError {
    fn from(error: ShiftsError) -> AppError {
        match error {
            ShiftsError::Invalid(code) => AppError::validation(code),
            ShiftsError::AlreadyOpen { till } => conflict("shift_already_open", "till", &till),
            ShiftsError::Closed(shift) => conflict("shift_closed", "shift", &shift),
            ShiftsError::NotFound(shift) => AppError::NotFound {
                entity: "shift".to_string(),
                key: shift.to_string(),
            },
            ShiftsError::NothingToReverse { ref_type, ref_id } => AppError::NotFound {
                entity: "shift_movement".to_string(),
                key: format!("{ref_type} {ref_id}"),
            },
            ShiftsError::AlreadyReversed { ref_id, .. } => {
                conflict("already_reversed", "ref_id", &ref_id)
            }
            ShiftsError::Core(error) => error,
            ShiftsError::Database(error) => AppError::internal(format!("shifts: {error}")),
        }
    }
}

/// A conflict with one named id, so the UI can say which till or shift it was.
fn conflict(code: &str, param: &str, id: &Id) -> AppError {
    let mut params = BTreeMap::new();
    params.insert(param.to_string(), id.to_string());
    AppError::Conflict {
        code: code.to_string(),
        params,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_turn_into_the_right_app_error_kinds() {
        assert_eq!(
            AppError::from(ShiftsError::Invalid("reason_missing")),
            AppError::validation("reason_missing")
        );
        let shift = Id::new();
        assert!(matches!(
            AppError::from(ShiftsError::Closed(shift)),
            AppError::Conflict { code, .. } if code == "shift_closed"
        ));
        let database = ShiftsError::Database(rusqlite::Error::QueryReturnedNoRows);
        assert!(matches!(
            AppError::from(database),
            AppError::Internal { .. }
        ));
    }
}
