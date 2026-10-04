---
tags: [task]
id: A0
model: claude-opus-5-5
effort: medium
phase: A
status: in review
---
# Task A0: Foundations (milestone M0)

**Model: Opus 5.5 · Effort: medium**

**Start in a fresh session.** This is the first task. Everything else depends on it.

## Read first
- `CLAUDE.md` (repo root), [[Repository Layout]], [[Module Map]], [[Module Rules]], [[Tech Stack]], [[Coding Standards]], [[Writing Style]]
- [[Module - core]] (you build `core` fully in this task)
- [[Frontend Architecture]], [[Design System]] (for the empty app shell and colour tokens)

## Build
1. **Cargo workspace** as in [[Repository Layout]]: root `Cargo.toml` with workspace lints and build profiles, `rust-toolchain.toml`.
2. **All 19 crates**, empty except `core`. Each has `Cargo.toml` (module crates depend only on `core`), `src/lib.rs`, a `migrations/` folder (for crates with tables), `tests/contract.rs` (empty test), and a **`CLAUDE.md`** made from `Templates/Template - Module CLAUDE.md`.
3. **`core`** completely: `Money`, `Qty`, `Bps`, `Id`, `Timestamp`, `BusinessDate`, `Clock`, `AppError`, `Permission`, `Secret`, `ReportTable`, `Bitmap1`, rounding, `allocate`, `cash_round`, `moving_average`, `business_date`, `format_money`/`parse_money`. **Tests first**, using examples E1–E16 from [[Money Tax and Rounding]]. Also add the shared `crates/core/tests/money_vectors.json`.
4. **`db`** with the migration runner only (open, settings applied to each connection, `migrate`, the `db_migrations` table) and its tests. Backups come later (task B-db).
5. **Tauri 2 app** (`crates/app`) that opens an empty window with the **flat app frame** and colour tokens from [[Design System]]. The window ignores dark mode. One example command to prove the bridge works.
6. **UI workspace** (`ui/`): React 19, TypeScript strict, Vite, Tailwind v4 (shadow utilities removed), shadcn/ui base, `shared/` folders, the `useText()` hook with `text/en.ts`, the fake bridge (`pnpm dev:mock`), the colour-contrast test (all pairs ≥ 7:1), `ActionButton`/`IconButton`/`Tip` with the required `tip` prop.
7. **ts-rs** type generation (`cargo xtask gen-types`).
8. **`xtask`**: `check-deps` (fails if a module crate depends on another module), `gen-types`, `new-module`.
9. **CI** (`.github/workflows/ci.yml`): Linux on every push (fmt, clippy, tests, check-deps, type check, UI lint and tests). Windows on pull requests. A Mac `.dmg` build on the `build-mac` label or a version tag, attached as a downloadable artifact.
10. **Cloud setup script** (`scripts/cloud-setup.sh`): installs Rust, Node/pnpm, cargo-nextest and Tauri's Linux packages, so every cloud session starts ready.
11. `CONTRIBUTING.md` (short, linking to the guides).

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- CI is green on Linux and Windows. The Mac build opens on the product owner's Mac to an empty, flat app window.
- `cargo xtask check-deps` fails if a module crate is made to depend on another module (prove it with a test).
- All `core` examples pass.
- [[Module - core]] and [[Module - db]] pages match the code.


## Result
- **Workspace:** root `Cargo.toml` (shared lints, profiles, dependency versions), `rust-toolchain.toml` (1.97.0), `.cargo/config.toml`, `clippy.toml`.
- **19 crates:** `core` (package `pos-core`), `db`, 16 business modules, `hardware` and `app`. Each module crate has `Cargo.toml` (depends on `pos-core` only), `src/lib.rs`, `tests/contract.rs` and `CLAUDE.md`. The 11 crates with tables have `migrations/` and `pub fn migrations()` (empty list for now).
- **core:** everything in [[Module - core]]; examples E1–E16, `money_vectors.json`, property tests.
- **db:** the migration runner and its tests ([[Module - db]]). Backups are task B-db.
- **app:** Tauri 2 window with the flat frame, fixed light theme, `app_info` command ([[Module - app]]).
- **UI:** React 19, TypeScript strict, Vite, Tailwind v4 (no shadows/blur/animations, only our colour tokens), shadcn base, `useText()` + `text/en.ts`, fake bridge (`pnpm dev:mock`), contrast test, flat-look test, `ActionButton`/`IconButton`/`Tip`, module-boundary lint rule.
- **xtask:** `check-deps` (tests prove it fails for a module → module dependency), `gen-types [--check]`, `new-module`.
- **CI:** `.github/workflows/ci.yml` — Linux on every push, Windows on pull requests, Mac `.dmg` on the `build-mac` label or a `v*` tag.
- **Cloud setup:** `scripts/cloud-setup.sh`, run at the start of every cloud session by the SessionStart hook in `.claude/settings.json`.
- **Left open:** listed under "Open items" on [[Module - core]], [[Module - db]] and [[Module - app]]. The Mac build still needs to be opened on the product owner's Mac.
