---
tags: [task]
id: B-inventory
model: claude-opus-5-5
effort: high
phase: B
depends_on: [A0]
status: ready
---
# Task B-inventory: Build the `inventory` module crate

**Model: Opus 5.5 · Effort: high**

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Module - inventory]]: the whole page, especially **"Working on this module"**
- `crates/inventory/CLAUDE.md`
- Requirements: [[Req - Inventory]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
Ledger, levels, batches with FEFO, adjustments and reasons, reorder rules, low stock, near expiry, stock counts, `verify`, randomized ledger tests.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/inventory/` and [[Module - inventory]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- `cargo nextest run -p inventory` is green. `cargo clippy -p inventory -- -D warnings` is clean. `cargo xtask check-deps` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - inventory]] matches the code.
- Pull request opened from branch `task/B-inventory`.

