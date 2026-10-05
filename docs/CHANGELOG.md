# Changelog

All notable changes to this project are recorded here.
Format: [Keep a Changelog](https://keepachangelog.com/). Versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]
### Added (task B-audit)
- `audit`: the append-only `audit_events` table with triggers that refuse UPDATE and DELETE, the `audit_v_events` view, `record` (refuses badly named actions, non-object details and secret-sounding keys) and `query` (newest first, filters by business date, user and action, pages of up to 500). USR-08.

### Added (task B-io)
- `io`: reads .csv (BOM, comma/semicolon/tab, quoted newlines, must be UTF-8) and .xlsx (first or chosen sheet) into a table of text cells; writes .csv (UTF-8 BOM, formula guard) and .xlsx (real numbers with number formats); exports a report to .xlsx with the shop name, report name, date range and time made (RPT-09); matches file headers to fields (`auto_map`); reads money, quantities, yes/no and dates without ever rounding (IO-04); lists bad cells with row and column (`CellErr`). 50k rows read in about 1.3 s (IO-05). Uses `csv`, `calamine` and `rust_xlsxwriter` (added to the Reuse Map).

### Added (task B-db)
- `db`: backups with `VACUUM INTO` (`backup_to`, then a read-only `quick_check`), `verify` before a restore (refuses damaged files, newer or edited migrations), `open_read_only`, `quick_check` for the startup check, change tracking for the backup timer (`change_marker`, `has_changed_since`), `backup_file_name` and `prune_backups` (newest N plus the last of each day). BAK-01, BAK-03, BAK-05, BAK-06.
- `db`: power-cut test: a child process that keeps committing is killed at random moments, and the last committed row must survive (NFR-R1).
### Changed (task B-db)
- `db` has no `db_meta` table: nothing needed it. `verify` no longer returns a sales count; `app` reads it through the sales module using `open_read_only`, so `db` still knows no module's tables.

### Added (task B-printing)
- `printing` crate: a document model; layouts for sale, void and return receipts, X/Z reports, labels and shelf tags; drawing to a 1-bit picture with cosmic-text on a tiny-skia page; PNG previews; shop logo dithering; ESC/POS (picture strips, cut, drawer pulse) and TSPL bytes. Every printed word comes from `ReceiptLabels` (ADR-022). Requirements HW-02 to HW-04, HW-06, HW-10, CAT-12, CAT-13, SHF-08, RET-04, RET-06, TAX-08, SET-14.
- Bundled fonts: Noto Sans Regular and Bold, plus a two-letter cut of Noto Sans Bengali for the taka sign ৳ (Noto Sans has none). SIL Open Font License.
- Golden images for 9 printouts in `crates/printing/tests/golden/`, waiting for the product owner's approval in C-M1.
- The cash drawer pulse is written as the full `ESC p m t1 t2`, because the `escpos` crate's own command leaves out the two timing bytes (the printer would swallow the next two bytes it receives).
### Changed (task B-printing)
- `deny.toml` ignores RUSTSEC-2026-0192 (`ttf-parser` unmaintained), which comes only through `cosmic-text` and reads only the bundled fonts.

### Added (task B-barcode)
- `barcode` module: reads scale stickers with Odoo-style rules (CHK-08), checks GTIN check digits (CAT-04), makes internal EAN-13 codes and finds rules that clash with the internal prefix (CAT-11), and draws EAN-13, EAN-8, UPC-A and Code 128 as SVG or as a printer picture (CAT-12). Uses `barcoders`; tests read every picture back with `rxing`.
- The barcode module page now says the pattern leaves out the check digit, and that functions with bad input return `Option`/`Result` instead of panicking.
### Changed (task A1: reuse before building, ADR-023)
- Module crates are kept apart by **cargo-deny** (`deny.toml`, `wrappers = ["app"]`), which also checks licences, advisories and sources. CI runs `cargo deny check` and `scripts/check-module-bans.sh`, which proves a module depending on another module is refused. The hand-written xtask command for this is gone; `cargo xtask new-module` adds new modules to `deny.toml`.
- Screen module folders are kept apart by **eslint-plugin-boundaries**, with a test that proves `pnpm lint` refuses a module importing another module.
- Screen text uses **i18next + react-i18next**. Each `text/en.ts` is a namespace registered in `ui/src/app/i18n.ts`; keys are typed, so a wrong key is a build error. Error messages use `{{name}}` placeholders.
- `core::format_money` groups digits with **num-format**. All money tests and `money_vectors.json` pass unchanged.
- Only one copy of the app can run (**tauri-plugin-single-instance**); a second start brings the open window to the front.
- GitHub Actions moved to `actions/checkout@v7`, `actions/setup-node@v7`, `pnpm/action-setup@v6`, `actions/upload-artifact@v7` (no more Node 20 warning). `scripts/cloud-setup.sh` installs cargo-deny.

### Added
- Cloud sessions set themselves up: a SessionStart hook runs `scripts/cloud-setup.sh`.
- M0 Foundations (task A0): Cargo workspace with 19 crates, each module with its own `CLAUDE.md`.
- `core`: Money, Qty, Bps, Id, Timestamp, BusinessDate, Clock, AppError, Permission, Secret, ReportTable, Bitmap1, rounding, allocate, cash_round, moving_average, business_date, format_money/parse_money, Migration/ModuleMigrations. Examples E1–E16 and shared money format examples.
- `db`: opening SQLite with the agreed settings and the migration runner (`db_migrations`, checksums, refuses edited or newer migrations).
- `app`: Tauri 2 window with the flat app frame and one example command (`app_info`).
- UI workspace: React 19, TypeScript, Vite, Tailwind v4 without shadows or animations, colour tokens with a 7 : 1 contrast test, `ActionButton`/`IconButton`/`Tip` with a required tooltip, `useText()`, fake bridge (`pnpm dev:mock`).
- `cargo xtask check-deps`, `gen-types` and `new-module`. CI for Linux, Windows and the Mac `.dmg`. `scripts/cloud-setup.sh`. `CONTRIBUTING.md`.
- Design vault: charter, scope, requirements, architecture, ADRs, module pages, UI design, test strategy, guides, runbooks.
- Design System: flat look, fixed colour code, tooltips on every button and field (ADR-021).
- Writing Style guide: plain language for docs, code, comments and screen text.
- Glossary section explaining technical words in plain English.
### Changed
- The `core` crate's package is named `pos-core` (`pos_core::Money`), because Rust already has a crate called `core`.
- `Migration` and `ModuleMigrations` live in `core` instead of `db`, so modules can describe their migrations without depending on `db`.
- Money helpers in `core` return `Result<_, AppError>`, so an overflow is an error instead of a wrong number.
- English only for v1.0, with screen and receipt text kept in one place so languages can be added later (ADR-022). Removed the second-language product name and Bangla fonts.
- Testing happens on macOS for now. Windows is still the shop target, checked in CI and on a real Windows PC in M13 (ADR-017).
- Receipts are printed as pictures so any currency symbol and the logo work on every printer (ADR-012).
