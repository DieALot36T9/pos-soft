---
tags: [task]
id: C-M7
effort: medium
phase: C
depends_on: [C-M6, B-reports]
status: waiting
---
# Task C-M7: Receipts and devices at the till

**Suggested effort: medium**

**Start in a fresh session**, only after the tasks it depends on are merged: C-M6, B-reports.
This task connects finished module crates into the app (`crates/app/`) and builds their screens.

## Read first
- This milestone's row in [[Roadmap]]
- Requirements: [[Req - Hardware and Printing]], [[Req - Shifts and Cash]] (SHF-05 to SHF-08)
- [[Module - app]] and the "Working on this module" section of each module involved
- [[Workflows]] (only the sections for this milestone), [[Frontend Architecture]], [[Design System]], [[Writing Style]]

## Build
Receipts printed after sales (Preview printer for now), reprint, drawer pulse, scale in the weight dialog, X and Z reports, print-failure banner with retry and system-printer fallback.

- Commands stay thin, each workflow is one transaction, and every new command gets its permission test.
- Screens: flat, colour-coded, a tooltip on every button and field, all text in the module's `text/en.ts`.
- If a module crate is missing something, add it **in that module** (with its contract test), then use it here.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- Receipts match the approved golden pictures. **Alpha build 2.**
- Integration tests in `crates/app/tests/` and Playwright flows for the new screens are green.
- Module pages and [[Workflows]] match the code.
- Pull request from `task/C-M7`, with the **Mac build** for the product owner to try (`build-mac` label).

