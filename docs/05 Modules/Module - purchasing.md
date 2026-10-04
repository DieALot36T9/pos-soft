---
tags: [module]
crate: purchasing
milestone: M10
---
# Module: purchasing

**Purpose:** suppliers, purchase orders, goods received (GRN), supplier returns, and what the shop owes suppliers.

**Requirements:** [[Req - Purchasing]]. **Depends on:** `core`, `rusqlite`.

## Tables
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
```rust
pub fn create_supplier / update_supplier / archive_supplier / list_suppliers
pub fn create_order(tx, NewOrder) -> Result<OrderRef>;  pub fn update_order(...);  pub fn mark_sent(...);  pub fn cancel_order(...);
pub fn order_for_receiving(conn, order: &Id) -> Result<Vec<OrderLineRemaining>>;
pub fn record_grn(tx, NewGrn /* lines include inventory_batch_id set by workflow */) -> Result<GrnRef>;   // updates PO received qty/status + ledger invoice row
pub fn record_return(tx, NewSupplierReturn) -> Result<ReturnRef>;   // ledger return_credit row
pub fn record_payment(tx, supplier: &Id, amount: Money, method_kind, reference, ctx) -> Result<Id>;
pub fn aging(conn, today: NaiveDate) -> Result<Vec<AgingRow>>;
pub fn statement(conn, &Id, from, to) -> Result<Statement>;
pub fn migrations() -> ModuleMigrations;
```

## Tests
- PO status transitions (partial → received when all lines are fully received; over-receiving is allowed but flagged).
- Ledger balance after an invoice, payment and return; append-only.
- Aging buckets with a fixed clock.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/purchasing/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/purchasing/` |
| **Screen folder** | `ui/src/modules/purchasing/` |
| **Requirements** | [[Req - Purchasing]] PUR-01 to PUR-06 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Batch ids created by inventory (via `app`), product names and SKUs for snapshots, and the low-stock list (for PO suggestions) |
| **Used by** (re-run their tests when the public API changes) | Receiving goods, supplier returns, supplier payments, the PO screens, reports |
| **Test it on its own** | `cargo nextest run -p purchasing` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Recording a GRN does **not** change stock itself. `app` calls inventory for that.
- The payables ledger is append-only.
- PO, GRN and return numbers are gapless.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.
