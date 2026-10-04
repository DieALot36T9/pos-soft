---
tags: [task]
id: C-M1
model: claude-opus-5-5
effort: medium
phase: C
depends_on: [B-printing, B-hardware, B-barcode]
status: waiting
---
# Task C-M1: Receipt pictures and fake hardware

**Model: Opus 5.5 · Effort: medium**

**Start in a fresh session**, only after the tasks it depends on are merged: B-printing, B-hardware, B-barcode.
This task connects finished module crates into the app (`crates/app/`) and builds their screens.

## Read first
- This milestone's row in [[Roadmap]]
- Requirements: [[Req - Hardware and Printing]] (HW-02 to HW-04, HW-10)
- [[Module - app]] and the "Working on this module" section of each module involved
- [[Workflows]] (only the sections for this milestone), [[Frontend Architecture]], [[Design System]], [[Writing Style]]

## Build
Wire `printing` and `hardware` into `app`: the **Preview printer** window, "Test print", the dev-only **scan simulator** and **fake scale** panel, and printing through the system print dialog (Save as PDF on Mac). Produce the first golden receipt pictures.

- Commands stay thin, each workflow is one transaction, and every new command gets its permission test.
- Screens: flat, colour-coded, a tooltip on every button and field, all text in the module's `text/en.ts`.
- If a module crate is missing something, add it **in that module** (with its contract test), then use it here.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- The product owner approves the receipt pictures on screen. Mac build attached to the pull request.
- Integration tests in `crates/app/tests/` and Playwright flows for the new screens are green.
- Module pages and [[Workflows]] match the code.
- Pull request from `task/C-M1`, with the **Mac build** for the product owner to try (`build-mac` label).

