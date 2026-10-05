---
tags: [module]
crate: purchasing
milestone: M10
---
# Module: purchasing

**Purpose:** suppliers, purchase orders, goods received (GRN), supplier returns, and what the shop owes suppliers.

**Requirements:** [[Req - Purchasing]]. **Depends on:** `core`, `rusqlite`, `thiserror`, `chrono` (calendar dates for due dates, aging and statements).

## Tables
Migration `001_init.sql`. Besides the tables below it adds indexes on `purchasing_order_lines(order_id)` and `purchasing_grn_lines(grn_id)`, and two triggers (`purchasing_ledger_no_update`, `purchasing_ledger_no_delete`) that abort any UPDATE or DELETE on the ledger.
```sql
CREATE TABLE purchasing_suppliers (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, contact_person TEXT, phone TEXT, email TEXT, address TEXT,
  tax_number TEXT, payment_terms_days INTEGER NOT NULL DEFAULT 0, notes TEXT,
  archived_at INTEGER, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE purchasing_orders (
  id TEXT PRIMARY KEY, number TEXT NOT NULL UNIQUE, supplier_id TEXT NOT NULL REFERENCES purchasing_suppliers(id),
  status TEXT NOT NULL CHECK (status IN ('draft','sent','partial','received','cancelled')),
  expected_date TEXT, notes TEXT, created_at INTEGER NOT NULL, created_by TEXT NOT NULL, sent_at INTEGER
);
CREATE TABLE purchasing_order_lines (
  id TEXT PRIMARY KEY, order_id TEXT NOT NULL REFERENCES purchasing_orders(id),
  product_id TEXT NOT NULL, product_name TEXT NOT NULL, sku TEXT NOT NULL,
  qty_milli INTEGER NOT NULL, unit_cost_minor INTEGER NOT NULL, received_milli INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE purchasing_grns (
  id TEXT PRIMARY KEY, number TEXT NOT NULL UNIQUE, supplier_id TEXT NOT NULL REFERENCES purchasing_suppliers(id),
  order_id TEXT REFERENCES purchasing_orders(id), supplier_invoice_no TEXT,
  invoice_total_minor INTEGER NOT NULL, lines_total_minor INTEGER NOT NULL,
  received_at INTEGER NOT NULL, business_date TEXT NOT NULL, user_id TEXT NOT NULL, notes TEXT
);
CREATE TABLE purchasing_grn_lines (
  id TEXT PRIMARY KEY, grn_id TEXT NOT NULL REFERENCES purchasing_grns(id), order_line_id TEXT REFERENCES purchasing_order_lines(id),
  product_id TEXT NOT NULL, product_name TEXT NOT NULL, qty_milli INTEGER NOT NULL, unit_cost_minor INTEGER NOT NULL,
  batch_no TEXT, expiry_date TEXT, inventory_batch_id TEXT
);
CREATE TABLE purchasing_returns (
  id TEXT PRIMARY KEY, number TEXT NOT NULL UNIQUE, supplier_id TEXT NOT NULL REFERENCES purchasing_suppliers(id),
  grn_id TEXT REFERENCES purchasing_grns(id), total_minor INTEGER NOT NULL, reason TEXT,
  at INTEGER NOT NULL, business_date TEXT NOT NULL, user_id TEXT NOT NULL
);
CREATE TABLE purchasing_return_lines (
  id TEXT PRIMARY KEY, return_id TEXT NOT NULL REFERENCES purchasing_returns(id),
  product_id TEXT NOT NULL, product_name TEXT NOT NULL, qty_milli INTEGER NOT NULL, unit_cost_minor INTEGER NOT NULL, inventory_batch_id TEXT
);
CREATE TABLE purchasing_ledger (                 -- append-only; + = shop owes supplier more
  id TEXT PRIMARY KEY, supplier_id TEXT NOT NULL REFERENCES purchasing_suppliers(id),
  amount_minor INTEGER NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('invoice','payment','return_credit','adjust')),
  method_kind TEXT, reference TEXT, ref_type TEXT, ref_id TEXT,
  due_date TEXT, at INTEGER NOT NULL, business_date TEXT NOT NULL, user_id TEXT NOT NULL
);
CREATE INDEX purchasing_ledger_supplier ON purchasing_ledger(supplier_id, at);
CREATE TABLE purchasing_balances (supplier_id TEXT PRIMARY KEY, balance_minor INTEGER NOT NULL);
CREATE TABLE purchasing_sequences (doc TEXT PRIMARY KEY, next INTEGER NOT NULL);   -- po, grn, sr
CREATE VIEW purchasing_v_payables AS
  SELECT s.id AS supplier_id, s.name, s.payment_terms_days, COALESCE(b.balance_minor, 0) AS balance_minor
  FROM purchasing_suppliers s LEFT JOIN purchasing_balances b ON b.supplier_id = s.id;
CREATE VIEW purchasing_v_ledger AS SELECT * FROM purchasing_ledger;
CREATE VIEW purchasing_v_grn_lines AS
  SELECT gl.*, g.number, g.supplier_id, g.received_at, g.business_date FROM purchasing_grn_lines gl JOIN purchasing_grns g ON g.id = gl.grn_id;
```

## Public API
Changes take `&Transaction`, reads take `&Connection`. Every function returns `Result<_, PurchasingError>`.
```rust
pub struct PurchasingCtx { pub at: Timestamp, pub business_date: BusinessDate, pub user_id: Id }   // who and when

// PUR-01 suppliers
pub fn create_supplier(tx, &SupplierInput, now: Timestamp) -> Result<Id>;
pub fn update_supplier(tx, &Id, &SupplierInput, now) -> Result<()>;
pub fn archive_supplier(tx, &Id, now) -> Result<()>;            // archived: no new orders or GRNs
pub fn list_suppliers(conn, include_archived: bool) -> Result<Vec<Supplier>>;   // by name
pub fn get_supplier(conn, &Id) -> Result<Supplier>;

// PUR-02 purchase orders (status: Draft → Sent → Partial → Received, or Cancelled)
pub fn create_order(tx, &NewOrder, &PurchasingCtx) -> Result<OrderRef>;   // draft, number PO-000001
pub fn update_order(tx, &Id, &NewOrder) -> Result<()>;   // draft only; replaces supplier, date, notes, lines
pub fn mark_sent(tx, &Id, now) -> Result<()>;            // draft → sent; again later: no change
pub fn cancel_order(tx, &Id) -> Result<()>;              // only before anything is received
pub fn get_order(conn, &Id) -> Result<Order>;            // with lines and received quantities
pub fn order_for_receiving(conn, &Id) -> Result<Vec<OrderLineRemaining>>;   // remaining never below 0
pub fn suggest_order_lines(supplier: &Id, low_stock: &[LowStockItem]) -> Vec<NewOrderLine>;   // pure

// PUR-03 receive goods
pub fn record_grn(tx, &NewGrn, &PurchasingCtx) -> Result<GrnRef>;
//   lines carry inventory_batch_id set by the workflow. Number GRN-000001. Adds each line to its
//   order line, sets the order to Partial or Received, and adds an `invoice` ledger row of
//   invoice_total, due business_date + payment_terms_days.
//   GrnRef { id, number, lines_total, invoice_total, total_differs, over_received: Vec<order line id>, order_status }

// PUR-04 supplier returns
pub fn record_return(tx, &NewSupplierReturn, &PurchasingCtx) -> Result<ReturnRef>;   // SR-000001; `return_credit` row of −total

// PUR-05 payables
pub fn record_payment(tx, supplier: &Id, amount: Money, method_kind: &str, reference: Option<&str>, &PurchasingCtx) -> Result<Id>;
pub fn record_adjustment(tx, supplier: &Id, amount: Money /* signed, not 0 */, reason: &str, &PurchasingCtx) -> Result<Id>;
pub fn balance(conn, &Id) -> Result<Money>;
pub fn unbalanced_suppliers(conn) -> Result<Vec<Id>>;    // cache ≠ ledger sum (ADR-010 "Verify data")
pub fn aging(conn, today: NaiveDate) -> Result<Vec<AgingRow>>;   // suppliers with a non-zero balance

// PUR-06
pub fn statement(conn, &Id, from: NaiveDate, to: NaiveDate) -> Result<Statement>;   // both days included

pub fn migrations() -> ModuleMigrations;
```
Types: `SupplierInput`, `Supplier`, `NewOrder`, `NewOrderLine`, `OrderRef`, `Order`, `OrderLine`, `OrderStatus`, `OrderLineRemaining`, `LowStockItem`, `NewGrn`, `NewGrnLine`, `GrnRef`, `NewSupplierReturn`, `NewReturnLine`, `ReturnRef`, `LedgerKind`, `AgingRow`, `Statement`, `StatementLine`.

**Errors.** `PurchasingError` turns into `AppError`:
| Variant | `AppError` | Codes |
|---|---|---|
| `Invalid { field, code }` | `Validation` | `supplier_name_missing`, `lines_missing`, `qty_not_positive`, `cost_negative`, `invoice_total_negative`, `order_missing`, `order_other_supplier`, `order_line_not_on_order`, `grn_other_supplier`, `amount_not_positive`, `amount_zero`, `method_missing`, `reason_missing`, `date_range_backwards` |
| `NotFound { entity, key }` | `NotFound` | entities `supplier`, `order`, `grn` |
| `Conflict { code }` | `Conflict` | `supplier_archived`, `order_not_draft`, `order_cancelled`, `order_has_receipts`, `order_closed` |
| `Core`, `Database` | the core error / `Internal` | |

## Rules in the code
- **Ledger signs:** invoice +, payment −, return credit −, adjustment either way. The cached balance in `purchasing_balances` is updated in the same transaction as each ledger row.
- **Due dates:** an invoice is due `payment_terms_days` after the GRN's business date. A positive adjustment is due on its own business date. Rows without a due date count from their business date.
- **Aging:** payments, returns and negative adjustments pay off the oldest amounts first (by due date). Buckets count days past due: current (0 or less), 1–30, 31–60, 61–90, 90+. A credit balance shows as a negative "current".
- **Totals:** a line's value is `core::mul_qty(unit_cost, qty)` (rounded per line). The GRN's `lines_total` is their sum. `total_differs` is a warning only: the ledger uses `invoice_total`.
- **Over-receiving** is allowed. The line ends up with more received than ordered and is listed in `over_received`. A GRN can't fill a Cancelled or Received order (`order_closed`).
- **Archived suppliers** get no new orders or GRNs (`supplier_archived`). Returns, payments, adjustments and statements still work.
- **Statement** rows are sorted by business date, then time. Opening = sum of rows before `from`.

## Tests
Each test name starts with the requirement (`pur_02_…`) or the [[Module Rules]] rule (`r2_…`) it checks. Every test builds its own in-memory database from this module's migrations (`tests/common/mod.rs`, `test_db()`).

| File | Tests |
|---|---|
| `tests/suppliers.rs` | `pur_01_create_supplier_keeps_every_field`, `pur_01_supplier_name_is_required`, `pur_01_update_supplier_changes_its_fields`, `pur_01_update_of_unknown_supplier_is_not_found`, `pur_01_archive_hides_the_supplier_but_keeps_it`, `pur_01_suppliers_are_listed_by_name`, `pur_01_archived_supplier_gets_no_new_orders` |
| `tests/orders.rs` | `pur_02_new_order_is_a_draft_with_a_po_number`, `pur_02_po_numbers_are_gapless_after_a_rollback`, `pur_02_order_needs_lines_with_a_positive_quantity`, `pur_02_draft_order_can_be_changed_but_a_sent_one_cannot`, `pur_02_send_marks_the_order_sent`, `pur_02_cancel_works_until_goods_arrive`, `pur_02_suggest_from_low_stock_uses_this_suppliers_products`, **`pur_02_status_goes_partial_then_received`**, **`pur_02_over_receiving_is_allowed_but_flagged`** |
| `tests/order_receiving.rs` | `pur_03_receiving_form_is_prefilled_with_remaining_quantities`, `pur_03_cancelled_or_received_orders_cannot_be_received_again`, `pur_03_grn_lines_must_belong_to_the_order`, `pur_03_order_must_be_from_the_same_supplier`, `pur_03_order_line_without_an_order_is_refused` |
| `tests/receiving.rs` | `pur_03_grn_without_an_order_gets_a_number_and_totals`, `pur_03_invoice_total_different_from_lines_is_flagged`, `pur_03_grn_raises_the_balance_by_the_invoice_total_with_a_due_date`, `pur_03_grn_keeps_batch_expiry_and_inventory_batch`, `pur_03_grn_checks_its_lines`, `pur_04_supplier_return_lowers_the_balance_by_its_value`, `pur_04_return_against_a_grn_must_be_the_same_supplier`, `pur_04_return_needs_lines_with_a_positive_quantity` |
| `tests/payables.rs` | **`pur_05_balance_after_invoice_payment_and_return`**, **`pur_05_ledger_is_append_only`**, `pur_05_payment_must_be_positive_and_have_a_method`, `pur_05_adjustment_moves_the_balance_either_way`, **`pur_05_aging_buckets_with_a_fixed_clock`**, `pur_06_statement_has_opening_and_closing_balances`, `pur_06_statement_refuses_a_backwards_date_range` |
| `tests/contract.rs` | `r4_migrations_belong_to_this_module_and_count_up_from_one`, `r2_every_table_and_view_has_the_module_prefix`, `r2_payables_view_has_the_published_columns`, `r2_ledger_view_has_the_published_columns`, `r2_grn_lines_view_has_the_published_columns`, `pur_05_payables_view_shows_zero_for_a_supplier_with_no_ledger_rows`, `pur_03_grn_lines_view_lists_each_received_line_with_its_grn`, `pur_05_balance_of_an_unknown_supplier_is_not_found`, `r3_errors_turn_into_app_errors_with_codes` |
| `src/aging.rs` (unit) | `pur_05_bucket_edges_are_0_30_60_90_days`, `pur_05_credits_pay_off_the_oldest_amounts_first`, `pur_05_overpayment_shows_as_negative_current` |

The three tests the plan asked for are in **bold**: PO status transitions (partial → received, over-receiving allowed but flagged), ledger balance after an invoice, payment and return (append-only), and aging buckets with a fixed clock.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/purchasing/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/purchasing/` |
| **Screen folder** | `ui/src/modules/purchasing/` |
| **Requirements** | [[Req - Purchasing]] PUR-01 to PUR-06 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Batch ids created by inventory (via `app`), product names and SKUs for snapshots, the low-stock list as `LowStockItem`s (for PO suggestions), the default return cost (current cost), and `PurchasingCtx` (who and when) |
| **Used by** (re-run their tests when the public API changes) | Receiving goods, supplier returns, supplier payments, the PO screens, reports |
| **Test it on its own** | `cargo nextest run -p purchasing` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Recording a GRN does **not** change stock itself. `app` calls inventory for that.
- The payables ledger is append-only.
- PO, GRN and return numbers are gapless: the counter in `purchasing_sequences` goes up inside the caller's transaction.
- `app` must check batch and expiry for products that track expiry before calling `record_grn`; this module doesn't know which products do.
- Recording a GRN does **not** update product cost either. `app` uses `core::moving_average` and catalog.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Built in task B-purchasing: tables, public API and tests above. No `app` wiring and no screens yet (Phase C).
- **ts-rs / serde derives:** the public structs have none yet, because no screen uses them. Add `Serialize`/`Deserialize` and `ts_rs::TS` (and `serde`/`ts-rs` to `Cargo.toml`) when the C task builds the purchasing screens, then run `cargo xtask gen-types`.
- **Error text:** the codes in the error table above need plain-English messages in `ui/src/shared/text/en/errors.ts` (C task).
- **Lists for screens:** there is no `list_orders`, `get_grn`, `list_grns` or `list_returns` yet. The C task adds them when a screen needs them (new functions, same tables).
- **Printing and export** (PO "Send", GRN note, statement export) belong to `printing`/`io` through `app`. This module only provides the data (`get_order`, `statement`).
- **Supplier phone numbers** are stored as typed. [[Data Conventions]] rule 9 says phones are stored in E.164 using the `phonenumber` crate and the shop's country. Decide in the C task whether suppliers follow it (it needs the shop's country from `settings`, passed in by `app`).
- [[Workflows]] calls the return function `record_supplier_return`; the code and this page call it `record_return`. Fix the Workflows page when `app` wires it.
- Migration 001 is the first release of these tables, so the "earlier release" migration test applies from migration 002 on.
