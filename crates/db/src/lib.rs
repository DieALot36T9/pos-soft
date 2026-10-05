//! Opens and sets up SQLite, runs every module's migrations, backs up and checks
//! the database. It knows nothing about any module's tables: it only runs the SQL
//! it's given.
//!
//! Page: `docs/05 Modules/Module - db.md`.

mod backup;
mod error;
mod migrate;
mod open;
mod prune;

pub use backup::{
    BackupInfo, backup_to, change_marker, has_changed_since, open_read_only, quick_check, verify,
};
pub use error::DbError;
pub use migrate::{migrate, pending};
pub use open::Db;
pub use prune::{backup_file_name, prune_backups};
// Modules describe their migrations with these core types; re-exported for convenience.
pub use pos_core::{Migration, ModuleMigrations};
