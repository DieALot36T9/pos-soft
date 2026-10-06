//! Users, roles, permissions, passwords and PINs, admin approvals and recovery.
//!
//! Page: `docs/05 Modules/Module - auth.md`.

mod error;
mod hashing;
mod login;
mod recovery;
mod roles;
mod rules;
mod users;

use pos_core::{Migration, ModuleMigrations};

pub use error::AuthError;
pub use login::{
    LoginOutcome, OverrideOutcome, SessionUser, session_user, verify_login, verify_override,
};
pub use recovery::{create_recovery_code, reset_with_recovery};
pub use roles::{
    ADMIN_ROLE_ID, CASHIER_ROLE_ID, NewRole, PermissionSet, RoleSummary, create_role, list_roles,
    permissions_of, set_role_permissions,
};
pub use users::{
    NewUser, SecretKind, UserSummary, UserTile, UserUpdate, create_user, list_users, login_tiles,
    set_secret, update_user,
};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "auth",
        migrations: MIGRATIONS,
    }
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/001_init.sql"),
}];
