---
tags: [adr]
status: accepted
---
# ADR-010 Append-only ledgers

## Context
Stock history, loyalty points, customer credit and supplier payables all need a full history and must never silently change.

## Decision
- Each is a **ledger table** of signed changes: `inventory_ledger`, `customers_points_ledger`, `customers_credit_ledger`, `purchasing_ledger`, plus `audit_events` and `shifts_movements`.
- Database triggers abort any UPDATE or DELETE on them.
- Current balances are kept in **cache tables** (`inventory_levels`, `customers_balances`, …) updated in the **same transaction** as each ledger insert.
- Corrections are new rows (reversals), never edits.
- A check confirms that each cache equals the sum of its ledger (tests + an admin "Verify data" action).

## Consequences
- ✅ Complete history and audit trail for free; stock history is just a query.
- ✅ Balances can be rebuilt from the ledger if a cache is ever wrong.
- ⚠️ Every change writes two rows. That's trivial cost.
