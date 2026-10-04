---
tags: [task]
id: C-M5
phase: C
depends_on: [C-M4, B-inventory]
status: waiting
---
# Task C-M5: Stock

**Start in a fresh session**, only after the tasks it depends on are merged: C-M4, B-inventory.
This task connects finished module crates into the app (`crates/app/`) and builds their screens.

## Read first
- This milestone's row in [[Roadmap]]
- Requirements: [[Req - Inventory]]
- [[Module - app]] and the "Working on this module" section of each module involved
- [[Workflows]] (only the sections for this milestone), [[Frontend Architecture]], [[Design System]], [[Writing Style]]

## Build
Adjustments, stock levels, history, batches, low-stock and near-expiry alerts on the dashboard, stock counts (create, count by scanning, review, post with backup).

- Commands stay thin, each workflow is one transaction, and every new command gets its permission test.
- Screens: flat, colour-coded, a tooltip on every button and field, all text in the module's `text/en.ts`.
- If a module crate is missing something, add it **in that module** (with its contract test), then use it here.

## Done when
- The "levels equal ledger" check passes after the randomized test.
- Integration tests in `crates/app/tests/` and Playwright flows for the new screens are green.
- Module pages and [[Workflows]] match the code.
- Pull request from `task/C-M5`, with the **Mac build** for the product owner to try (`build-mac` label).

## Result
*(filled in by the session)*
