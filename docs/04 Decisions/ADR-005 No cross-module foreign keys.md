---
tags: [adr]
status: accepted
---
# ADR-005 No cross-module foreign keys

## Context
Sales lines refer to products, ledgers refer to users, and so on. SQL foreign keys across modules would make migration order matter and tie one module's schema to another's.

## Decision
- Foreign keys are used **within** a module (e.g. `sales_lines.sale_id → sales_sales.id`).
- References **to another module's rows** are plain `TEXT` ids with no SQL constraint.
- Workflows in `app` check that referenced rows exist before writing.
- Rows that are referenced elsewhere are never deleted, only archived ([[Data Conventions]] rule 5), so references can't dangle.

## Consequences
- ✅ Each module's migrations run on their own.
- ✅ Modules can be tested with only their own tables.
- ⚠️ The database can't enforce integrity across modules. Mitigated by workflow checks, archive-only, and a `xtask verify-refs` check run in tests and available as an admin "Verify data" action.
