---
tags: [task]
id: B-audit
phase: B
depends_on: [A0]
status: ready
---
# Task B-audit: Build the `audit` module crate

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Module - audit]]: the whole page, especially **"Working on this module"**
- `crates/audit/CLAUDE.md`
- Requirements: [[Req - Users and Security]] (USR-08)
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
The append-only audit table with triggers, `record`, `query`.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/audit/` and [[Module - audit]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- `cargo nextest run -p audit` is green. `cargo clippy -p audit -- -D warnings` is clean. `cargo xtask check-deps` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - audit]] matches the code.
- Pull request opened from branch `task/B-audit`.

