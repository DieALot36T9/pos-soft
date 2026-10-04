---
tags: [adr]
status: accepted
---
# ADR-020 Negative stock allowed by default

## Context
In real shops, stock data is often wrong (goods not received in the system yet). Blocking a sale at the till because the system thinks stock is 0 upsets customers.

## Decision
- Setting `allow_negative_stock`, **default on**. Sales show a non-blocking warning, and stock can go below zero.
- With batches, a sale that runs past all batches records the remainder with no batch.
- Negative stock is highlighted in stock reports and on the dashboard so it gets fixed (receive the goods or adjust).

## Consequences
- ✅ The till never stops because of data problems.
- ⚠️ Stock value can be temporarily understated. The negative-stock report makes this visible.
