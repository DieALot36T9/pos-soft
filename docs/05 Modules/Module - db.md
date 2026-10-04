---
tags: [module]
crate: db
milestone: M0, M2
---
# Module: db

**Purpose:** open and configure SQLite, run migrations from all modules, back up, restore and check integrity. It knows **nothing** about any module's schema.

**Requirements:** [[Req - Backup and Logging]]. **Design:** [[Database Strategy]].
**Depends on:** `core`, `rusqlite` (bundled), `sha2` (checksums).

## Tables
```sql
CREATE TABLE db_migrations (
  module      TEXT NOT NULL,
  version     INTEGER NOT NULL,
  name        TEXT NOT NULL,
  checksum    TEXT NOT NULL,
  applied_at  INTEGER NOT NULL,
  PRIMARY KEY (module, version)
);
CREATE TABLE db_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);  -- app_version, created_at, change_counter
```

## Public API
Built in A0 (migration runner):
```rust
pub use pos_core::{Migration, ModuleMigrations};   // defined in core, so modules don't depend on db

pub struct Db { /* writer: Mutex<Connection>, reader: Mutex<Connection> */ }
impl Db {
  pub fn open(path: &Path) -> Result<Db, DbError>;
  pub fn open_in_memory() -> Result<Db, DbError>;             // tests: shared-cache memory db, a new one per call
  pub fn write<T>(&self, f: impl FnOnce(&Transaction) -> Result<T, AppError>) -> Result<T, AppError>; // BEGIN IMMEDIATE … COMMIT
  pub fn read<T>(&self, f: impl FnOnce(&Connection) -> Result<T, AppError>) -> Result<T, AppError>;
}
pub fn pending(db: &Db, all: &[ModuleMigrations]) -> Result<Vec<(String, u32)>, DbError>;
pub fn migrate(db: &Db, all: &[ModuleMigrations]) -> Result<(), DbError>;
pub enum DbError { Sqlite(rusqlite::Error), ChecksumMismatch { module, version },
                   NewerDatabase { module, version }, BadMigrationList { module, detail } } // → AppError::Internal
```
- Every connection gets the settings from [[Database Strategy]] (WAL, `synchronous = FULL`, foreign keys, busy timeout, temp store, cache, mmap).
- `migrate` checks everything first: an edited migration (checksum = SHA-256 of the SQL) or a migration this app doesn't know (including a whole unknown module) refuses to start. Then it runs each module's missing migrations in one transaction per module, in the order `app` gives.
- A module's versions must go strictly up; a module may be listed only once.

Still to build (task B-db):
```rust
pub fn backup_to(db: &Db, file: &Path) -> Result<BackupInfo>;   // VACUUM INTO + verify
pub fn verify(file: &Path) -> Result<BackupInfo>;               // quick_check, schema version, sales count
pub fn quick_check(db: &Db) -> Result<bool>;
pub fn has_changed_since(db: &Db, marker: u64) -> bool;        // for the backup timer
pub fn prune_backups(dir: &Path, keep_last: usize, keep_daily_days: u32) -> Result<()>;
```
Restore is a file swap done by `app` at restart, using `verify` first.

## Tests
- Pragmas applied on both connections (file and memory). *(done)*
- `tests/contract.rs`: applied once, in order, idempotent; `pending`; checksum mismatch refuses; unknown newer version or module refuses; a failing migration leaves its module untouched; versions out of order refused; `write` rolls back on error. *(done)*
- Backup: produced file passes `verify`; works while a write transaction is open on the writer.
- Pruning keeps the right files (fixed clock).
- **Power-cut test (M13):** a child process loops completing transactions, the parent kills it at random moments, then reopens: `quick_check` ok, and the last committed row is present.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/db/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/db/` |
| **Screen folder** | `ui/src/modules/backup/` |
| **Requirements** | [[Req - Backup and Logging]] BAK-01 to BAK-06 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | The list of every module's migrations, file paths, and backup settings, all passed in by `app` |
| **Used by** (re-run their tests when the public API changes) | `app` startup and shutdown, the backup timer (`app/src/jobs.rs`), backup commands, and workflows that back up first (import, restore, stock-count posting) |
| **Test it on its own** | `cargo nextest run -p db` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- `db` must never know any module's tables. It only runs the SQL it's given.
- Never copy the live database file. Always use `VACUUM INTO`.
- A released migration is never edited. Changes go in a new file.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Done in A0: `Db` (open, settings, write, read), `migrate`, `pending`, the `db_migrations` table.
- To do (task B-db): the `db_meta` table, `backup_to`, `verify`, `quick_check`, `has_changed_since`, `prune_backups`, and their tests (backup while a write is open, pruning with a fixed clock).
- To do (M13): the power-cut test.
