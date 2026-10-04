---
tags: [module]
crate: sales
milestone: M6, M9
---
# Module: sales

**Purpose:** carts (in progress and held), completed sales with snapshots, payments, payment methods, invoice numbers, returns and voids.

**Requirements:** [[Req - Checkout]], [[Req - Payments]], [[Req - Returns and Voids]]. **Design:** [[ADR-011 Snapshots on sale lines]], [[Workflows]].
**Depends on:** `core`, `rusqlite`.

## Tables
```sql
CREATE TABLE sales_payment_methods (
  id TEXT PRIMARY KEY, name TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('cash','card','wallet','other','credit','points','exchange_credit')),
  requires_reference INTEGER NOT NULL DEFAULT 0, active INTEGER NOT NULL DEFAULT 1,
  sort INTEGER NOT NULL DEFAULT 0, is_system INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE sales_sequences (till_id TEXT NOT NULL, doc TEXT NOT NULL CHECK (doc IN ('sale','return')), next INTEGER NOT NULL, PRIMARY KEY (till_id, doc));

-- Carts: saved on every change
CREATE TABLE sales_carts (
  id TEXT PRIMARY KEY, till_id TEXT NOT NULL, shift_id TEXT NOT NULL, user_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('open','held')),
  customer_id TEXT, price_list_id TEXT, note TEXT,
  invoice_discount_kind TEXT CHECK (invoice_discount_kind IN ('percent','amount')), invoice_discount_value INTEGER,
  exchange_return_id TEXT,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE sales_cart_lines (
  id TEXT PRIMARY KEY, cart_id TEXT NOT NULL REFERENCES sales_carts(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL, product_id TEXT NOT NULL, qty_milli INTEGER NOT NULL,
  source TEXT NOT NULL CHECK (source IN ('scan','search','scale_label','manual')),
  embedded_price_minor INTEGER, price_override_minor INTEGER, override_by TEXT,
  manual_discount_kind TEXT CHECK (manual_discount_kind IN ('percent','amount')), manual_discount_value INTEGER,
  added_at INTEGER NOT NULL
);

-- Completed sales: never updated except void fields
CREATE TABLE sales_sales (
  id TEXT PRIMARY KEY, invoice_no TEXT NOT NULL UNIQUE,
  till_id TEXT NOT NULL, shift_id TEXT NOT NULL, user_id TEXT NOT NULL, business_date TEXT NOT NULL, completed_at INTEGER NOT NULL,
  customer_id TEXT, customer_name TEXT, customer_phone TEXT, price_list_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('completed','voided')),
  base_minor INTEGER NOT NULL, promo_discount_minor INTEGER NOT NULL, manual_discount_minor INTEGER NOT NULL,
  net_minor INTEGER NOT NULL, tax_minor INTEGER NOT NULL, gross_minor INTEGER NOT NULL,
  rounding_minor INTEGER NOT NULL, total_minor INTEGER NOT NULL, change_minor INTEGER NOT NULL,
  cost_minor INTEGER NOT NULL,
  tax_mode TEXT NOT NULL, exchange_return_id TEXT,
  voided_at INTEGER, voided_by TEXT, void_approved_by TEXT, void_reason TEXT
);
CREATE INDEX sales_sales_date ON sales_sales(business_date);
CREATE INDEX sales_sales_shift ON sales_sales(shift_id);
CREATE INDEX sales_sales_customer ON sales_sales(customer_id, completed_at);
CREATE TABLE sales_lines (
  id TEXT PRIMARY KEY, sale_id TEXT NOT NULL REFERENCES sales_sales(id), seq INTEGER NOT NULL,
  product_id TEXT NOT NULL, name TEXT NOT NULL, sku TEXT NOT NULL, unit_symbol TEXT NOT NULL,
  category_id TEXT NOT NULL, brand_id TEXT,
  qty_milli INTEGER NOT NULL, unit_price_minor INTEGER NOT NULL, retail_price_minor INTEGER NOT NULL,
  base_minor INTEGER NOT NULL, promotion_id TEXT, promotion_name TEXT, promo_discount_minor INTEGER NOT NULL,
  tier_discount_minor INTEGER NOT NULL, manual_discount_minor INTEGER NOT NULL, invoice_discount_minor INTEGER NOT NULL,
  net_minor INTEGER NOT NULL, tax_minor INTEGER NOT NULL, gross_minor INTEGER NOT NULL,
  tax_rate_id TEXT NOT NULL, tax_name TEXT NOT NULL, tax_bps INTEGER NOT NULL, tax_components_json TEXT NOT NULL,
  unit_cost_minor INTEGER NOT NULL, price_overridden INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX sales_lines_sale ON sales_lines(sale_id);
CREATE INDEX sales_lines_product ON sales_lines(product_id);

CREATE TABLE sales_returns (
  id TEXT PRIMARY KEY, return_no TEXT NOT NULL UNIQUE, sale_id TEXT REFERENCES sales_sales(id),
  till_id TEXT NOT NULL, shift_id TEXT NOT NULL, user_id TEXT NOT NULL, approved_by TEXT,
  business_date TEXT NOT NULL, at INTEGER NOT NULL, customer_id TEXT, reason TEXT,
  net_minor INTEGER NOT NULL, tax_minor INTEGER NOT NULL, gross_minor INTEGER NOT NULL,
  rounding_minor INTEGER NOT NULL, total_minor INTEGER NOT NULL, cost_minor INTEGER NOT NULL
);
CREATE TABLE sales_return_lines (
  id TEXT PRIMARY KEY, return_id TEXT NOT NULL REFERENCES sales_returns(id), sale_line_id TEXT REFERENCES sales_lines(id),
  product_id TEXT NOT NULL, name TEXT NOT NULL, qty_milli INTEGER NOT NULL,
  net_minor INTEGER NOT NULL, tax_minor INTEGER NOT NULL, gross_minor INTEGER NOT NULL,
  tax_rate_id TEXT NOT NULL, tax_bps INTEGER NOT NULL, unit_cost_minor INTEGER NOT NULL,
  restock INTEGER NOT NULL CHECK (restock IN (0,1))
);
CREATE INDEX sales_return_lines_sale_line ON sales_return_lines(sale_line_id);

CREATE TABLE sales_payments (
  id TEXT PRIMARY KEY, doc_type TEXT NOT NULL CHECK (doc_type IN ('sale','return')), doc_id TEXT NOT NULL,
  method_id TEXT NOT NULL REFERENCES sales_payment_methods(id), kind TEXT NOT NULL,
  amount_minor INTEGER NOT NULL, tendered_minor INTEGER, reference TEXT, at INTEGER NOT NULL
);
CREATE INDEX sales_payments_doc ON sales_payments(doc_type, doc_id);

CREATE VIEW sales_v_lines AS            -- for reports: sale lines with header fields
  SELECT l.*, s.invoice_no, s.business_date, s.completed_at, s.till_id, s.shift_id, s.user_id, s.customer_id, s.status
  FROM sales_lines l JOIN sales_sales s ON s.id = l.sale_id;
CREATE VIEW sales_v_sales AS SELECT * FROM sales_sales;
CREATE VIEW sales_v_return_lines AS
  SELECT rl.*, r.return_no, r.business_date, r.at, r.till_id, r.shift_id, r.user_id, r.customer_id, r.sale_id
  FROM sales_return_lines rl JOIN sales_returns r ON r.id = rl.return_id;
CREATE VIEW sales_v_payments AS
  SELECT p.*, COALESCE(s.business_date, r.business_date) AS business_date, COALESCE(s.shift_id, r.shift_id) AS shift_id,
         COALESCE(s.status, 'completed') AS doc_status
  FROM sales_payments p
  LEFT JOIN sales_sales s ON p.doc_type = 'sale' AND s.id = p.doc_id
  LEFT JOIN sales_returns r ON p.doc_type = 'return' AND r.id = p.doc_id;
```

## Public API (main)
```rust
// carts
pub fn open_cart(tx, till: &Id, shift: &Id, user: &Id, now) -> Result<Id>;
pub fn current_cart(conn, till: &Id, user: &Id) -> Result<Option<Cart>>;
pub fn add_line(tx, cart: &Id, NewCartLine, merge: bool, now) -> Result<Id>;
pub fn set_qty(tx, line: &Id, Qty) -> Result<()>;            // 0 removes
pub fn remove_line(tx, line: &Id) -> Result<()>;
pub fn set_line_discount / set_price_override / set_invoice_discount / set_customer / set_price_list
pub fn hold(tx, cart: &Id, note: Option<String>, now) -> Result<()>;
pub fn held_carts(conn, till: &Id) -> Result<Vec<HeldCartRow>>;
pub fn resume(tx, cart: &Id, user: &Id, shift: &Id) -> Result<()>;
pub fn cancel(tx, cart: &Id) -> Result<Cart>;                // returns the snapshot for audit
pub fn open_carts_on_shift(conn, shift: &Id) -> Result<u32>;
// completion
pub fn complete(tx, CompletedSale /* header + snapshot lines + payments */) -> Result<SaleRef>; // assigns invoice_no, deletes the cart
pub fn get_sale(conn, &Id) -> Result<Sale>;
pub fn find_by_invoice(conn, &str) -> Result<Option<Sale>>;
pub fn last_sale(conn, till: &Id) -> Result<Option<Sale>>;
pub fn returnable_qty(conn, sale: &Id) -> Result<HashMap<Id /*line*/, Qty>>;
pub fn void(tx, sale: &Id, by: &Id, approved_by: Option<&Id>, reason: &str, now) -> Result<()>;
pub fn record_return(tx, NewReturn) -> Result<ReturnRef>;
pub fn has_any_sale(conn) -> Result<bool>;
// payment methods
pub fn payment_methods(conn) -> Result<Vec<PaymentMethod>>; pub fn save_payment_method(tx, …)
pub fn migrations() -> ModuleMigrations;
```
The sales module **does not calculate prices**. `complete` receives fully calculated snapshot lines from the workflow and checks that they're consistent: Σ lines = header totals, payments ≥ total.

## Tests
- Invoice numbers are gapless per till under 1,000 sequential completes; a failed transaction doesn't use a number.
- `complete` refuses inconsistent totals.
- Merge vs non-merge scans, quantity 0 removes, hold/resume/cancel, cart restored after reopening the database.
- `returnable_qty` after partial returns; void refuses after the shift is closed (checked by the workflow; the module provides the shift id).
- The views return the expected columns (contract).

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/sales/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/sales/` |
| **Screen folder** | `ui/src/modules/checkout/` (till screen, payment, held sales, returns) and `ui/src/modules/sales/` (sales journal, receipt viewer) |
| **Requirements** | [[Req - Checkout]], [[Req - Payments]], [[Req - Returns and Voids]] |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | **Fully priced** lines from `app` (sales never calculates prices or tax), plus the shift, till and user ids |
| **Used by** (re-run their tests when the public API changes) | All checkout commands, returns, voids, closing a shift (open-cart check), settings (`has_any_sale`), reports (`sales_v_*` views) |
| **Test it on its own** | `cargo nextest run -p sales` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Invoice numbers must be gapless. They're assigned inside `complete()`, in the same transaction.
- Carts are saved on every change, so a crash loses nothing.
- A completed sale is never changed, except its void fields.
- `complete()` refuses totals that don't add up. Don't loosen this check.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Not started (see [[Task Index]]).
