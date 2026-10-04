---
tags: [adr]
status: accepted
---
# ADR-007 Quantities in thousandths

## Context
The first plan used grams for weight. But products are sold by the piece, kg, litre, ml and pack, and units are configurable.

## Decision
`core::Qty(i64)` = quantity × 1000 in the **product's own unit**. 1 piece = 1000, 1.234 kg = 1234, 0.5 L = 500. Each unit has allowed decimals (0–3), and quantities are validated against it. A scale reading in grams is converted to the product's unit (kg) when it's read.

## Consequences
- ✅ One type for every unit and every calculation.
- ✅ Gram precision for kg products; ml precision for litre products.
- ⚠️ Products sold in grams as their unit (unit = g) can't have fractions of a gram. That's fine.
