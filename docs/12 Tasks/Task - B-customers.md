---
tags: [task]
id: B-customers
model: claude-opus-5-5
effort: medium
phase: B
depends_on: [A0, A1]
status: ready
---
# Task B-customers: Build the `customers` module crate

**Model: Opus 5.5 · Effort: medium**

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Reuse Map]]: use the libraries listed for this module; don't hand-write what a listed library does
- [[Module - customers]]: the whole page, especially **"Working on this module"**
- `crates/customers/CLAUDE.md`
- Requirements: [[Req - Customers and Loyalty]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
Customers, phone cleaning, search, points and credit ledgers with balances, statements, points maths.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/customers/` and [[Module - customers]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- `cargo nextest run -p customers` is green. `cargo clippy -p customers -- -D warnings` is clean. `cargo deny check` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - customers]] matches the code.
- Pull request opened from branch `task/B-customers`.

