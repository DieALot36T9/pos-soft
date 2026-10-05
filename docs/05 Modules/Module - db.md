---
tags: [module]
crate: db
milestone: M0, M2
---
# Module: db

**Purpose:** open and configure SQLite, run migrations from all modules, back up, restore and check integrity. It knows **nothing** about any module's schema.

**Requirements:** [[Req - Backup and Logging]]. **Design:** [[Database Strategy]].
**Depends on:** `core`, `rusqlite` (bundled), `sha2` (checksums). Tests only: `chrono` (to build fixed clock times).

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
```
`migrate` creates this table itself, so `db` has no migration files of its own.

*There is no `db_meta` table (an earlier plan had one for `app_version`, `created_at` and `change_counter`). Task B-db found nothing needs it: the change counter only has to live while the app runs (a backup is always taken on close), and the schema version of a file comes from `db_migrations`.*

## Public API
Built in A0 (migration runner):
```rust
pub use pos_core::{Migration, ModuleMigrations};   // defined in core, so modules don't depend on db

pub struct Db { /* writer: Mutex<Connection>, reader: Mutex<Connection>, changes: AtomicU64 */ }
impl Db {
  pub fn open(path: &Path) -> Result<Db, DbError>;
  pub fn open_in_memory() -> Result<Db, DbError>;             // tests: shared-cache memory db, a new one per call
  pub fn write<T>(&self, f: impl FnOnce(&Transaction) -> Result<T, AppError>) -> Result<T, AppError>; // BEGIN IMMEDIATE … COMMIT
  pub fn read<T>(&self, f: impl FnOnce(&Connection) -> Result<T, AppError>) -> Result<T, AppError>;
}
pub fn pending(db: &Db, all: &[ModuleMigrations]) -> Result<Vec<(String, u32)>, DbError>;
pub fn migrate(db: &Db, all: &[ModuleMigrations]) -> Result<(), DbError>;
pub enum DbError { Sqlite(rusqlite::Error), ChecksumMismatch { module, version },
                   NewerDatabase { module, version }, BadMigrationList { module, detail },
                   CheckFailed { file, detail }, Io(std::io::Error) } // → AppError::Internal
```
- Every connection gets the settings from [[Database Strategy]] (WAL, `synchronous = FULL`, foreign keys, busy timeout, temp store, cache, mmap).
- `migrate` checks everything first: an edited migration (checksum = SHA-256 of the SQL) or a migration this app doesn't know (including a whole unknown module) refuses to start. Then it runs each module's missing migrations in one transaction per module, in the order `app` gives.
- A module's versions must go strictly up; a module may be listed only once.

Built in B-db (backups, checks, change tracking):
```rust
pub struct BackupInfo { pub file: PathBuf, pub size_bytes: u64,
                        pub migrations: Vec<(String, u32)> }  // newest version per module, sorted by module

pub fn backup_to(db: &Db, file: &Path) -> Result<BackupInfo, DbError>;          // VACUUM INTO, fsync, then check
pub fn verify(file: &Path, all: &[ModuleMigrations]) -> Result<BackupInfo, DbError>; // quick_check + not newer + not edited
pub fn open_read_only(file: &Path) -> Result<Connection, DbError>;              // app asks other modules about a backup
pub fn quick_check(db: &Db) -> Result<bool, DbError>;                          // live database, at startup
pub fn change_marker(db: &Db) -> u64;                                          // for the backup timer
pub fn has_changed_since(db: &Db, marker: u64) -> bool;
pub fn backup_file_name(clock: &dyn Clock, reason: &str) -> String;            // pos-YYYYMMDD-HHMMSS-<reason>.sqlite
pub fn prune_backups(dir: &Path, keep_last: usize, keep_daily_days: u32,
                     clock: &dyn Clock) -> Result<Vec<PathBuf>, DbError>;      // returns the deleted files
```
- **`backup_to`** runs `VACUUM INTO` on the **reader**, so it holds the last committed data and never waits for an open write. It refuses to overwrite an existing file. It then flushes the file to disk and checks it (`quick_check`, read-only). A failed check is `DbError::CheckFailed` (BAK-03: `app` shows the warning banner).
- **`verify`** is the check before a restore (BAK-05): `quick_check`, then every row of the file's `db_migrations` must be a migration this app knows, unchanged (`NewerDatabase`, `ChecksumMismatch`). An older backup is fine; `migrate` brings it up to date after the restore. A file that isn't a database is `CheckFailed`.
- **Sales count and last invoice number** for the restore list: `db` can't read the sales tables. `app` opens the file with `open_read_only` and passes the connection to the sales module's own read functions. *(The first plan had `verify` return the sales count; that would make `db` know a module's tables.)*
- **`quick_check`** returns `false` when the database is damaged (including "not a database"), and an error only when SQLite fails for another reason (for example the file is locked).
- **Change tracking:** `Db` keeps a counter in memory. A `write` that changed rows (SQLite's `total_changes` moved) and committed adds one; so does a `migrate` that ran something. A write that only read, or rolled back, is not a change. The timer keeps `change_marker` from its last backup and skips the next one if `has_changed_since` is false. Changes made outside this `Db` (another program) aren't seen; the single-instance rule (NFR-R8) means there are none.
- **`backup_file_name`** uses local date and time. The reason keeps lower-case letters, digits and dashes; anything else becomes a dash (`"Pre Restore"` → `pre-restore`).
- **`prune_backups`** keeps the newest `keep_last`, plus the newest backup of each day that is less than `keep_daily_days` days old (today is day 0, so 30 keeps today and the 29 days before). Files not named like a backup are never touched. `app` calls it for the main and the second folder.

Restore is a file swap done by `app` at restart, using `verify` first.

## Tests
- Pragmas applied on both connections (file and memory). *(done, `src/open.rs`)*
- `tests/contract.rs`: applied once, in order, idempotent; `pending`; checksum mismatch refuses; unknown newer version or module refuses; a failing migration leaves its module untouched; versions out of order refused; `write` rolls back on error. *(done)*
- `tests/backup.rs` *(done)*:
  - BAK-01: produced file passes `verify` and holds the data; works from a memory database; works while a write transaction is open (the unfinished row is left out); never overwrites an existing file.
  - BAK-01: change marker moves only when a write changes rows; a rolled-back write is not a change; running migrations is a change, re-running none is not.
  - BAK-03: `verify` refuses a file that isn't a database, a damaged database, and a missing file (without creating it).
  - BAK-05: `verify` refuses a backup from a newer app (newer version or unknown module), and accepts one from an older app.
  - BAK-06: `quick_check` passes on a healthy file and memory database.
- `tests/prune.rs` *(done, fixed clock)*: BAK-01 file name from local time and reason; unsafe reason letters; keep newest N; keep last of each day for 30 days (29 days old kept, 30 dropped); other files left alone; missing folder is an error. Unit tests in `src/prune.rs` for reading names.
- `tests/power_cut.rs` **(NFR-R1, BAK-06)** *(done)*: the test binary starts itself again as a child that loops committing rows and prints each number after its commit returns. The parent kills it at a random moment (8 rounds), then reopens: `quick_check` ok, and the last reported row is present. Killing the process doesn't lose the operating system's file cache, so a real power cut on the shop's Windows PC is still checked by hand in M13.

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
- Done in B-db: `backup_to`, `verify`, `open_read_only`, `quick_check`, `change_marker`, `has_changed_since`, `backup_file_name`, `prune_backups`, and the power-cut test. `db_meta` dropped (see Tables).
- **Done in `app` (C-M2):** the backup timer, backups on close, before migrations and before restores, the copy to the second folder (BAK-02), the restore file swap at the next start, and the startup recovery screen (`ui/src/modules/backup/`, with the Backups screen).
- Still for `app`: backups before imports (C-M12) and count postings (C-M5), and the daily `integrity_check` on the newest backup.
- For M13: the power cut on a real Windows PC (pulling the plug), on top of the killed-process test here.
- Screens (`ui/src/modules/backup/`): Backups (list, Back up now, restore with a confirmation) and the recovery screen, built in C-M2.
