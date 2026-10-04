---
tags: [task]
id: B-sales
model: claude-opus-5-5
effort: high
phase: B
depends_on: [A0, A1]
status: ready
---
# Task B-sales: Build the `sales` module crate

**Model: Opus 5.5 · Effort: high**

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Reuse Map]]: use the libraries listed for this module; don't hand-write what a listed library does
- [[Module - sales]]: the whole page, especially **"Working on this module"**
- `crates/sales/CLAUDE.md`
- Requirements: [[Req - Checkout]], [[Req - Payments]], [[Req - Returns and Voids]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
Payment methods, carts and held carts, completing a sale from **already-priced** lines, gapless invoice numbers, returns, voids, the `sales_v_*` views.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/sales/` and [[Module - sales]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- `cargo nextest run -p sales` is green. `cargo clippy -p sales -- -D warnings` is clean. `cargo deny check` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - sales]] matches the code.
- Pull request opened from branch `task/B-sales`.

