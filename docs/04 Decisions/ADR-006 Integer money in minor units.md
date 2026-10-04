---
tags: [adr]
status: accepted
---
# ADR-006 Integer money in minor units

## Context
Floating-point numbers can't represent most decimal amounts exactly. The app is universal, so currencies can have 0 (JPY), 2 (BDT, USD) or 3 (KWD) decimals.

## Decision
- `core::Money(i64)` holds **minor units** of the shop's single currency.
- The number of decimals is a **setting chosen at setup** and locked after the first sale.
- Arithmetic uses `i128` intermediates and the rounding helpers in `core`. Floats are banned for money by clippy.
- Display and parsing (`"12.50"` → 1250) happen only in `core::format`/`core::parse` and the UI `formatMoney`.

## Consequences
- ✅ Exact maths. Totals always add up.
- ✅ Any currency works.
- ⚠️ A single currency per database. Multi-currency is out of scope.
