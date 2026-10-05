//! Backups and integrity checks (Database Strategy, "Backups" and "Integrity").
//! A backup is always made with `VACUUM INTO`, never by copying the live file.

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use pos_core::ModuleMigrations;
use rusqlite::{Connection, ErrorCode, OpenFlags, named_params};

use crate::migrate::{check_applied, read_applied};
use crate::open::lock;
use crate::{Db, DbError};

/// What a checked backup file holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupInfo {
    /// Where the backup file is.
    pub file: PathBuf,
    /// The file's size in bytes.
    pub size_bytes: u64,
    /// The newest migration version of each module in the file, as (module, version),
    /// sorted by module name.
    pub migrations: Vec<(String, u32)>,
}

/// Writes a consistent copy of the database to `file` and checks it (BAK-01, BAK-03).
/// Sales can carry on while it runs. Refuses to overwrite a file that already exists.
pub fn backup_to(db: &Db, file: &Path) -> Result<BackupInfo, DbError> {
    let file_text = path_text(file)?;
    {
        // The reader sees the last committed data, so an open write is left out.
        let reader = lock(&db.reader);
        reader.execute("VACUUM INTO :file", named_params! { ":file": file_text })?;
    }
    // Push the new file to the disk, so a power cut right after can't lose it.
    OpenOptions::new().write(true).open(file)?.sync_all()?;
    check_file(file)
}

/// Checks a backup file before it's restored or listed (BAK-03, BAK-05): it must pass
/// `quick_check` and hold no migration this app doesn't know (or one that was edited).
pub fn verify(file: &Path, all: &[ModuleMigrations]) -> Result<BackupInfo, DbError> {
    let info = check_file(file)?;
    let connection = open_read_only(file)?;
    if has_migrations_table(&connection)? {
        check_applied(&read_applied(&connection)?, all)?;
    }
    Ok(info)
}

/// Opens a database or backup file for reading only. `app` uses it to ask other
/// modules for a backup's details, such as the number of sales (BAK-05).
pub fn open_read_only(file: &Path) -> Result<Connection, DbError> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    Ok(Connection::open_with_flags(file, flags)?)
}

/// Runs `PRAGMA quick_check` on the live database (BAK-06). `true` means it's healthy.
pub fn quick_check(db: &Db) -> Result<bool, DbError> {
    let reader = lock(&db.reader);
    Ok(quick_check_problems(&reader)?.is_empty())
}

/// A number that goes up whenever the data changes. The backup timer keeps the
/// number from its last backup and passes it to `has_changed_since`.
pub fn change_marker(db: &Db) -> u64 {
    db.changes.load(Ordering::SeqCst)
}

/// True if any data changed since `change_marker` gave `marker` (BAK-01: the timer
/// skips the backup when nothing changed).
pub fn has_changed_since(db: &Db, marker: u64) -> bool {
    change_marker(db) != marker
}

/// Opens the file read-only, runs `quick_check` and reads its migration versions.
fn check_file(file: &Path) -> Result<BackupInfo, DbError> {
    let size_bytes = std::fs::metadata(file)?.len();
    let failed = |detail: String| DbError::CheckFailed {
        file: file.display().to_string(),
        detail,
    };
    let connection = open_read_only(file).map_err(|error| failed(error.to_string()))?;
    let problems = quick_check_problems(&connection).map_err(|error| failed(error.to_string()))?;
    if !problems.is_empty() {
        return Err(failed(problems.join("; ")));
    }
    Ok(BackupInfo {
        file: file.to_path_buf(),
        size_bytes,
        migrations: newest_versions(&connection)?,
    })
}

/// The problems `PRAGMA quick_check` finds. Empty means the database is healthy.
/// A file too damaged to read at all counts as a problem, not as a failed query.
fn quick_check_problems(connection: &Connection) -> Result<Vec<String>, rusqlite::Error> {
    match read_quick_check(connection) {
        Ok(lines) => Ok(lines.into_iter().filter(|line| line != "ok").collect()),
        Err(error) if is_damage(&error) => Ok(vec![error.to_string()]),
        Err(error) => Err(error),
    }
}

fn read_quick_check(connection: &Connection) -> Result<Vec<String>, rusqlite::Error> {
    let mut statement = connection.prepare("PRAGMA quick_check")?;
    let mut rows = statement.query([])?;
    let mut lines = Vec::new();
    while let Some(row) = rows.next()? {
        lines.push(row.get(0)?);
    }
    Ok(lines)
}

fn is_damage(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase)
    )
}

fn has_migrations_table(connection: &Connection) -> Result<bool, DbError> {
    let count: i64 = connection.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'db_migrations'",
        [],
        |row| row.get(0),
    )?;
    Ok(count == 1)
}

fn newest_versions(connection: &Connection) -> Result<Vec<(String, u32)>, DbError> {
    if !has_migrations_table(connection)? {
        return Ok(Vec::new());
    }
    let mut statement = connection.prepare(
        "SELECT module, max(version) FROM db_migrations GROUP BY module ORDER BY module",
    )?;
    let mut rows = statement.query([])?;
    let mut versions = Vec::new();
    while let Some(row) = rows.next()? {
        versions.push((row.get(0)?, row.get(1)?));
    }
    Ok(versions)
}

/// SQLite takes the backup file name as text.
fn path_text(file: &Path) -> Result<&str, DbError> {
    file.to_str().ok_or_else(|| {
        DbError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("backup path is not valid text: {}", file.display()),
        ))
    })
}
