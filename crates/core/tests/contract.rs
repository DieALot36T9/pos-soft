//! Contract tests: the public API behaves as the module page says.
//! The detailed rules are tested in `examples.rs`, `money_vectors.rs` and
//! `properties.rs`; this file checks the shapes other modules rely on.

use pos_core::{AppError, Money, Permission};

#[test]
fn every_permission_from_the_roles_page_exists() {
    assert_eq!(Permission::ALL.len(), 42);
}

#[test]
fn module_errors_can_become_app_errors() {
    let error: AppError = pos_core::parse_money("abc", 2).unwrap_err().into();
    assert!(matches!(error, AppError::Validation { .. }));
}

#[test]
fn money_starts_at_zero() {
    assert_eq!(Money::default(), Money::zero());
}
