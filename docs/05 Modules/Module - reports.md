---
tags: [module]
crate: reports
milestone: M11 (shift report in M6/M7)
---
# Module: reports

**Purpose:** read-only queries that turn the published views of other modules into reports. **Owns no tables. Reads only `*_v_*` views.**

**Requirements:** [[Req - Reports]], [[Req - Shifts and Cash]] SHF-08. **Depends on:** `core`, `rusqlite`, `thiserror`, `chrono` (clock times on the shift report, date sums). **Dev-dependencies (tests only, rule R1):** `audit`, `catalog`, `customers`, `inventory`, `purchasing`, `sales`, `shifts`, so the tests can build their views with their own migrations.

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

Every row (and the totals row) has exactly one cell per column, and each cell is either `Empty` or the column's kind. A `Date` column's totals row has an empty first cell (it can't hold the word "Total").

## Public API
Every function takes the **reader** connection (`&rusqlite::Connection`) and returns `Result<ReportTable, ReportsError>`. `ReportsError` turns into `AppError`: `Invalid("date_range_backwards")` → `Validation`, `ShiftNotFound` → `NotFound { entity: "shift" }`, database errors → `Internal`.
```rust
pub fn daily_summary(conn, date: BusinessDate, till: Option<&Id>, shop: &ShopContext) -> Result<ReportTable>;
pub fn shift_report(conn, shift: &Id, shop: &ShopContext) -> Result<ReportTable>;      // "X report – shift still open" / "Z report"
pub fn sales_by(conn, by: SalesDimension, range: DateRange, filters: &SalesFilters, shop: &ShopContext) -> Result<ReportTable>;
pub fn profit(conn, by: GroupBy, range: DateRange) -> Result<ReportTable>;
pub fn stock_value(conn, by: GroupBy) -> Result<ReportTable>;
pub fn low_stock(conn) -> Result<ReportTable>;
pub fn near_expiry(conn, today: BusinessDate, days: u32) -> Result<ReportTable>;
pub fn stock_movement(conn, range: DateRange) -> Result<ReportTable>;
pub fn tax_report(conn, range: DateRange) -> Result<ReportTable>;
pub fn payables_aging(conn, today: BusinessDate) -> Result<ReportTable>;
pub fn receivables_aging(conn, today: BusinessDate) -> Result<ReportTable>;
pub fn discounts_overrides(conn, range: DateRange, shop: &ShopContext) -> Result<ReportTable>;
pub fn migrations() -> ModuleMigrations;   // empty: reports own no tables

pub struct DateRange { pub from: BusinessDate, pub to: BusinessDate }   // both days included; to < from is refused
pub enum SalesDimension { Product, Category, Brand, Cashier, Hour, Weekday, Date }
pub enum GroupBy { Product, Category, Brand }
pub struct SalesFilters { pub category_id: Option<Id>, pub brand_id: Option<Id>, pub cashier_id: Option<Id>, pub top: Option<u32> }
pub struct ShopContext {                 // what app knows and reports don't own
  pub user_names: BTreeMap<Id, String>,            // from auth
  pub till_names: BTreeMap<Id, String>,            // from settings
  pub payment_method_names: BTreeMap<Id, String>,  // from sales (no view publishes them)
  pub utc_offset_minutes: i32,                     // for "Hour" and the shift's clock times
}
pub const ACTION_CART_CANCEL: &str = "cart.cancel";     // audit action names the reports count;
pub const ACTION_SALE_VOID: &str = "sale.void";         // app::audit_actions must use the same text
pub const ACTION_DRAWER_NO_SALE: &str = "drawer.no_sale";
```
A missing name shows the id (or, for a payment method, its kind in words, e.g. "Card"), so a report never fails over a name.

## Reports
| Function | Views read | Content |
|---|---|---|
| `daily_summary` (RPT-01) | sales_v_sales, sales_v_lines, sales_v_return_lines, sales_v_payments, audit_v_events (cancelled carts) | Main rows: Gross sales (count, before discounts), Promotion / Customer / Manual discounts, Returns (count, amount), Net sales (after discounts and returns, with tax), Tax, Rounding, Total taken, Voided sales (count, amount), Cancelled carts, Average sale, Items sold. Sections: Tax by rate, Payments by method (sales, refunds, net), Voided sales. The till filter is shown in `params.filters` |
| `shift_report` (RPT-02, SHF-05, SHF-08) | shifts_v_shifts, shifts_v_movements, sales_v_sales, sales_v_lines, sales_v_return_lines, sales_v_payments, audit_v_events | The same main rows for the shift. Sections: Shift (till, cashier, business date, opened, closed, status), Tax by rate, Payments by method, Cash (float, cash sales, refunds, void reversals, cash in, out, drops, customer and supplier payments, expected, counted, difference), Voided sales. A closed shift shows the saved expected / counted / difference; an open one adds up the cash so far and leaves counted empty |
| `sales_by` (RPT-03) | sales_v_lines, sales_v_return_lines, catalog_v_products, catalog_v_categories | Quantity, net sales (without tax), tax, discount, number of sales, share of total. Sorted by net sales (largest first); hour, weekday (Monday first) and date in time order. `top` keeps the first N rows; totals still cover all |
| `profit` (RPT-04) | sales_v_lines, sales_v_return_lines, catalog_v_products, catalog_v_categories | Net sales, cost (saved on each line; returns take off their own saved cost), gross profit, margin |
| `stock_value` (RPT-05, INV-08) | inventory_v_levels, catalog_v_products | On hand × the product's cost price. By product: on hand, cost, value. By category/brand: number of products, value. Products with no stock are left out |
| `low_stock` (RPT-05, INV-03) | inventory_v_levels, catalog_v_products, purchasing_v_grn_lines + purchasing_v_payables (supplier who last delivered it) | Products **below** the reorder level, biggest shortage first. Suggested order = the reorder quantity, or (if 0) enough to reach the level |
| `near_expiry` (RPT-05) | inventory_v_batches, catalog_v_products | Batches with stock expiring by `today + days`, and expired ones. Days left (negative = expired), on hand, value at batch cost, status |
| `stock_movement` (RPT-05) | inventory_v_ledger, catalog_v_products | Per product: opening, received (receive, opening, supplier return), sold (sale, sale void), returned, adjusted (adjust, count), closing |
| `tax_report` (RPT-06, TAX-09) | sales_v_lines, sales_v_return_lines | Per rate: net sales, tax collected, returns, tax refunded, net tax; then a row per component (TAX-03). Refunded tax is split over the components of the sale line it came from, same rule as `tax::split_components` |
| `payables_aging` / `receivables_aging` (RPT-07) | purchasing_v_ledger + purchasing_v_payables (names), customers_v_credit_ledger + customers_v_customers (names) | Current, 1–30, 31–60, 61–90, over 90 days, total. Credits pay off the oldest debts first; overpayment shows as a negative "current". Suppliers age from the due date (or business date if none), customers from the business date. Zero balances are left out |
| `discounts_overrides` (RPT-08) | sales_v_lines, audit_v_events | Per cashier: sales with a manual discount and the amount, price overrides, voids, cancelled carts, no-sale openings, overrides approved. Section "Overrides approved": cashier, action, approver, count |

## Rules
- Reports use the **reader** connection only, and read only `*_v_*` views.
- Voided sales are excluded from sales totals and listed separately. Their payments are left out of "Payments by method".
- Returns are subtracted on their own business date (not the original sale's date).
- Sale lines group by the category and brand saved on the line. Return lines have none saved, so they use the product's **current** category and brand.
- A return's rounding is worked out as refunds paid − return line amounts (there is no `sales_v_returns` view).
- Percentages (`share`, margin, rates) are `Bps`, rounded half away from zero. A negative part or a whole of 0 or less shows an empty cell, because `Cell::Percent(Bps)` can't be negative.
- All sums use checked `Money` maths; an overflow is an error, never a wrap.

## Tests
A fixture shop (`tests/support/`) with a scripted day: sales on two tills, a promotion, a manual discount, a price override with cash rounding, a void, a return the same day and one the next day, cash in/out/drop, a closed and an open shift, audit events, stock with batches and a ledger, supplier and customer ledgers. Every number below was worked out by hand from that script (the table is in `tests/support/mod.rs`). The fixture writes rows straight into the other modules' tables (test code only); the reports themselves read only the views.

| File | Tests |
|---|---|
| `tests/daily.rs` | `rpt_01_daily_summary_adds_up_the_day`, `rpt_01_tax_is_listed_by_rate`, `rpt_01_payments_by_method_are_sales_minus_refunds`, `rpt_01_voided_sales_are_listed_separately`, `rpt_01_one_till_only`, `rpt_01_a_day_without_sales_shows_zeros`, `rpt_01_returns_count_on_their_own_day` |
| `tests/shift.rs` | `shf_08_z_report_shows_the_shift_details`, `shf_08_z_report_counts_only_this_shift`, `shf_08_cash_summary_matches_the_count`, `shf_05_x_report_while_the_shift_is_open`, `rpt_02_unknown_shift_is_not_found` |
| `tests/sales_analysis.rs` | `rpt_03_sales_by_category`, `rpt_03_returns_count_on_their_own_business_date`, `rpt_03_sales_by_product_and_brand`, `rpt_03_sales_by_cashier`, `rpt_03_sales_by_hour_use_local_time`, `rpt_03_sales_by_weekday_and_date`, `rpt_03_filters_and_top_n`, `rpt_03_a_backwards_date_range_is_refused`, `rpt_04_profit_uses_the_cost_saved_on_each_line`, `rpt_04_profit_by_product_and_brand` |
| `tests/stock.rs` | `rpt_05_stock_value_by_product`, `rpt_05_stock_value_by_category_and_brand`, `rpt_05_low_stock_with_supplier_and_suggested_order`, `rpt_05_near_expiry_and_expired_batches`, `rpt_05_stock_movement_from_opening_to_closing` |
| `tests/money_owed.rs` | `tax_09_tax_report_by_rate_and_component`, `rpt_07_supplier_aging_pays_off_the_oldest_first`, `rpt_07_customer_aging`, `rpt_08_discounts_and_overrides_per_cashier` |
| `tests/contract.rs` | `every_report_has_one_cell_of_the_right_kind_per_column`, `every_report_runs_on_an_empty_database` (proves every view and column read exists in its owner's migrations), `column_keys_are_unique_in_each_table`, `reports_owns_no_tables` |
| Unit tests | `aging.rs`: bucket edges 0/30/60/90, oldest debts paid first, overpayment as negative current. `tax.rs`: TAX-03 split (towards zero, last part gets the rest, refunds). `stock.rs`: INV-03 suggested order. `table.rs`: share rounding and empty shares. `shift.rs`: local clock time. `sql.rs`: only named parameters are bound. `types.rs`, `error.rs` |

- Benchmark (RPT-11): every report on the reference database ≤ 3 s; daily summary ≤ 1 s. **Not built yet** (see Open items).

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/reports/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/reports/` |
| **Screen folder** | `ui/src/modules/reports/` |
| **Requirements** | [[Req - Reports]] RPT-01 to RPT-11; [[Req - Shifts and Cash]] SHF-08 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | The reader connection, the date range, filters, today's date, and a `ShopContext` (user, till and payment method names, local UTC offset) |
| **Used by** (re-run their tests when the public API changes) | Report commands, closing a shift (Z report), printing the daily summary |
| **Test it on its own** | `cargo nextest run -p reports` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Read only `*_v_*` views, never another module's tables.
- Use the reader connection only.
- If a view lacks a column you need, add it in the module that owns the view (with its contract test). Don't work around it here.
- Test fixtures may write other modules' tables directly (tests only). The report code itself must not; `contract::every_report_runs_on_an_empty_database` shows a missing view or column at once.
- Audit action names are matched as text (`ACTION_*`); keep them in step with `app::audit_actions`.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Built in task B-reports (Rust only). `app` commands, RPT-10 visibility checks (cashiers see only X/Z of their own shifts, everything else needs `ReportsViewAll`) and screens come in C-M7 / C-M11.
- **Count per denomination (SHF-08)** is not on the shift report: `shifts_counts` has no published view. Add `shifts_v_counts` in the shifts module (with its contract test), then a "Count" section here.
- **Benchmark (RPT-11)** not built: it needs the reference database from `xtask seed`, which doesn't exist yet. Add `benches/reports.rs` (criterion) once it does.
- **Audit action names:** `app::audit_actions` (C-M6) must write `cart.cancel`, `sale.void` and `drawer.no_sale` exactly as `ACTION_*` here, or the cancelled-cart, void and no-sale counts stay at 0. Price overrides are counted from `sales_v_lines.price_overridden`, so they need no action name.
- **Negative percentages:** `core::Cell::Percent(Bps)` can't hold a negative margin, so a loss shows an empty margin cell. If the product owner wants "−5.00%", `core` needs a signed percent cell.
- **Hours across a daylight-saving change** use one offset (`ShopContext::utc_offset_minutes`). Fine for Bangladesh (no DST); revisit for shops with DST.
- `sales_by` takes the category/brand filter for **returns** from the product's current catalog entry (return lines have no saved category). If a product changes category, its old returns move with it.
- RPT-07 supplier and customer **statements** are in `purchasing::statement` / `customers::statement`; `app` turns them into a `ReportTable` (or a later task adds a function here).
- RPT-09 export (XLSX, PDF) lives in `io` and the A4 print view, not here.
