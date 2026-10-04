//! The one error type that crosses into the UI (see Errors and Logging).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Id, Permission};

/// Every error the UI can receive. Rust sends codes, never sentences; the UI turns
/// each `code` into plain English using `ui/src/shared/text/en/errors.ts`.
/// Each module has its own error enum that turns into this one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum AppError {
    /// The user can fix what they typed. `field` names the form field, if there is one.
    #[error("validation failed: {code}")]
    Validation {
        field: Option<String>,
        code: String,
        params: BTreeMap<String, String>,
    },
    /// Something the user asked for doesn't exist.
    #[error("{entity} {key} was not found")]
    NotFound { entity: String, key: String },
    /// The action clashes with existing data. Example: barcode_in_use.
    #[error("conflict: {code}")]
    Conflict {
        code: String,
        params: BTreeMap<String, String>,
    },
    /// The user's role doesn't allow this action.
    #[error("permission denied: {permission:?}")]
    PermissionDenied { permission: Permission },
    /// An admin can approve this one action. The UI shows the approval dialog.
    #[error("admin approval needed: {permission:?}")]
    OverrideRequired { permission: Permission },
    /// A device failed. Example: the printer is offline.
    #[error("{device} failed: {code} ({detail})")]
    Hardware {
        device: String,
        code: String,
        detail: String,
    },
    /// A bug. The user sees only the reference `id`; `detail` goes to the log only.
    #[error("internal error {id}: {detail}")]
    Internal {
        id: String,
        #[serde(skip)]
        #[ts(skip)]
        detail: String,
    },
}

impl AppError {
    /// A mistake in what the user typed, with no extra details. Example: `validation("name_missing")`.
    pub fn validation(code: &str) -> AppError {
        AppError::Validation {
            field: None,
            code: code.to_string(),
            params: BTreeMap::new(),
        }
    }

    /// A bug. Gets a fresh reference id so the log entry can be found later.
    pub fn internal(detail: impl Into<String>) -> AppError {
        AppError::Internal {
            id: Id::new().as_str().to_string(),
            detail: detail.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_details_never_reach_the_ui() {
        let error = AppError::internal("secret table name");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.starts_with(r#"{"kind":"internal","id":""#));
        assert!(!json.contains("secret"));
    }

    #[test]
    fn validation_is_sent_as_kind_code_params_and_field() {
        let error = AppError::Validation {
            field: Some("price".to_string()),
            code: "money_empty".to_string(),
            params: BTreeMap::new(),
        };
        assert_eq!(
            serde_json::to_string(&error).unwrap(),
            r#"{"kind":"validation","field":"price","code":"money_empty","params":{}}"#
        );
    }

    #[test]
    fn permission_errors_name_the_permission_in_snake_case() {
        let error = AppError::OverrideRequired {
            permission: Permission::SaleVoid,
        };
        assert_eq!(
            serde_json::to_string(&error).unwrap(),
            r#"{"kind":"override_required","permission":"sale_void"}"#
        );
    }
}
