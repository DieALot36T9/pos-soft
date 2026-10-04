//! Opening the database: one writer connection and one reader connection,
//! each with the same settings (Database Strategy, "Connection settings").

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use pos_core::{AppError, Id};
use rusqlite::{Connection, OpenFlags, Transaction, TransactionBehavior};

use crate::DbError;
use crate::error::sqlite_to_app;

/// Settings applied to every connection, in this order.
const CONNECTION_SETTINGS: &[&str] = &[
    // Readers don't block the writer.
    "PRAGMA journal_mode = WAL",
    // A committed sale survives a power cut.
    "PRAGMA synchronous = FULL",
    // Foreign keys inside one module are enforced.
    "PRAGMA foreign_keys = ON",
    "PRAGMA busy_timeout = 5000",
    "PRAGMA temp_store = MEMORY",
    // 64 MB page cache.
    "PRAGMA cache_size = -65536",
    // 256 MB memory map.
    "PRAGMA mmap_size = 268435456",
];

/// The open database. All changes go through the one writer, so they never clash.
/// Reports and lists use the reader, which sees a consistent snapshot.
pub struct Db {
    pub(crate) writer: Mutex<Connection>,
    reader: Mutex<Connection>,
}

impl Db {
    /// Opens (or creates) the database file at `path`.
    pub fn open(path: &Path) -> Result<Db, DbError> {
        let writer = Connection::open(path)?;
        configure(&writer)?;
        let reader = Connection::open(path)?;
        configure(&reader)?;
        Ok(Db::from_connections(writer, reader))
    }

    /// Opens a new, empty database in memory, for tests. Each call gets its own database.
    pub fn open_in_memory() -> Result<Db, DbError> {
        // Both connections share one in-memory database through its unique name.
        let uri = format!("file:pos-memory-{}?mode=memory&cache=shared", Id::new());
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_URI;
        let writer = Connection::open_with_flags(&uri, flags)?;
        configure(&writer)?;
        let reader = Connection::open_with_flags(&uri, flags)?;
        configure(&reader)?;
        Ok(Db::from_connections(writer, reader))
    }

    fn from_connections(writer: Connection, reader: Connection) -> Db {
        Db {
            writer: Mutex::new(writer),
            reader: Mutex::new(reader),
        }
    }

    /// Runs `work` in one write transaction (BEGIN IMMEDIATE … COMMIT).
    /// If `work` returns an error, nothing it did is saved.
    pub fn write<T>(
        &self,
        work: impl FnOnce(&Transaction) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut connection = lock(&self.writer);
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sqlite_to_app)?;
        let value = work(&transaction)?;
        transaction.commit().map_err(sqlite_to_app)?;
        Ok(value)
    }

    /// Runs `work` on the reader connection. Use it for reports, lists and searches.
    pub fn read<T>(
        &self,
        work: impl FnOnce(&Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let connection = lock(&self.reader);
        work(&connection)
    }
}

/// Takes a connection's lock. If an earlier user panicked, the connection is
/// still fine (its transaction was rolled back), so we carry on with it.
pub(crate) fn lock(connection: &Mutex<Connection>) -> MutexGuard<'_, Connection> {
    connection
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Applies the connection settings. Some pragmas answer with a row, so every
/// answer is read and thrown away.
fn configure(connection: &Connection) -> Result<(), DbError> {
    for setting in CONNECTION_SETTINGS {
        let mut statement = connection.prepare(setting)?;
        let mut answers = statement.query([])?;
        while answers.next()?.is_some() {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setting(connection: &Connection, name: &str) -> String {
        let value: rusqlite::types::Value = connection
            .query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
            .unwrap();
        match value {
            rusqlite::types::Value::Integer(number) => number.to_string(),
            rusqlite::types::Value::Text(text) => text,
            other => format!("{other:?}"),
        }
    }

    fn check_settings(connection: &Connection, journal_mode: &str, is_file: bool) {
        assert_eq!(setting(connection, "journal_mode"), journal_mode);
        assert_eq!(setting(connection, "synchronous"), "2"); // FULL
        assert_eq!(setting(connection, "foreign_keys"), "1");
        assert_eq!(setting(connection, "busy_timeout"), "5000");
        assert_eq!(setting(connection, "temp_store"), "2"); // MEMORY
        assert_eq!(setting(connection, "cache_size"), "-65536");
        // A memory database has no file to map, so mmap_size only applies to files.
        if is_file {
            assert_eq!(setting(connection, "mmap_size"), "268435456");
        }
    }

    #[test]
    fn settings_are_applied_to_both_connections_of_a_file_database() {
        let folder = std::env::temp_dir().join(format!("pos-db-test-{}", Id::new()));
        std::fs::create_dir_all(&folder).unwrap();
        let db = Db::open(&folder.join("pos.sqlite")).unwrap();

        check_settings(&lock(&db.writer), "wal", true);
        check_settings(&lock(&db.reader), "wal", true);

        drop(db);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn settings_are_applied_to_both_connections_of_a_memory_database() {
        let db = Db::open_in_memory().unwrap();
        // A memory database can't use WAL; SQLite keeps its journal in memory.
        check_settings(&lock(&db.writer), "memory", false);
        check_settings(&lock(&db.reader), "memory", false);
    }

    #[test]
    fn each_memory_database_is_separate() {
        let first = Db::open_in_memory().unwrap();
        let second = Db::open_in_memory().unwrap();
        first
            .write(|tx| {
                tx.execute_batch("CREATE TABLE only_here (id TEXT)")
                    .map_err(sqlite_to_app)
            })
            .unwrap();
        let found: i64 = second
            .read(|conn| {
                conn.query_row(
                    "SELECT count(*) FROM sqlite_master WHERE name = 'only_here'",
                    [],
                    |row| row.get(0),
                )
                .map_err(sqlite_to_app)
            })
            .unwrap();
        assert_eq!(found, 0);
    }
}
