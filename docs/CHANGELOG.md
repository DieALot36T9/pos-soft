# Changelog

All notable changes to this project are recorded here.
Format: [Keep a Changelog](https://keepachangelog.com/). Versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]
### Added
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
