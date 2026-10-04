---
tags: [task]
id: B-auth
model: claude-opus-5-5
effort: medium
phase: B
depends_on: [A0, A1]
status: ready
---
# Task B-auth: Build the `auth` module crate

**Model: Opus 5.5 · Effort: medium**

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Reuse Map]]: use the libraries listed for this module; don't hand-write what a listed library does
- [[Module - auth]]: the whole page, especially **"Working on this module"**
- `crates/auth/CLAUDE.md`
- Requirements: [[Req - Users and Security]], [[Roles and Permissions]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
Roles, permissions, users, argon2id hashing, login with lockout, admin-approval check, recovery code.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/auth/` and [[Module - auth]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- `cargo nextest run -p auth` is green. `cargo clippy -p auth -- -D warnings` is clean. `cargo deny check` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - auth]] matches the code.
- Pull request opened from branch `task/B-auth`.

