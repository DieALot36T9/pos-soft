//! The auth module's errors, and how each one reaches the UI as an `AppError` code.

use std::collections::BTreeMap;

use pos_core::AppError;

/// Everything that can go wrong in this module.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("role not found")]
    RoleNotFound { role: String },
    #[error("user not found")]
    UserNotFound { user: String },
    #[error("role name missing")]
    RoleNameMissing,
    #[error("role name in use")]
    RoleNameInUse,
    #[error("maximum discount above 100%")]
    MaxDiscountTooHigh,
    #[error("the Admin role can't be changed")]
    AdminRoleFixed,
    #[error("username missing")]
    UsernameMissing,
    #[error("display name missing")]
    DisplayNameMissing,
    #[error("username in use")]
    UsernameInUse,
    #[error("no PIN and no password")]
    SecretMissing,
    #[error("PIN is not 4 to 8 digits")]
    PinFormat,
    #[error("password shorter than 8 characters")]
    PasswordTooShort,
    #[error("this would leave no active admin")]
    LastActiveAdmin,
    #[error("user is disabled")]
    UserInactive,
    #[error("override user unknown or disabled")]
    OverrideUserUnknown,
    #[error("recovery code wrong")]
    RecoveryCodeWrong,
    #[error("recovery user is not an admin")]
    RecoveryUserNotAdmin,
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("hashing error: {0}")]
    Hashing(String),
}

impl From<AuthError> for AppError {
    fn from(error: AuthError) -> AppError {
        match error {
            AuthError::RoleNotFound { role } => not_found("role", role),
            AuthError::UserNotFound { user } => not_found("user", user),
            AuthError::RoleNameMissing => field_error("name", "role_name_missing"),
            AuthError::RoleNameInUse => conflict("role_name_in_use"),
            AuthError::MaxDiscountTooHigh => field_error("max_discount", "max_discount_too_high"),
            AuthError::AdminRoleFixed => conflict("admin_role_fixed"),
            AuthError::UsernameMissing => field_error("username", "username_missing"),
            AuthError::DisplayNameMissing => field_error("display_name", "display_name_missing"),
            AuthError::UsernameInUse => conflict("username_in_use"),
            AuthError::SecretMissing => AppError::validation("secret_missing"),
            AuthError::PinFormat => field_error("pin", "pin_format"),
            AuthError::PasswordTooShort => field_error("password", "password_too_short"),
            AuthError::LastActiveAdmin => conflict("last_active_admin"),
            AuthError::UserInactive => conflict("user_inactive"),
            AuthError::OverrideUserUnknown => AppError::validation("override_user_unknown"),
            AuthError::RecoveryCodeWrong => field_error("code", "recovery_code_wrong"),
            AuthError::RecoveryUserNotAdmin => AppError::validation("recovery_user_not_admin"),
            AuthError::Database(error) => AppError::internal(format!("auth database: {error}")),
            AuthError::Hashing(detail) => AppError::internal(format!("auth hashing: {detail}")),
        }
    }
}

fn not_found(entity: &str, key: String) -> AppError {
    AppError::NotFound {
        entity: entity.to_string(),
        key,
    }
}

fn conflict(code: &str) -> AppError {
    AppError::Conflict {
        code: code.to_string(),
        params: BTreeMap::new(),
    }
}

fn field_error(field: &str, code: &str) -> AppError {
    AppError::Validation {
        field: Some(field.to_string()),
        code: code.to_string(),
        params: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_details_never_reach_the_ui() {
        let error: AppError = AuthError::Hashing("salt detail".to_string()).into();
        assert!(matches!(error, AppError::Internal { .. }));
        assert!(!serde_json::to_string(&error).unwrap().contains("salt"));
    }

    #[test]
    fn pin_errors_name_the_pin_field() {
        let error: AppError = AuthError::PinFormat.into();
        let json = serde_json::to_string(&error).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"validation","field":"pin","code":"pin_format","params":{}}"#
        );
    }
}
