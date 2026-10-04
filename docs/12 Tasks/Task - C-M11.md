---
tags: [task]
id: C-M11
model: claude-opus-5-5
effort: medium
phase: C
depends_on: [C-M10, B-reports, B-io]
status: waiting
---
# Task C-M11: Reports

**Model: Opus 5.5 · Effort: medium**

**Start in a fresh session**, only after the tasks it depends on are merged: C-M10, B-reports, B-io.
This task connects finished module crates into the app (`crates/app/`) and builds their screens.

## Read first
- This milestone's row in [[Roadmap]]
- Requirements: [[Req - Reports]]
- [[Module - app]] and the "Working on this module" section of each module involved
- [[Workflows]] (only the sections for this milestone), [[Frontend Architecture]], [[Design System]], [[Writing Style]]

## Build
Reports screen with the generic table, every report, printing on 80 mm and A4, Excel and PDF export.

- Commands stay thin, each workflow is one transaction, and every new command gets its permission test.
- Screens: flat, colour-coded, a tooltip on every button and field, all text in the module's `text/en.ts`.
- If a module crate is missing something, add it **in that module** (with its contract test), then use it here.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- One year of generated data reports in under 3 s.
- Integration tests in `crates/app/tests/` and Playwright flows for the new screens are green.
- Module pages and [[Workflows]] match the code.
- Pull request from `task/C-M11`, with the **Mac build** for the product owner to try (`build-mac` label).

