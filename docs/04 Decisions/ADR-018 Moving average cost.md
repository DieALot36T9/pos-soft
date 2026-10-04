---
tags: [adr]
status: accepted
---
# ADR-018 Moving average cost

## Context
"Receive goods: cost price updates." The method affects profit reports and stock value.

## Decision
- Default: **moving average** cost, updated on every GRN ([[Money Tax and Rounding#Moving average cost (receiving)]]).
- Setting: **last purchase cost** as an alternative.
- Sale lines store the unit cost at the time of sale ([[ADR-011 Snapshots on sale lines]]).
- Batches store their own receiving cost for information, but sales use the product's cost (no per-batch costing).

## Consequences
- ✅ Simple, standard for retail, and smooths out price swings.
- ⚠️ No FIFO costing. Acceptable for a grocery POS (not an accounting system).
