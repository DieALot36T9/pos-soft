---
tags: [task]
id: B-printing
phase: B
depends_on: [A0]
status: ready
---
# Task B-printing: Build the `printing` module crate

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Module - printing]]: the whole page, especially **"Working on this module"**
- `crates/printing/CLAUDE.md`
- Requirements: [[Req - Hardware and Printing]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
Document model, receipt/report/label/shelf-tag layouts, drawing to a picture with cosmic-text + tiny-skia, PNG output, ESC/POS and TSPL encoding, golden-image tests (first images are approved by the product owner in C-M1).

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/printing/` and [[Module - printing]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- `cargo nextest run -p printing` is green. `cargo clippy -p printing -- -D warnings` is clean. `cargo xtask check-deps` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - printing]] matches the code.
- Pull request opened from branch `task/B-printing`.

