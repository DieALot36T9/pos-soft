---
tags: [task]
id: C-M12
model: claude-opus-5-5
effort: medium
phase: C
depends_on: [C-M11, B-io]
status: done
---
# Task C-M12: Import/export, labels and shelf tags

**Model: Opus 5.5 · Effort: medium**

**Start in a fresh session**, only after the tasks it depends on are merged: C-M11, B-io.
This task connects finished module crates into the app (`crates/app/`) and builds their screens.

## Read first
- [[Reuse Map]]: use the listed libraries (i18next, react-hotkeys-hook, Intl.NumberFormat, tauri plugins, …)
- This milestone's row in [[Roadmap]]
- Requirements: [[Req - Import and Export]], [[Req - Catalog]] (CAT-12, CAT-13)
- [[Module - app]] and the "Working on this module" section of each module involved
- [[Workflows]] (only the sections for this milestone), [[Frontend Architecture]], [[Design System]], [[Writing Style]]

## Build
Import wizard for every entity (with backup first), exports, templates, barcode labels and shelf tags (A4 and label printer, preview first).

- Commands stay thin, each workflow is one transaction, and every new command gets its permission test.
- Screens: flat, colour-coded, a tooltip on every button and field, all text in the module's `text/en.ts`.
- If a module crate is missing something, add it **in that module** (with its contract test), then use it here.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- 50k products import in under 60 s; an export imports back unchanged.
- Integration tests in `crates/app/tests/` and Playwright flows for the new screens are green.
- Module pages and [[Workflows]] match the code.
- Pull request from `task/C-M12`, with the **Mac build** for the product owner to try (`build-mac` label).

