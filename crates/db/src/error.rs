//! What can go wrong while opening or migrating the database.

use pos_core::AppError;

/// Everything that can go wrong in the db module.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// SQLite refused something. The details go to the log.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// A migration that already ran was edited afterwards. Released migrations must never change.
    #[error("migration {module} {version} was changed after it ran")]
    ChecksumMismatch { module: String, version: u32 },
    /// The database was made by a newer app, which knows a migration this app doesn't.
    #[error("the database has migration {module} {version}, which this app doesn't know")]
    NewerDatabase { module: String, version: u32 },
    /// A module's migration list is wrong, for example versions out of order.
    #[error("migration list for {module} is wrong: {detail}")]
    BadMigrationList { module: String, detail: String },
}

impl From<DbError> for AppError {
    fn from(error: DbError) -> AppError {
        AppError::internal(error.to_string())
    }
}

/// Turns a SQLite error into an app error, keeping the details for the log.
pub(crate) fn sqlite_to_app(error: rusqlite::Error) -> AppError {
    AppError::internal(format!("database error: {error}"))
}
