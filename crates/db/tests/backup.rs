//! Contract tests for backups, checks and change tracking, as described on `Module - db.md`.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use db::{
    Db, DbError, backup_to, change_marker, has_changed_since, migrate, open_read_only, quick_check,
    verify,
};
use pos_core::{AppError, Id, Migration, ModuleMigrations};

const FRUIT_V1: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: "CREATE TABLE fruit_items (id TEXT PRIMARY KEY, name TEXT NOT NULL);",
}];

const FRUIT_V2: &[Migration] = &[
    FRUIT_V1[0],
    Migration {
        version: 2,
        name: "add_colour",
        sql: "ALTER TABLE fruit_items ADD COLUMN colour TEXT NOT NULL DEFAULT '';",
    },
];

fn fruit(migrations: &'static [Migration]) -> ModuleMigrations {
    ModuleMigrations {
        module: "fruit",
        migrations,
    }
}

/// A fresh, empty folder for one test. Deleted when the guard is dropped.
struct Folder(PathBuf);

impl Folder {
    fn new() -> Folder {
        let path = std::env::temp_dir().join(format!("pos-db-backup-{}", Id::new()));
        std::fs::create_dir_all(&path).unwrap();
        Folder(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn add_fruit(db: &Db, id: &str) {
    db.write(|tx| {
        tx.execute(
            "INSERT INTO fruit_items (id, name) VALUES (:id, 'apple')",
            rusqlite::named_params! { ":id": id },
        )
        .map_err(|error| AppError::internal(error.to_string()))?;
        Ok(())
    })
    .unwrap();
}

fn file_db_with_fruit(folder: &Folder) -> Db {
    let db = Db::open(&folder.file("pos.sqlite")).unwrap();
    migrate(&db, &[fruit(FRUIT_V2)]).unwrap();
    add_fruit(&db, "a");
    add_fruit(&db, "b");
    db
}

fn count_fruit(connection: &rusqlite::Connection) -> i64 {
    connection
        .query_row("SELECT count(*) FROM fruit_items", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn bak_01_backup_file_passes_verify_and_holds_the_data() {
    let folder = Folder::new();
    let db = file_db_with_fruit(&folder);
    let file = folder.file("pos-20261004-101500-manual.sqlite");

    let made = backup_to(&db, &file).unwrap();
    assert_eq!(made.file, file);
    assert!(made.size_bytes > 0);
    assert_eq!(made.migrations, vec![("fruit".to_string(), 2)]);

    let checked = verify(&file, &[fruit(FRUIT_V2)]).unwrap();
    assert_eq!(checked, made);

    let connection = open_read_only(&file).unwrap();
    assert_eq!(count_fruit(&connection), 2);
}

#[test]
fn bak_01_backup_works_from_a_memory_database() {
    let folder = Folder::new();
    let db = Db::open_in_memory().unwrap();
    migrate(&db, &[fruit(FRUIT_V1)]).unwrap();
    add_fruit(&db, "a");
    let file = folder.file("backup.sqlite");

    backup_to(&db, &file).unwrap();

    assert_eq!(count_fruit(&open_read_only(&file).unwrap()), 1);
}

#[test]
fn bak_01_backup_works_while_a_write_transaction_is_open() {
    let folder = Folder::new();
    let db = file_db_with_fruit(&folder);
    let file = folder.file("backup.sqlite");

    // The backup runs while the writer is in the middle of adding a third row.
    let made = db
        .write(|tx| {
            tx.execute(
                "INSERT INTO fruit_items (id, name) VALUES ('c', 'pear')",
                [],
            )
            .map_err(|error| AppError::internal(error.to_string()))?;
            backup_to(&db, &file).map_err(AppError::from)
        })
        .unwrap();

    // The backup holds what was committed before it started, not the unfinished row.
    assert_eq!(made.migrations, vec![("fruit".to_string(), 2)]);
    assert_eq!(count_fruit(&open_read_only(&file).unwrap()), 2);
}

#[test]
fn bak_01_backup_never_overwrites_an_existing_file() {
    let folder = Folder::new();
    let db = file_db_with_fruit(&folder);
    let file = folder.file("backup.sqlite");
    std::fs::write(&file, b"keep me").unwrap();

    assert!(backup_to(&db, &file).is_err());
    assert_eq!(std::fs::read(&file).unwrap(), b"keep me");
}

#[test]
fn bak_03_verify_refuses_a_file_that_is_not_a_database() {
    let folder = Folder::new();
    let file = folder.file("broken.sqlite");
    std::fs::write(&file, vec![7_u8; 8192]).unwrap();

    let result = verify(&file, &[fruit(FRUIT_V2)]);
    assert!(
        matches!(result, Err(DbError::CheckFailed { .. })),
        "{result:?}"
    );
}

#[test]
fn bak_03_verify_refuses_a_damaged_database() {
    let folder = Folder::new();
    let db = file_db_with_fruit(&folder);
    let file = folder.file("backup.sqlite");
    backup_to(&db, &file).unwrap();

    // Scribble over the second page, where the tables' data lives.
    let mut bytes = std::fs::read(&file).unwrap();
    let page_size = usize::from(u16::from_be_bytes([bytes[16], bytes[17]]));
    for byte in &mut bytes[page_size..page_size * 2] {
        *byte = 0xAB;
    }
    std::fs::write(&file, &bytes).unwrap();

    let result = verify(&file, &[fruit(FRUIT_V2)]);
    assert!(
        matches!(result, Err(DbError::CheckFailed { .. })),
        "{result:?}"
    );
}

#[test]
fn bak_03_verify_refuses_a_missing_file() {
    let folder = Folder::new();
    let result = verify(&folder.file("not-there.sqlite"), &[fruit(FRUIT_V2)]);
    assert!(result.is_err());
    // Checking must not create the file.
    assert!(!folder.file("not-there.sqlite").exists());
}

#[test]
fn bak_05_verify_refuses_a_backup_from_a_newer_app() {
    let folder = Folder::new();
    let db = file_db_with_fruit(&folder);
    let file = folder.file("backup.sqlite");
    backup_to(&db, &file).unwrap();

    // This app only knows fruit version 1; the backup has version 2.
    let result = verify(&file, &[fruit(FRUIT_V1)]);
    assert!(
        matches!(result, Err(DbError::NewerDatabase { ref module, version: 2 }) if module == "fruit"),
        "{result:?}"
    );
    // A whole module this app doesn't know is refused too.
    assert!(matches!(
        verify(&file, &[]),
        Err(DbError::NewerDatabase { .. })
    ));
}

#[test]
fn bak_05_verify_accepts_a_backup_from_an_older_app() {
    let folder = Folder::new();
    let db = Db::open(&folder.file("pos.sqlite")).unwrap();
    migrate(&db, &[fruit(FRUIT_V1)]).unwrap();
    let file = folder.file("backup.sqlite");
    backup_to(&db, &file).unwrap();

    let checked = verify(&file, &[fruit(FRUIT_V2)]).unwrap();
    assert_eq!(checked.migrations, vec![("fruit".to_string(), 1)]);
}

#[test]
fn bak_06_quick_check_passes_on_a_healthy_database() {
    let folder = Folder::new();
    let db = file_db_with_fruit(&folder);
    assert!(quick_check(&db).unwrap());
    assert!(quick_check(&Db::open_in_memory().unwrap()).unwrap());
}

#[test]
fn bak_01_change_marker_moves_only_when_a_write_changes_rows() {
    let db = Db::open_in_memory().unwrap();
    migrate(&db, &[fruit(FRUIT_V1)]).unwrap();
    let marker = change_marker(&db);
    assert!(!has_changed_since(&db, marker));

    // A write that only reads changes nothing.
    db.write(|tx| {
        tx.query_row("SELECT count(*) FROM fruit_items", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|error| AppError::internal(error.to_string()))
    })
    .unwrap();
    assert!(!has_changed_since(&db, marker));

    add_fruit(&db, "a");
    assert!(has_changed_since(&db, marker));
    let after_add = change_marker(&db);
    assert!(!has_changed_since(&db, after_add));
}

#[test]
fn bak_01_rolled_back_write_is_not_a_change() {
    let db = Db::open_in_memory().unwrap();
    migrate(&db, &[fruit(FRUIT_V1)]).unwrap();
    let marker = change_marker(&db);

    let result: Result<(), AppError> = db.write(|tx| {
        tx.execute("INSERT INTO fruit_items (id, name) VALUES ('a', 'x')", [])
            .map_err(|error| AppError::internal(error.to_string()))?;
        Err(AppError::validation("stop"))
    });
    assert!(result.is_err());
    assert!(!has_changed_since(&db, marker));
}

#[test]
fn bak_01_running_migrations_is_a_change() {
    let db = Db::open_in_memory().unwrap();
    let marker = change_marker(&db);
    migrate(&db, &[fruit(FRUIT_V1)]).unwrap();
    assert!(has_changed_since(&db, marker));

    // Nothing left to run: no change.
    let marker = change_marker(&db);
    migrate(&db, &[fruit(FRUIT_V1)]).unwrap();
    assert!(!has_changed_since(&db, marker));
}
