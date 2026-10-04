---
tags: [adr]
status: accepted
---
# ADR-011 Snapshots on sale lines

## Context
Product names, prices, tax rates and costs change. Old receipts, reprints, returns and profit reports must show what actually happened.

## Decision
Completed documents store copies of everything they used: product name (both languages), SKU, unit symbol, unit price, discounts and the promotion id/name, tax rate id/name/bps and component split, net/tax/gross, **unit cost at the time of sale**, and the customer's name and phone.

## Consequences
- ✅ Reprints are identical to the original. Profit is exact. Returns refund the right amounts.
- ✅ Reports don't need to join catalog tables for history.
- ⚠️ Rows are larger (~150 bytes per line). That's acceptable.
