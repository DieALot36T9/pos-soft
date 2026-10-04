---
tags: [task]
id: B-barcode
phase: B
depends_on: [A0]
status: ready
---
# Task B-barcode: Build the `barcode` module crate

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Module - barcode]]: the whole page, especially **"Working on this module"**
- `crates/barcode/CLAUDE.md`
- Requirements: [[Req - Checkout]] (CHK-08), [[Req - Catalog]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
Scale-label rule parsing (Odoo style), check digits, internal barcodes, SVG and bitmap output.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/barcode/` and [[Module - barcode]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- `cargo nextest run -p barcode` is green. `cargo clippy -p barcode -- -D warnings` is clean. `cargo xtask check-deps` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - barcode]] matches the code.
- Pull request opened from branch `task/B-barcode`.

## Result
*(filled in by the session: what was built, what's left open, notes for Phase C)*
