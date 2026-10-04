---
tags: [task]
id: B-tax
phase: B
depends_on: [A0]
status: ready
---
# Task B-tax: Build the `tax` module crate

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Module - tax]]: the whole page, especially **"Working on this module"**
- `crates/tax/CLAUDE.md`
- Requirements: [[Req - Tax]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
Tax rates and parts, `calc_line`, `split_components`, the TAX-05 table as tests, property tests.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/tax/` and [[Module - tax]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- `cargo nextest run -p tax` is green. `cargo clippy -p tax -- -D warnings` is clean. `cargo xtask check-deps` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - tax]] matches the code.
- Pull request opened from branch `task/B-tax`.

## Result
*(filled in by the session: what was built, what's left open, notes for Phase C)*
