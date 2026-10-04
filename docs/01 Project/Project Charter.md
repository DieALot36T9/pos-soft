---
tags: [project]
---
# Project Charter

## Vision
A fast, reliable, **fully offline** point-of-sale system for supermarkets and grocery shops ("supershops"). It is **universal**: the shop that installs it chooses its own tax rates, currency, cash rounding, payment methods and staff permissions. Nothing is hard-coded for one country.

## Goals
1. A cashier can scan, take payment and print a receipt with **only the keyboard**, with each scan showing in under 100 ms.
2. **No sale is ever lost**: power cuts, crashes and full disks never corrupt or drop a completed sale.
3. The owner can run the whole shop from it: catalog, stock, suppliers, customers, loyalty, credit and reports.
4. The code stays **easy to understand and safe to change**: modules are independent, every rule is tested, and the docs in this vault are kept current.

## Success criteria for v1.0
- Every requirement in `02 Requirements/` passes its acceptance tests.
- The [[UAT Script]] (a full simulated trading day) runs on real hardware with zero data-loss bugs and zero blocking bugs.
- Performance budgets in [[Non-Functional Requirements]] are met with 50,000 products and 5 million sale lines.
- Signed installers exist for Windows. Linux and macOS installers build in CI.

## People and responsibilities
| Role | Who | Responsibilities |
|---|---|---|
| Product owner | You (the shop owner / project owner) | Decide scope, accept milestones, run UAT, buy test hardware, own the signing certificates |
| Developer, designer, tester | Claude | Design, code, tests, docs, CI, release builds |
| Admin users | Shop owner/manager at each installation | Setup, products, prices, stock, staff |
| Cashiers | Shop staff | Checkout, holds, own shift, customer lookup |

### Where the product owner must be involved (the developer can't do these)
- **M1 receipt check (on your Mac, no hardware needed):** look at the receipt pictures on screen and approve how they look.
- **By M13:** have a Windows 10 PC, a receipt printer with a cash drawer, a barcode scanner and (if you sell by weight) a scale, for the first real-hardware test and UAT. Until then, everything is built and tested with fake hardware.
- **Milestone demos:** try each milestone build and accept or reject it.
- **Code signing (later, once the product works):** buy a Windows code-signing certificate (or set up Azure Trusted Signing). If macOS builds are needed, set up an Apple Developer account.
- **UAT:** a real cashier runs the [[UAT Script]].
- **Pilot:** install at one till and report problems.

## Constraints
- **Offline only.** No feature may need internet access.
- **Main target:** Windows 10/11 64-bit, 16 GB RAM, with WebView2 installed (it ships with Windows 11 and recent Windows 10). Linux and macOS are built and tested in CI.
- **Testing machine for now:** the product owner's Mac. Every milestone gives a Mac build to try.
- **Language:** English only in v1.0, built so more languages can be added later.
- **Look:** flat and plain, colour-coded, with a tooltip on everything ([[Design System]]).
- **One till per installation** in v1.0. The data model is ready for several tills later ([[ADR-009 ULID ids and till identity]]).
- **One developer, who is an AI.** This is why the design relies on strict types, small modules, tests written first, and docs as the project's memory.

## Decisions taken with the product owner
| Question | Answer | Effect |
|---|---|---|
| Whose tax rules? | Universal: each shop enters its own rates | [[Req - Tax]], [[ADR-008 Tax model]] |
| Target computers | Windows 10, 16 GB RAM | [[ADR-017 Windows first all three OS built]] |
| Who develops? | Claude | Stack chosen for reliability of AI-written code: [[Tech Stack]] |
| Phasing | **Everything in your list is available from day one (v1.0)** | [[Scope]] has no Soon/Later split |
| Language | English only, ready for more later | [[ADR-022 Ready for more languages later]] |
| Testing machine | Your Mac for now; a Windows PC before UAT | [[ADR-017 Windows first all three OS built]] |
| Look and feel | Flat, colour-coded, tooltips on everything, nothing fancy | [[Design System]], [[ADR-021 Flat colour-coded UI with tooltips]] |
| Wording of docs and code | Plain and easy to understand | [[Writing Style]] |

## Out of scope for v1.0
See [[Scope#Out of scope]].
