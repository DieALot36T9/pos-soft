---
tags: [module]
crate: reports
milestone: M11 (shift report in M6/M7)
---
# Module: reports

**Purpose:** read-only queries that turn the published views of other modules into reports. **Owns no tables. Reads only `*_v_*` views.**

**Requirements:** [[Req - Reports]], [[Req - Shifts and Cash]] SHF-08. **Depends on:** `core`, `rusqlite`.

## One output shape for every report
`ReportTable` is defined in **`core`** (not here), because `io` and `printing` also use it and modules can't depend on each other.
```rust
pub struct ReportTable {
  pub title: String,                     // plain English title, e.g. "Daily summary"
  pub params: ReportParams,              // date range, filters
  pub columns: Vec<Column>,              // key, title, kind: Text | Money | Qty | Int | Percent | Date
  pub rows: Vec<Vec<Cell>>,
  pub totals: Option<Vec<Cell>>,
  pub sections: Vec<ReportTable>,        // e.g. Daily summary has sections (payments, tax, …)
}
```
The UI shows it with one generic table component. `io` exports it to XLSX. `printing` lays it out for 80 mm. The A4 print view renders it as HTML. **Adding a report = one query function plus one menu entry.**

## Reports
| Function | Views read |
|---|---|
| `daily_summary(conn, date, till)` | sales_v_sales, sales_v_lines, sales_v_return_lines, sales_v_payments, audit_v_events (cancelled carts) |
| `shift_report(conn, shift)` | shifts_v_shifts, shifts_v_movements, sales_v_sales, sales_v_lines, sales_v_payments |
| `sales_by(conn, dim, range, filters)` (product, category, brand, cashier, hour, weekday, date) | sales_v_lines, sales_v_return_lines, catalog_v_products, catalog_v_categories |
| `profit(conn, dim, range)` | sales_v_lines, sales_v_return_lines |
| `stock_value(conn, group_by)` | inventory_v_levels, catalog_v_products |
| `low_stock(conn)` | inventory_v_levels, catalog_v_products, purchasing_v_payables (supplier names) |
| `near_expiry(conn, today, days)` | inventory_v_batches, catalog_v_products |
| `stock_movement(conn, range)` | inventory_v_ledger |
| `tax_report(conn, range)` | sales_v_lines, sales_v_return_lines |
| `payables_aging(conn, today)` / `receivables_aging(conn, today)` | purchasing_v_ledger, customers_v_credit_ledger |
| `discounts_overrides(conn, range)` | sales_v_lines, audit_v_events |

## Rules
- Reports use the **reader** connection only.
- Voided sales are excluded from sales totals and listed separately.
- Returns are subtracted on their own business date (not the original sale's date).

## Tests
- A fixture shop with a scripted day (sales, returns, a void, discounts, cash in/out); every report's numbers are checked against hand-calculated values.
- Benchmark: every report on the reference database ≤ 3 s; daily summary ≤ 1 s.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/reports/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/reports/` |
| **Screen folder** | `ui/src/modules/reports/` |
| **Requirements** | [[Req - Reports]] RPT-01 to RPT-11; [[Req - Shifts and Cash]] SHF-08 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | The date range, filters and today's date |
| **Used by** (re-run their tests when the public API changes) | Report commands, closing a shift (Z report), printing the daily summary |
| **Test it on its own** | `cargo nextest run -p reports` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Read only `*_v_*` views, never another module's tables.
- Use the reader connection only.
- If a view lacks a column you need, add it in the module that owns the view (with its contract test). Don't work around it here.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.
