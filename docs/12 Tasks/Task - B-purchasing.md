---
tags: [task]
id: B-purchasing
effort: medium
phase: B
depends_on: [A0]
status: ready
---
# Task B-purchasing: Build the `purchasing` module crate

**Suggested effort: medium**

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Module - purchasing]]: the whole page, especially **"Working on this module"**
- `crates/purchasing/CLAUDE.md`
- Requirements: [[Req - Purchasing]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
Suppliers, purchase orders, GRNs, supplier returns, payables ledger, payments, aging, statements.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/purchasing/` and [[Module - purchasing]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- `cargo nextest run -p purchasing` is green. `cargo clippy -p purchasing -- -D warnings` is clean. `cargo xtask check-deps` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - purchasing]] matches the code.
- Pull request opened from branch `task/B-purchasing`.

