---
tags: [adr]
status: accepted
---
# ADR-008 Tax model

## Context
The app is universal. Each shop sets its own tax. Countries differ: inclusive vs exclusive prices, multiple rates, split display (CGST/SGST), and sometimes compound taxes.

## Decision
- Shop-wide **mode**: prices include tax, or prices exclude tax.
- **Named rates** in basis points (0–10000). **Each product has exactly one rate.**
- Optional **components** of a rate (for display and reporting only). Components must add up to the rate.
- Tax is calculated **per line** after all discounts, rounded half away from zero ([[Money Tax and Rounding]]).
- Sale lines store `tax_rate_id`, name, bps and component split as snapshots.
- Compound taxes (tax on tax) are **not supported** in v1.0.

## Consequences
- ✅ Covers VAT (BD, EU, UK), GST (IN, AU), sales tax (US, flat per shop), and zero-rated/exempt goods.
- ⚠️ Per-line rounding can differ by a minor unit from "round the invoice total" rules. This is documented, and it's the more common POS practice.
- ⚠️ US-style stacked city/state rates must be entered as one combined rate.
- ⚠️ Compound taxes need a future ADR.
