---
tags: [task]
id: B-catalog
phase: B
depends_on: [A0]
status: ready
---
# Task B-catalog: Build the `catalog` module crate

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Module - catalog]]: the whole page, especially **"Working on this module"**
- `crates/catalog/CLAUDE.md`
- Requirements: [[Req - Catalog]], [[Req - Checkout]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
Products, units, categories, brands, barcodes, variants, kits, images, FTS search, archive, price history, sequences. Include the 50k-product search benchmark.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/catalog/` and [[Module - catalog]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- `cargo nextest run -p catalog` is green. `cargo clippy -p catalog -- -D warnings` is clean. `cargo xtask check-deps` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - catalog]] matches the code.
- Pull request opened from branch `task/B-catalog`.

## Result
*(filled in by the session: what was built, what's left open, notes for Phase C)*
