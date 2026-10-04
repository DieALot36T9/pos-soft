---
tags: [architecture, modules, rules]
---
# Module Rules

These rules let someone work on one module without breaking the others. They're **enforced by the compiler and CI** where possible, and by the review checklist otherwise.

## R1. Dependencies
- A module crate's `Cargo.toml` may list: `core`, `rusqlite`, `serde`, `thiserror`, `ulid`, and third-party crates **approved on its module page**.
- **A module never depends on another module crate.** CI runs `cargo xtask check-deps`, which fails if any crate other than `app` lists a module crate.
- `app` is the only crate that depends on every module.
- One exception, for tests only: `reports` may list modules as **dev-dependencies**, so its tests can build those modules' views with their own migrations. `check-deps` allows this and nothing else.

## R2. Data ownership
- Every table and view is prefixed with its module name: `catalog_products`, `sales_v_lines`.
- A module **reads and writes only its own tables.**
- Other modules see its data in exactly two ways:
  1. **Public Rust functions** (for workflows), and
  2. **Published read views** named `<module>_v_<name>` (for reports and lists). Views are part of the module's public contract and covered by contract tests.
- **No foreign keys across modules** ([[ADR-005 No cross-module foreign keys]]). References to another module's rows are plain `TEXT` ids. Workflows check that they exist.

## R3. Public API shape
- `lib.rs` only declares modules and re-exports the public API. Everything else is `pub(crate)` or private.
- Functions that change data take `&rusqlite::Transaction`. Functions that read take `&rusqlite::Connection`. **Modules never open connections or start transactions**; the caller (app workflow) owns the transaction.
- Inputs and outputs are plain structs with `serde` and `ts-rs` derives where the UI needs them.
- Each module has one error enum (`CatalogError`, …) that converts into `core::AppError`.
- Prefer **pure functions** (no database) for rules (pricing engine, tax calculation, barcode parsing, receipt layout), with thin loader functions around them.

## R4. Migrations
- Each module ships `migrations/NNN_description.sql`, embedded with `include_str!` and exposed as `pub fn migrations() -> ModuleMigrations`.
- **Migrations are never edited after release.** Changes go in a new file. The runner stores checksums and refuses to start on a mismatch.
- Migrations must work on an empty database **and** on any earlier released version (tested).

## R5. Changing a public API or view
1. Update the contract test first (it fails).
2. Change the code. Update every caller in `app` in the same change.
3. Update the module page's API/views section.
4. If a view's columns change: add a new migration that recreates the view. Never break a column other code reads without changing that code in the same change.

## R6. Tests per module
- Unit tests next to the code (`#[cfg(test)]`).
- `tests/contract.rs`: the public API and views behave as the module page says.
- Every test uses an in-memory database built by the module's own migrations (`test_db()` helper).

## R7. UI side
- Each module's UI lives in `ui/src/modules/<module>/` with its own `api.ts`, pages, components and `text/en.ts` (all screen text).
- UI modules import only `ui/src/shared/*`, `ui/src/generated/*` and their own folder. **No imports from another module's UI folder** (eslint `no-restricted-imports` rule). Shared widgets go to `shared/`.

## R8. Every module carries its own instructions
- Each module page in `05 Modules/` has a **"Working on this module"** section: folders, requirements, what it gets from `app`, who uses it, how to test it alone, and its traps. Reading that page, these rules and [[Module - core]] is enough to work on the module.
- Each module folder has a short `CLAUDE.md` (`crates/<name>/` and `ui/src/modules/<folder>/`) that points to its page. Claude Code loads it automatically when working in that folder. Template: `Templates/Template - Module CLAUDE.md`.
- Each module can be built and tested on its own: `cargo nextest run -p <name>`.
- When a module's public API changes, its page's **Used by** list says which `app` workflows to re-test.

## Review checklist
- [ ] No new module-to-module dependency
- [ ] No SQL touching another module's tables
- [ ] New tables/views prefixed; migration added, not edited
- [ ] Public API changes covered by contract tests
- [ ] Module page updated, including its "Working on this module" section
- [ ] Money/qty/rates use `Money`/`Qty`/`Bps`
