---
tags: [task]
id: B-pricing
model: claude-opus-5-5
effort: high
phase: B
depends_on: [A0, A1]
status: in review
---
# Task B-pricing: Build the `pricing` module crate

**Model: Opus 5.5 · Effort: high**

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Reuse Map]]: use the libraries listed for this module; don't hand-write what a listed library does
- [[Module - pricing]]: the whole page, especially **"Working on this module"**
- `crates/pricing/CLAUDE.md`
- Requirements: [[Req - Pricing and Promotions]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
Price lists, tiers, promotions, and the **pure** pricing engine with every PRC-06 example as a test.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/pricing/` and [[Module - pricing]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- `cargo nextest run -p pricing` is green. `cargo clippy -p pricing -- -D warnings` is clean. `cargo deny check` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - pricing]] matches the code.
- Pull request opened from branch `task/B-pricing`.

