---
tags: [module]
crate: inventory
milestone: M5
---
# Module: inventory

**Purpose:** how much of each product the shop has, where it came from and where it went: ledger, levels, batches with expiry (FEFO), adjustments, reorder rules, stock counts.

**Requirements:** [[Req - Inventory]]. **Design:** [[ADR-010 Append-only ledgers]], [[ADR-020 Negative stock allowed by default]].
**Depends on:** `core`, `rusqlite`.

## Tables
```sql
CREATE TABLE inventory_batches (
  id TEXT PRIMARY KEY, product_id TEXT NOT NULL,
  batch_no TEXT, expiry_date TEXT, unit_cost_minor INTEGER NOT NULL DEFAULT 0,
  received_at INTEGER NOT NULL,
  on_hand_milli INTEGER NOT NULL DEFAULT 0          -- cache, same transaction as the ledger
);
CREATE INDEX inventory_batches_fefo ON inventory_batches(product_id, expiry_date);

CREATE TABLE inventory_ledger (
  id              TEXT PRIMARY KEY,
  product_id      TEXT NOT NULL,
  batch_id        TEXT REFERENCES inventory_batches(id),
  qty_delta_milli INTEGER NOT NULL CHECK (qty_delta_milli <> 0),
  kind            TEXT NOT NULL CHECK (kind IN ('sale','sale_void','return','receive','supplier_return','adjust','count','opening')),
  reason_code     TEXT,                              -- for adjust
  note            TEXT,
  ref_type        TEXT, ref_id TEXT,                 -- 'sale', 'grn', 'count', ...
  unit_cost_minor INTEGER NOT NULL DEFAULT 0,
  at              INTEGER NOT NULL,
  business_date   TEXT NOT NULL,
  till_id         TEXT NOT NULL,
  user_id         TEXT
);
CREATE INDEX inventory_ledger_product ON inventory_ledger(product_id, at);
CREATE INDEX inventory_ledger_ref ON inventory_ledger(ref_type, ref_id);
CREATE INDEX inventory_ledger_date ON inventory_ledger(business_date);
-- append-only triggers (no UPDATE / DELETE)

CREATE TABLE inventory_levels (product_id TEXT PRIMARY KEY, on_hand_milli INTEGER NOT NULL, updated_at INTEGER NOT NULL);
CREATE TABLE inventory_reorder_rules (product_id TEXT PRIMARY KEY, reorder_level_milli INTEGER NOT NULL, reorder_qty_milli INTEGER NOT NULL DEFAULT 0);
CREATE TABLE inventory_reasons (
  code TEXT PRIMARY KEY, name TEXT NOT NULL, direction TEXT NOT NULL CHECK (direction IN ('in','out','both')),
  is_system INTEGER NOT NULL DEFAULT 0, active INTEGER NOT NULL DEFAULT 1
);  -- seeded: damage(out), expiry(out), theft(out), count_error(both), opening(in), internal_use(out), other(both)

CREATE TABLE inventory_counts (
  id TEXT PRIMARY KEY, number TEXT NOT NULL UNIQUE, name TEXT NOT NULL,
  scope TEXT NOT NULL CHECK (scope IN ('full','partial')), scope_json TEXT NOT NULL DEFAULT '{}',
  status TEXT NOT NULL CHECK (status IN ('open','posted','cancelled')),
  created_at INTEGER NOT NULL, created_by TEXT NOT NULL, posted_at INTEGER, posted_by TEXT
);
CREATE TABLE inventory_count_lines (
  count_id TEXT NOT NULL REFERENCES inventory_counts(id), product_id TEXT NOT NULL, batch_id TEXT,
  counted_milli INTEGER NOT NULL, expected_milli INTEGER NOT NULL,   -- expected captured at first count
  counted_at INTEGER NOT NULL, counted_by TEXT NOT NULL,
  PRIMARY KEY (count_id, product_id, batch_id)
);
CREATE TABLE inventory_sequences (name TEXT PRIMARY KEY, next INTEGER NOT NULL);

CREATE VIEW inventory_v_levels AS
  SELECT l.product_id, l.on_hand_milli, r.reorder_level_milli, r.reorder_qty_milli
  FROM inventory_levels l LEFT JOIN inventory_reorder_rules r ON r.product_id = l.product_id;
CREATE VIEW inventory_v_ledger AS SELECT * FROM inventory_ledger;
CREATE VIEW inventory_v_batches AS SELECT * FROM inventory_batches WHERE on_hand_milli <> 0;
```

## Public API
```rust
pub struct Movement { pub product_id: Id, pub qty_delta: Qty, pub kind: MoveKind, pub batch: BatchChoice /* Fefo | Specific(Id) | None */,
                      pub reason_code: Option<String>, pub note: Option<String>, pub ref_: Option<(String, Id)>,
                      pub unit_cost: Money, pub ctx: MoveCtx /* at, business_date, till_id, user_id */ }
pub fn record(tx, m: Movement, allow_negative: bool) -> Result<Vec<RecordedRow>>;  // FEFO may split into several rows
pub fn reverse(tx, ref_type: &str, ref_id: &Id, kind: MoveKind, ctx) -> Result<()>; // exact opposite rows (void)
pub fn create_batch(tx, product: &Id, batch_no: Option<&str>, expiry: Option<NaiveDate>, cost: Money, now) -> Result<Id>;
pub fn on_hand(conn, product: &Id) -> Result<Qty>;
pub fn on_hand_many(conn, ids: &[Id]) -> Result<HashMap<Id, Qty>>;
pub fn availability(conn, product: &Id, today: NaiveDate) -> Result<Availability>; // on_hand, has_expired_batch
pub fn set_reorder_rule(tx, product: &Id, level: Qty, qty: Qty) -> Result<()>;
pub fn low_stock(conn, Page) -> Result<Vec<LowStockRow>>;
pub fn near_expiry(conn, today: NaiveDate, days: u32) -> Result<Vec<BatchRow>>;
pub fn history(conn, product: &Id, f: &HistoryFilter) -> Result<Vec<LedgerRowWithBalance>>;
// counts
pub fn create_count / record_count_line / count_variances / post_count / cancel_count
pub fn verify(conn) -> Result<Vec<Mismatch>>;    // levels and batch caches vs ledger sums
pub fn migrations() -> ModuleMigrations;
```

## Tests
- INV-01 example. FEFO across 3 batches with different expiries; running past the batches with negative stock allowed vs refused.
- `reverse` produces the exact opposite rows (same batches).
- Count: expected captured at first count; sales after counting don't create variance; posting creates `count` rows only for differences; full count zeroes uncounted products.
- proptest: random sequences of movements → `verify()` is empty.
- Append-only triggers.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/inventory/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/inventory/` |
| **Screen folder** | `ui/src/modules/inventory/` |
| **Requirements** | [[Req - Inventory]] INV-01 to INV-10 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | The allow-negative and near-expiry settings, whether a product tracks expiry, kits already expanded into components, and `MoveCtx`, all passed in by `app` |
| **Used by** (re-run their tests when the public API changes) | Completing a sale, returns, voids, receiving goods, supplier returns, adjustments, stock-count posting, scanning (stock warning), reports |
| **Test it on its own** | `cargo nextest run -p inventory` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Every stock change goes through `record()`, which writes the ledger row **and** updates the cached level in the same transaction.
- Never UPDATE or DELETE ledger rows. Corrections are new rows.
- `reverse()` must create the exact opposite rows, same batches included.
- The randomized test "levels equal ledger sums" must stay green.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Not started (see [[Task Index]]).
