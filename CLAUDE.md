# Supershop POS: rules for working in this repo

The design lives in the Obsidian vault at `docs/`. Start at `docs/00 Home.md`.

## Before changing code
0. If you were given a task id (e.g. `B-catalog`), read `docs/12 Tasks/Task - <id>.md` first and follow it. One task per session.
1. Work on **one module at a time**. Read its page in `docs/05 Modules/Module - <name>.md`, especially **"Working on this module"**, and its folder's `CLAUDE.md`. Stay inside that module's folders.
2. Read `docs/03 Architecture/Module Rules.md` and `docs/09 Guides/Coding Standards.md` if you haven't in this session.

## UI redesign sessions
If you're asked to mock or redesign a screen, start at `docs/13 UI Review/UI Review Home.md` and follow it. One page per session.

## Hard rules
- A module crate depends only on `core` (plus approved third-party crates). Only `app` connects modules. Never add one module crate to another's `Cargo.toml`.
- A module reads and writes only its own tables (`<module>_*`). Other modules read it through its public functions or its published `<module>_v_*` views.
- Money is `Money` (i64 minor units). Quantity is `Qty` (i64 thousandths). Tax rates are `Bps`. Never use floats for these.
- Business rules live in Rust. The UI displays data and calls commands; it never calculates totals, tax or stock.
- Permissions are checked in `app` command handlers, in Rust.
- Ledger tables are append-only. Never UPDATE or DELETE ledger rows.
- **Reuse before building:** check `docs/03 Architecture/Reuse Map.md` first. Use established standards and maintained, permissively licensed libraries. Never copy code from GPL/LGPL/AGPL projects.
- Tests first (red → green → refactor). A bug fix starts with a failing test.
- Keep it simple: plain functions and structs. Don't add macros, trait hierarchies, async or generics unless a page in the vault records the need.
- **Plain language everywhere** (docs, comments, names, screen text): see `docs/09 Guides/Writing Style.md`.
- **UI:** flat (no shadows, gradients or animations), the fixed colour code, and a tooltip on every button and field. See `docs/07 UI Design/Design System.md`.
- **Language:** English only, but all screen text goes in the module's `text/en.ts` (never inline), and printed words come from `ReceiptLabels`, so other languages can be added later (ADR-022).
- **Test machine:** the product owner tests on macOS for now. Windows is the shop target; keep CI green on Windows (ADR-017).

## Definition of Done
See `docs/08 Testing/Definition of Done.md`. In short: tests pass, `cargo clippy -- -D warnings`, `cargo fmt --check`, `pnpm lint`, `pnpm test`, generated TS types are up to date, and **the module page and changelog are updated in the same change**.
