//! Contract tests for the migration runner, as described on `Module - db.md`.

// Test helpers may stop at the first failure, just like the tests themselves.
#![allow(clippy::unwrap_used)]

use db::{Db, DbError, migrate, pending};
use pos_core::{Migration, ModuleMigrations};

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

const VEG_V1: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: "CREATE TABLE veg_items (id TEXT PRIMARY KEY);",
}];

fn fruit(migrations: &'static [Migration]) -> ModuleMigrations {
    ModuleMigrations {
        module: "fruit",
        migrations,
    }
}

fn veg() -> ModuleMigrations {
    ModuleMigrations {
        module: "veg",
        migrations: VEG_V1,
    }
}

fn table_exists(db: &Db, table: &str) -> bool {
    db.read(|conn| {
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = :name",
                rusqlite::named_params! { ":name": table },
                |row| row.get(0),
            )
            .map_err(|error| pos_core::AppError::internal(error.to_string()))?;
        Ok(count == 1)
    })
    .unwrap()
}

fn applied_rows(db: &Db) -> Vec<(String, u32, String)> {
    db.read(|conn| {
        let mut statement = conn
            .prepare("SELECT module, version, name FROM db_migrations ORDER BY module, version")
            .map_err(|error| pos_core::AppError::internal(error.to_string()))?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(|error| pos_core::AppError::internal(error.to_string()))?;
        Ok(rows.map(|row| row.unwrap()).collect())
    })
    .unwrap()
}

#[test]
fn migrations_run_once_in_order_for_every_module() {
    let db = Db::open_in_memory().unwrap();
    migrate(&db, &[fruit(FRUIT_V2), veg()]).unwrap();

    assert!(table_exists(&db, "fruit_items"));
    assert!(table_exists(&db, "veg_items"));
    assert_eq!(
        applied_rows(&db),
        vec![
            ("fruit".to_string(), 1, "init".to_string()),
            ("fruit".to_string(), 2, "add_colour".to_string()),
            ("veg".to_string(), 1, "init".to_string()),
        ]
    );
}

#[test]
fn running_migrate_again_changes_nothing() {
    let db = Db::open_in_memory().unwrap();
    migrate(&db, &[fruit(FRUIT_V2)]).unwrap();
    migrate(&db, &[fruit(FRUIT_V2)]).unwrap();
    assert_eq!(applied_rows(&db).len(), 2);
}

#[test]
fn pending_lists_what_a_newer_app_would_add() {
    let db = Db::open_in_memory().unwrap();
    migrate(&db, &[fruit(FRUIT_V1)]).unwrap();

    let waiting = pending(&db, &[fruit(FRUIT_V2), veg()]).unwrap();
    assert_eq!(
        waiting,
        vec![("fruit".to_string(), 2), ("veg".to_string(), 1)]
    );

    migrate(&db, &[fruit(FRUIT_V2), veg()]).unwrap();
    assert!(pending(&db, &[fruit(FRUIT_V2), veg()]).unwrap().is_empty());
}

#[test]
fn an_edited_migration_is_refused() {
    let db = Db::open_in_memory().unwrap();
    migrate(&db, &[veg()]).unwrap();

    const EDITED: &[Migration] = &[Migration {
        version: 1,
        name: "init",
        sql: "CREATE TABLE veg_items (id TEXT PRIMARY KEY, extra TEXT);",
    }];
    let edited = ModuleMigrations {
        module: "veg",
        migrations: EDITED,
    };
    let error = migrate(&db, &[edited]).unwrap_err();
    assert!(matches!(
        error,
        DbError::ChecksumMismatch { version: 1, .. }
    ));
}

#[test]
fn a_database_from_a_newer_app_is_refused() {
    let db = Db::open_in_memory().unwrap();
    migrate(&db, &[fruit(FRUIT_V2)]).unwrap();

    let error = migrate(&db, &[fruit(FRUIT_V1)]).unwrap_err();
    assert!(matches!(error, DbError::NewerDatabase { version: 2, .. }));
}

#[test]
fn a_module_this_app_does_not_know_is_refused() {
    let db = Db::open_in_memory().unwrap();
    migrate(&db, &[fruit(FRUIT_V1), veg()]).unwrap();

    let error = migrate(&db, &[fruit(FRUIT_V1)]).unwrap_err();
    assert!(matches!(error, DbError::NewerDatabase { .. }));
}

#[test]
fn a_failing_migration_leaves_its_module_untouched() {
    const BROKEN: &[Migration] = &[
        Migration {
            version: 1,
            name: "init",
            sql: "CREATE TABLE broken_items (id TEXT PRIMARY KEY);",
        },
        Migration {
            version: 2,
            name: "typo",
            sql: "CREATE TABLEX nope;",
        },
    ];
    let broken = ModuleMigrations {
        module: "broken",
        migrations: BROKEN,
    };
    let db = Db::open_in_memory().unwrap();

    assert!(migrate(&db, &[broken]).is_err());
    assert!(!table_exists(&db, "broken_items"));
    assert!(applied_rows(&db).is_empty());
}

#[test]
fn versions_out_of_order_are_refused() {
    const BACKWARDS: &[Migration] = &[
        Migration {
            version: 2,
            name: "second",
            sql: "",
        },
        Migration {
            version: 1,
            name: "first",
            sql: "",
        },
    ];
    let backwards = ModuleMigrations {
        module: "backwards",
        migrations: BACKWARDS,
    };
    let db = Db::open_in_memory().unwrap();
    let error = migrate(&db, &[backwards]).unwrap_err();
    assert!(matches!(error, DbError::BadMigrationList { .. }));
}

#[test]
fn write_rolls_back_when_the_work_fails() {
    let db = Db::open_in_memory().unwrap();
    migrate(&db, &[veg()]).unwrap();

    let result: Result<(), _> = db.write(|tx| {
        tx.execute("INSERT INTO veg_items (id) VALUES ('a')", [])
            .map_err(|error| pos_core::AppError::internal(error.to_string()))?;
        Err(pos_core::AppError::validation("stop_here"))
    });
    assert!(result.is_err());

    let count: i64 = db
        .read(|conn| {
            conn.query_row("SELECT count(*) FROM veg_items", [], |row| row.get(0))
                .map_err(|error| pos_core::AppError::internal(error.to_string()))
        })
        .unwrap();
    assert_eq!(count, 0);
}
