---
tags: [home]
---
# Supershop POS (working name)

An **offline, universal point-of-sale system for supermarkets and grocery shops**. Any shop in any country can set its own tax rates, currency, rounding, payment methods and staff roles. It runs on Windows (the shop target), macOS (where it's tested for now) and Linux. The screens are flat, colour-coded and explained by tooltips. Everything (screens, docs and code) is written in plain English.

> **Status:** Phase 2 (Design) is complete in this vault. Milestone M0 (Foundations, task A0) is built and in review. Next up are the B tasks. See [[Roadmap]] and [[Task Index]].

---

## Start here
| If you want to… | Read |
|---|---|
| Understand what we're building and why | [[Project Charter]] → [[Scope]] |
| See what changed from the first plan | [[Changes From First Plan]] |
| Know the order of work | [[Roadmap]] |
| Understand the system design | [[System Overview]] → [[Module Map]] → [[Module Rules]] |
| See why a decision was made | [[ADR Index]] |
| Know exactly what a feature must do | [[Requirements Index]] |
| Work on a module | `05 Modules/` → e.g. [[Module - sales]] |
| See the screens | [[Screen Map]] |
| Write or run tests | [[Test Strategy]] → [[Definition of Done]] |
| Build the app with Claude cloud sessions | [[Building with Cloud Sessions]] → [[Task Index]] |
| Set up a computer for development | [[Dev Environment Setup]] |
| Write docs, code comments or screen text | [[Writing Style]] |
| See the colours, buttons and tooltips | [[Design System]] |
| Install, back up or fix a till | `10 Runbooks/` |

## Vault map
- **01 Project**: [[Project Charter]], [[Scope]], [[Roadmap]], [[Risks]], [[Glossary]], [[Changes From First Plan]]
- **02 Requirements**: [[Requirements Index]], [[Non-Functional Requirements]], [[Roles and Permissions]]
- **03 Architecture**: [[System Overview]], [[Tech Stack]], [[Module Map]], [[Module Rules]], [[Workflows]], [[Data Conventions]], [[Money Tax and Rounding]], [[Database Strategy]], [[Frontend Architecture]], [[Security Model]], [[Errors and Logging]], [[Repository Layout]]
- **04 Decisions**: [[ADR Index]]
- **05 Modules**: one page per module (19 modules)
- **06 Database**: [[Database Overview]]
- **07 UI Design**: [[Screen Map]], [[Checkout Screen]], [[Setup Wizard]], [[Back Office Screens]], [[Keyboard Shortcuts]], [[Design System]]
- **08 Testing**: [[Test Strategy]], [[Definition of Done]], [[Test Data]], [[UAT Script]]
- **09 Guides**: [[Building with Cloud Sessions]], [[Writing Style]], [[Coding Standards]], [[Dev Environment Setup]], [[Adding a Module]], [[TDD Workflow]], [[Release Process]]
- **10 Runbooks**: [[Installation]], [[Backup and Restore]], [[Printer Setup]], [[Scale Setup]], [[Troubleshooting]]
- **11 User Manual**: [[Cashier Guide]], [[Admin Guide]]
- **Templates**: module page, ADR, user story, milestone, bug report
- **12 Tasks**: [[Task Index]]: one brief per piece of work, for fresh cloud sessions
- [[CHANGELOG]]

## Vault rules
1. **A change to a module updates its page in `05 Modules/`** in the same change. This is part of the [[Definition of Done]].
2. A decision that is hard to reverse gets an ADR (use `Templates/Template - ADR`).
3. Requirements use stable IDs (e.g. `CHK-08`). Tests reference these IDs in their names or comments.
4. Diagrams are written in Mermaid so they live as text and can be reviewed in diffs.
