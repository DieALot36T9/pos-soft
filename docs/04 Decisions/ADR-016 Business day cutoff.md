---
tags: [adr]
status: accepted
---
# ADR-016 Business day cutoff

## Context
Shops trade past midnight. A sale at 00:30 belongs to the previous trading day for reports.

## Decision
- Setting `business_day_cutoff_hour` (0–23, default 4).
- `business_date = local_date(now − cutoff hours)`, calculated only in `core::business_date`.
- Shifts get their business date when opened. Sales use the **shift's** business date, so a shift that crosses the cutoff stays in one business day.

## Consequences
- ✅ Daily reports match how shop owners think about their day.
- ⚠️ Time-zone changes (DST) are handled by using the local offset at the time of the event.
