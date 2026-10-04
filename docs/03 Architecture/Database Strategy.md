---
tags: [architecture, database]
---
# Database Strategy

Engine: **SQLite** (bundled with rusqlite, so every OS gets the same version, with FTS5 enabled). [[ADR-003 SQLite with rusqlite]]
Schema overview: [[Database Overview]]. Conventions: [[Data Conventions]].

## Files
| File | Location (Windows) |
|---|---|
| Database | `%APPDATA%\<app-id>\pos.sqlite` (+ `-wal`, `-shm`) |
| Backups | `%APPDATA%\<app-id>\backups\` + optional second folder |
| Images | `%APPDATA%\<app-id>\images\` |
| Logs | `%APPDATA%\<app-id>\logs\` |
Linux/macOS use the standard app data directory that Tauri reports.

## Connection settings (set on every connection)
```sql
PRAGMA journal_mode = WAL;        -- readers don't block the writer
PRAGMA synchronous = FULL;        -- a committed sale survives a power cut
PRAGMA foreign_keys = ON;         -- within-module FKs enforced
PRAGMA busy_timeout = 5000;
PRAGMA temp_store = MEMORY;
PRAGMA cache_size = -65536;       -- 64 MB page cache
PRAGMA mmap_size = 268435456;     -- 256 MB
```
`synchronous = FULL` costs a little write speed (a sale still commits in a few ms on an SSD) but closes the gap where WAL + NORMAL can lose the last transactions on power loss.

## Connections
- **Writer:** one `Connection` behind a `Mutex`. Every workflow takes it, runs `BEGIN IMMEDIATE`, and commits. This means there's only one writer, so there are no write conflicts.
- **Reader:** one separate `Connection` for reports, lists and searches. In WAL mode it reads a consistent snapshot while the writer works.
- Cart reads during checkout use the **writer** connection, so a cart change is always read back immediately.
- Queries on the checkout path use `prepare_cached`.

## Migrations
- Each module provides `ModuleMigrations { module: "catalog", migrations: &[Migration { version: 1, name: "init", sql: include_str!(...) }, ...] }`.
- `db::migrate(&mut conn, &[all modules])`:
  1. Creates `db_migrations(module, version, name, checksum, applied_at)` if missing.
  2. For each module in the order `app` provides, applies the missing versions in order, **each module's batch in one transaction**.
  3. Refuses to start if an applied migration's checksum changed, or if the database has a version this build doesn't know (a newer app made it).
- Because there are no cross-module foreign keys, module order doesn't matter for tables. Views only read their own module's tables, so they don't depend on order either.
- A backup is always taken before pending migrations run.

## Search (FTS5)
- `catalog_search` is an FTS5 table: `name, sku, barcodes, category, brand`, tokenizer `unicode61 remove_diacritics 2`, prefix indexes `prefix='2 3 4'`.
- The catalog module updates the product's search row **in the same transaction** as every product, barcode, category or brand change (rebuild that product's row). This is written in code rather than triggers, so it's easy to read and test.
- Exact barcode/SKU matches use normal unique indexes and are checked before FTS.

## Indexes
Each module page lists its indexes. General rules:
- Every column used in a `WHERE` or `JOIN` on the checkout path has an index.
- Sale lines: `(sale_id)`, `(product_id)`. Sales: `(business_date)`, `(shift_id)`, `(customer_id)`, unique `(invoice_no)`.
- Ledgers: `(product_id, at)`, `(ref_type, ref_id)`, `(business_date)`.
- Partial unique index: one open shift per till.

## Backups
- `VACUUM INTO '<file>'` from the reader connection. It produces a compact, consistent copy without stopping sales.
- Then the copy is opened read-only and `PRAGMA quick_check` runs. On success, it's copied to the second folder if one is set.
- Retention: the newest N, plus the last backup of each day for 30 days.
- Triggers: timer (if anything changed since the last backup, tracked by `PRAGMA data_version` plus a counter), on close, before migrations, imports, restores and count postings, and manual.

## Restore
1. `verify(file)`: `quick_check` and a schema version check (must not be newer than the app).
2. Back up the current database.
3. Close connections, replace the database file (and delete stale `-wal`/`-shm`), restart the app.

## Integrity
- `quick_check` on every startup (takes about 1 s per GB).
- `integrity_check` (the full check) runs on the newest backup once a day in the background, never on the live database during trading.
- `PRAGMA optimize` when the app closes.

## Capacity
At the reference size (5M sale lines at ~150 bytes, plus ledgers and indexes), the database is roughly 2–3 GB, well within SQLite's comfortable range. Performance tests in M13 use a generated database of this size (`cargo xtask seed --size reference`).

## Multi-till later (not in v1.0)
Because every id is a ULID and every document carries a `till_id`, a later version can add a LAN sync or a server till without changing ids or numbering. **Never** put the SQLite file on a network share; it corrupts. [[ADR-009 ULID ids and till identity]]
