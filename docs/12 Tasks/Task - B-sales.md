---
tags: [task]
id: B-sales
phase: B
depends_on: [A0]
status: ready
---
# Task B-sales: Build the `sales` module crate

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
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
- `cargo nextest run -p sales` is green. `cargo clippy -p sales -- -D warnings` is clean. `cargo xtask check-deps` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - sales]] matches the code.
- Pull request opened from branch `task/B-sales`.

## Result
*(filled in by the session: what was built, what's left open, notes for Phase C)*
