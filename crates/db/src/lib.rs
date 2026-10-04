//! Opens and sets up SQLite and runs every module's migrations.
//! It knows nothing about any module's tables: it only runs the SQL it's given.
//!
//! Page: `docs/05 Modules/Module - db.md`. Backups, restore and integrity checks
//! come later (task B-db); see "Open items" there.

mod error;
mod migrate;
mod open;

pub use error::DbError;
pub use migrate::{migrate, pending};
pub use open::Db;
// Modules describe their migrations with these core types; re-exported for convenience.
pub use pos_core::{Migration, ModuleMigrations};
