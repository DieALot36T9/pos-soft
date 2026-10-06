---
tags: [task]
id: C-M4
model: claude-opus-5-5
effort: medium
phase: C
depends_on: [C-M3, B-catalog, B-barcode]
status: done
---
# Task C-M4: Products

**Model: Opus 5.5 · Effort: medium**

**Start in a fresh session**, only after the tasks it depends on are merged: C-M3, B-catalog, B-barcode.
This task connects finished module crates into the app (`crates/app/`) and builds their screens.

## Read first
- [[Reuse Map]]: use the listed libraries (i18next, react-hotkeys-hook, Intl.NumberFormat, tauri plugins, …)
- This milestone's row in [[Roadmap]]
- Requirements: [[Req - Catalog]]
- [[Module - app]] and the "Working on this module" section of each module involved
- [[Workflows]] (only the sections for this milestone), [[Frontend Architecture]], [[Design System]], [[Writing Style]]

## Build
Product save workflow (catalog + reorder rule + price history + audit), product list and edit screens (all tabs), categories, brands, units, barcode generation, variants, kits, images.

- Commands stay thin, each workflow is one transaction, and every new command gets its permission test.
- Screens: flat, colour-coded, a tooltip on every button and field, all text in the module's `text/en.ts`.
- If a module crate is missing something, add it **in that module** (with its contract test), then use it here.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- Search under 150 ms on 50k generated products; all product screens work by keyboard.
- Integration tests in `crates/app/tests/` and Playwright flows for the new screens are green.
- Module pages and [[Workflows]] match the code.
- Pull request from `task/C-M4`, with the **Mac build** for the product owner to try (`build-mac` label).

