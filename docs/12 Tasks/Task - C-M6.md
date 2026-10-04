---
tags: [task]
id: C-M6
model: claude-opus-5-5
effort: high
phase: C
depends_on: [C-M5, B-sales, B-shifts, B-pricing, B-tax]
status: waiting
---
# Task C-M6: Shifts and checkout

**Model: Opus 5.5 · Effort: high**

**Start in a fresh session**, only after the tasks it depends on are merged: C-M5, B-sales, B-shifts, B-pricing, B-tax.
This task connects finished module crates into the app (`crates/app/`) and builds their screens.

## Read first
- This milestone's row in [[Roadmap]]
- Requirements: [[Req - Checkout]], [[Req - Payments]], [[Req - Shifts and Cash]]
- [[Module - app]] and the "Working on this module" section of each module involved
- [[Workflows]] (only the sections for this milestone), [[Frontend Architecture]], [[Design System]], [[Writing Style]]

## Build
Scan and cart workflows (reprice on every change), the [[Checkout Screen]], weight dialog (fake scale and manual), scale stickers, hold/resume/cancel, payment dialog with all methods, change, rounding, complete-sale workflow, open/close shift, cash in/out.

- Commands stay thin, each workflow is one transaction, and every new command gets its permission test.
- Screens: flat, colour-coded, a tooltip on every button and field, all text in the module's `text/en.ts`.
- If a module crate is missing something, add it **in that module** (with its contract test), then use it here.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- A full day of selling by keyboard only; killing the app at any moment loses no completed sale. **Alpha build 1.**
- Integration tests in `crates/app/tests/` and Playwright flows for the new screens are green.
- Module pages and [[Workflows]] match the code.
- Pull request from `task/C-M6`, with the **Mac build** for the product owner to try (`build-mac` label).

