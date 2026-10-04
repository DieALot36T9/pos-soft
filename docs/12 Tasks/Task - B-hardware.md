---
tags: [task]
id: B-hardware
phase: B
depends_on: [A0]
status: ready
---
# Task B-hardware: Build the `hardware` module crate

**Start in a fresh session.** Work only on this module. You don't need to know the rest of the app.

## Read first (and nothing else unless needed)
- [[Module - hardware]]: the whole page, especially **"Working on this module"**
- `crates/hardware/CLAUDE.md`
- Requirements: [[Req - Hardware and Printing]]
- [[Module Rules]], [[Module - core]], [[Coding Standards]], [[Writing Style]], [[TDD Workflow]]

## Build
Printer connections (Windows spooler, network, serial, CUPS, file, **Preview**), scale reading with profiles, port and printer listing, and the fakes (`MockPrinter`, `MockScale`, `FakeScale`) that behave like real devices.

- Rust only: the crate's migrations, public API and tests, exactly as its page describes.
- **No `app` wiring and no screens.** Those come in Phase C.
- **Tests first.** Every test listed on the module page, plus contract tests for the public API and views.
- Stay inside `crates/hardware/` and [[Module - hardware]]. If the page is wrong or missing something, fix the page in the same pull request and say why.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- `cargo nextest run -p hardware` is green. `cargo clippy -p hardware -- -D warnings` is clean. `cargo xtask check-deps` passes.
- Every test listed on the module page exists and names its requirement ID.
- [[Module - hardware]] matches the code.
- Pull request opened from branch `task/B-hardware`.

