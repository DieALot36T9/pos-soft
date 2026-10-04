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
```rust
pub struct Migration { pub version: u32, pub name: &'static str, pub sql: &'static str }
pub struct ModuleMigrations { pub module: &'static str, pub migrations: &'static [Migration] }

pub struct Db { /* writer: Mutex<Connection>, reader: Mutex<Connection> */ }
impl Db {
  pub fn open(path: &Path) -> Result<Db>;
  pub fn open_in_memory() -> Result<Db>;                        // tests (shared-cache memory db)
  pub fn write<T>(&self, f: impl FnOnce(&Transaction) -> Result<T>) -> Result<T>; // BEGIN IMMEDIATE … COMMIT
  pub fn read<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T>;
}
pub fn pending(db: &Db, all: &[ModuleMigrations]) -> Result<Vec<(String, u32)>>;
pub fn migrate(db: &Db, all: &[ModuleMigrations]) -> Result<()>;
pub fn backup_to(db: &Db, file: &Path) -> Result<BackupInfo>;   // VACUUM INTO + verify
pub fn verify(file: &Path) -> Result<BackupInfo>;               // quick_check, schema version, sales count
pub fn quick_check(db: &Db) -> Result<bool>;
pub fn has_changed_since(db: &Db, marker: u64) -> bool;        // for the backup timer
pub fn prune_backups(dir: &Path, keep_last: usize, keep_daily_days: u32) -> Result<()>;
```
Restore is a file swap done by `app` at restart, using `verify` first.

## Tests
- Pragmas applied on both connections.
- Migrations: applied once, in order, idempotent; checksum mismatch refuses; unknown newer version refuses.
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
- Not started (see [[Task Index]]).
