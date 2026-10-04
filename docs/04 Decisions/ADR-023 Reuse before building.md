---
tags: [adr]
status: accepted
---
# ADR-023 Reuse before building

## Context
The product owner asked that nothing be reinvented: use existing solutions, open-source patterns and established standards. Hand-written versions of solved problems cost time, hide bugs, and are harder for a new person (or session) to understand than a well-known library.

## Decision
- **Check the [[Reuse Map]] before writing anything.** Prefer, in order: an established **standard** → a maintained, permissively licensed **library** → a well-known **pattern** from open-source POS/ERP projects → our own code.
- Our own code is allowed only when nothing fits. The reason is recorded in the Reuse Map.
- **Licences:** dependencies must be permissive (MIT, Apache-2.0, BSD, ISC, Zlib, Unicode, and OFL for fonts). `cargo-deny` enforces this. Code is never copied from GPL/LGPL/AGPL projects; ideas may be learned from them.
- Applied immediately to A0's code (task [[Task - A1 Reuse Alignment]]): `cargo-deny` replaces A0's hand-written module dependency check, `eslint-plugin-boundaries` replaces the custom ESLint rule, i18next replaces A0's own text hook, and `num-format` replaces hand-written digit grouping. The per-module migration runner stays (no library supports per-module versions).

## Consequences
- ✅ Less of our own code to test and maintain, and more of it is familiar to anyone who knows the standard tools.
- ✅ The licence check protects the product owner's options for licensing and selling the app.
- ⚠️ Some more dependencies. Each must pass the maintenance and licence bar on the Reuse Map.
