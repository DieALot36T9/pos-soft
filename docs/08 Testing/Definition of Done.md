---
tags: [testing, process]
---
# Definition of Done

A change (feature, fix or refactor) is **done** only when every box is ticked:

- [ ] A failing test was written first, and it now passes
- [ ] Tests name the requirement IDs they cover
- [ ] All CI gates pass on Windows, Linux and macOS ([[Test Strategy#CI gates (merge is blocked if any fail)]])
- [ ] No new warnings (clippy, tsc, eslint)
- [ ] [[Module Rules]] checklist passes (no cross-module imports or table access)
- [ ] Generated TS types are up to date
- [ ] New screen text is in the module's `text/en.ts`, in plain English, with a tooltip for every new button and field
- [ ] Code comments and docs follow [[Writing Style]]
- [ ] Permissions decided for any new command (and in the permissions test)
- [ ] Audit events added for any new sensitive action
- [ ] **The module page in `05 Modules/` is updated** (API, tables, tests, and its "Working on this module" section)
- [ ] Requirements updated if behaviour changed. ADR written if a hard-to-reverse decision was made
- [ ] `CHANGELOG.md` updated under *Unreleased*
- [ ] For UI changes: checked with the keyboard alone, at 1366×768, and against the [[Design System]] (colour meanings, no shadows or animations)
- [ ] For printing changes: golden images re-approved by the product owner
- [ ] For migrations: tested on an empty database **and** on the previous release's database

## Milestone done
- [ ] Every story for the milestone passes
- [ ] Demo build shared and **accepted by the product owner**
- [ ] Milestone note written (`Templates/Template - Milestone`)
