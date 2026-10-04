---
tags: [task]
id: B-shifts
phase: B
depends_on: [A0]
status: ready
---
# Task B-shifts: Build the `shifts` module crate

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Module - shifts]]: the whole page, especially **"Working on this module"**
- `crates/shifts/CLAUDE.md`
- Requirements: [[Req - Shifts and Cash]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
Shifts, money movements, denomination counts, expected cash, close, summary, reversal.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/shifts/` and [[Module - shifts]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- `cargo nextest run -p shifts` is green. `cargo clippy -p shifts -- -D warnings` is clean. `cargo xtask check-deps` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - shifts]] matches the code.
- Pull request opened from branch `task/B-shifts`.

## Result
*(filled in by the session: what was built, what's left open, notes for Phase C)*
