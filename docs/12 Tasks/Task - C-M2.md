---
tags: [task]
id: C-M2
effort: medium
phase: C
depends_on: [C-M1, B-settings, B-tax, B-auth, B-db]
status: waiting
---
# Task C-M2: Settings, setup wizard and backups

**Suggested effort: medium**

**Start in a fresh session**, only after the tasks it depends on are merged: C-M1, B-settings, B-tax, B-auth, B-db.
This task connects finished module crates into the app (`crates/app/`) and builds their screens.

## Read first
- This milestone's row in [[Roadmap]]
- Requirements: [[Req - Setup and Settings]], [[Req - Backup and Logging]], [[Req - Tax]]
- [[Module - app]] and the "Working on this module" section of each module involved
- [[Workflows]] (only the sections for this milestone), [[Frontend Architecture]], [[Design System]], [[Writing Style]]

## Build
Startup sequence, the [[Setup Wizard]] (incl. restore-from-backup), Settings screens, tax-rate screens, backup timer and on-close backup, Backups screen with restore, the startup integrity check, the logging switch.

- Commands stay thin, each workflow is one transaction, and every new command gets its permission test.
- Screens: flat, colour-coded, a tooltip on every button and field, all text in the module's `text/en.ts`.
- If a module crate is missing something, add it **in that module** (with its contract test), then use it here.

## Done when
- Anything not finished is listed under **"Open items"** on the module page, so a later session can find it without any hand-over.
- A fresh install goes through the wizard; backups appear on a timer and on close; restore works.
- Integration tests in `crates/app/tests/` and Playwright flows for the new screens are green.
- Module pages and [[Workflows]] match the code.
- Pull request from `task/C-M2`, with the **Mac build** for the product owner to try (`build-mac` label).

