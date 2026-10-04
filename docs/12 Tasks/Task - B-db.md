---
tags: [task]
id: B-db
phase: B
depends_on: [A0]
status: ready
---
# Task B-db: Build the `db` module crate

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Module - db]]: the whole page, especially **"Working on this module"**
- `crates/db/CLAUDE.md`
- Requirements: [[Req - Backup and Logging]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
The migration runner was built in A0. Build the rest: backups (`VACUUM INTO` + check), `verify`, `prune_backups`, `quick_check`, change tracking for the backup timer, and the power-cut test.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/db/` and [[Module - db]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- `cargo nextest run -p db` is green. `cargo clippy -p db -- -D warnings` is clean. `cargo xtask check-deps` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - db]] matches the code.
- Pull request opened from branch `task/B-db`.

## Result
*(filled in by the session: what was built, what's left open, notes for Phase C)*
