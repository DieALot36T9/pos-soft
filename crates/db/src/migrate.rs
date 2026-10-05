//! The migration runner (Database Strategy, "Migrations").
//! Each module's missing migrations run in one transaction per module.

use std::collections::BTreeMap;

use pos_core::{Clock, ModuleMigrations, SystemClock};
use rusqlite::{Connection, TransactionBehavior, named_params};
use sha2::{Digest, Sha256};

use crate::open::lock;
use crate::{Db, DbError};

const CREATE_MIGRATIONS_TABLE: &str = "
CREATE TABLE IF NOT EXISTS db_migrations (
  module      TEXT NOT NULL,
  version     INTEGER NOT NULL,
  name        TEXT NOT NULL,
  checksum    TEXT NOT NULL,
  applied_at  INTEGER NOT NULL,
  PRIMARY KEY (module, version)
);";

/// The migrations already in the database: module → (version → checksum).
pub(crate) type Applied = BTreeMap<String, BTreeMap<u32, String>>;

/// Lists the migrations that `migrate` would run, as (module, version), in order.
pub fn pending(db: &Db, all: &[ModuleMigrations]) -> Result<Vec<(String, u32)>, DbError> {
    check_lists(all)?;
    let connection = lock(&db.writer);
    connection.execute_batch(CREATE_MIGRATIONS_TABLE)?;
    let applied = read_applied(&connection)?;
    check_applied(&applied, all)?;

    let mut waiting = Vec::new();
    for module in all {
        for migration in module.migrations {
            if !is_applied(&applied, module.module, migration.version) {
                waiting.push((module.module.to_string(), migration.version));
            }
        }
    }
    Ok(waiting)
}

/// Brings the database up to date: runs every missing migration, module by module,
/// in the order given. Refuses to run if an applied migration was edited, or if the
/// database was made by a newer app. The app takes a backup before calling this.
pub fn migrate(db: &Db, all: &[ModuleMigrations]) -> Result<(), DbError> {
    check_lists(all)?;
    let mut connection = lock(&db.writer);
    connection.execute_batch(CREATE_MIGRATIONS_TABLE)?;
    let applied = read_applied(&connection)?;
    // Check everything before changing anything.
    check_applied(&applied, all)?;

    for module in all {
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut ran_any = false;
        for migration in module.migrations {
            if is_applied(&applied, module.module, migration.version) {
                continue;
            }
            ran_any = true;
            transaction.execute_batch(migration.sql)?;
            transaction.execute(
                "INSERT INTO db_migrations (module, version, name, checksum, applied_at)
                 VALUES (:module, :version, :name, :checksum, :applied_at)",
                named_params! {
                    ":module": module.module,
                    ":version": migration.version,
                    ":name": migration.name,
                    ":checksum": checksum(migration.sql),
                    ":applied_at": SystemClock.now().0,
                },
            )?;
        }
        transaction.commit()?;
        if ran_any {
            db.note_change();
        }
    }
    Ok(())
}

/// Checks each module's list: versions start at 1 or more and go strictly up,
/// and no module appears twice.
fn check_lists(all: &[ModuleMigrations]) -> Result<(), DbError> {
    let mut seen_modules = Vec::new();
    for module in all {
        if seen_modules.contains(&module.module) {
            return Err(bad_list(module.module, "the module is listed twice"));
        }
        seen_modules.push(module.module);
        let mut previous_version = 0;
        for migration in module.migrations {
            if migration.version <= previous_version {
                return Err(bad_list(
                    module.module,
                    &format!(
                        "version {} comes after {previous_version}",
                        migration.version
                    ),
                ));
            }
            previous_version = migration.version;
        }
    }
    Ok(())
}

/// Checks that every migration in the database is one this app knows, unchanged.
pub(crate) fn check_applied(applied: &Applied, all: &[ModuleMigrations]) -> Result<(), DbError> {
    for (module_name, versions) in applied {
        let known = all.iter().find(|module| module.module == module_name);
        for (version, stored_checksum) in versions {
            let migration =
                known.and_then(|module| module.migrations.iter().find(|m| m.version == *version));
            let Some(migration) = migration else {
                return Err(DbError::NewerDatabase {
                    module: module_name.clone(),
                    version: *version,
                });
            };
            if checksum(migration.sql) != *stored_checksum {
                return Err(DbError::ChecksumMismatch {
                    module: module_name.clone(),
                    version: *version,
                });
            }
        }
    }
    Ok(())
}

pub(crate) fn read_applied(connection: &Connection) -> Result<Applied, DbError> {
    let mut statement =
        connection.prepare("SELECT module, version, checksum FROM db_migrations")?;
    let mut rows = statement.query([])?;
    let mut applied = Applied::new();
    while let Some(row) = rows.next()? {
        let module: String = row.get(0)?;
        let version: u32 = row.get(1)?;
        let stored_checksum: String = row.get(2)?;
        applied
            .entry(module)
            .or_default()
            .insert(version, stored_checksum);
    }
    Ok(applied)
}

fn is_applied(applied: &Applied, module: &str, version: u32) -> bool {
    applied
        .get(module)
        .is_some_and(|versions| versions.contains_key(&version))
}

/// The SHA-256 of a migration's SQL, as lower-case hex.
fn checksum(sql: &str) -> String {
    let digest = Sha256::digest(sql.as_bytes());
    let mut text = String::with_capacity(64);
    for byte in digest.iter() {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn bad_list(module: &str, detail: &str) -> DbError {
    DbError::BadMigrationList {
        module: module.to_string(),
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_is_sha256_hex() {
        assert_eq!(
            checksum(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
