---
tags: [module]
crate: inventory
milestone: M5
---
# Module: inventory

**Purpose:** how much of each product the shop has, where it came from and where it went: ledger, levels, batches with expiry (FEFO), adjustments, reorder rules, stock counts.

**Requirements:** [[Req - Inventory]]. **Design:** [[ADR-010 Append-only ledgers]], [[ADR-020 Negative stock allowed by default]].
**Depends on:** `core`, `rusqlite`, `serde`, `thiserror`, `chrono` (expiry dates are `NaiveDate`), `serde_json` (a count's scope details are stored as JSON), `ts-rs` (TypeScript types for the stock screens, ADR-014). Tests also use `proptest`.

## Tables
Migration `migrations/001_init.sql`. The SQL below is a summary; the file is the source of truth.
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
  reason_code     TEXT,                              -- needed for adjust
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
CREATE INDEX inventory_ledger_batch ON inventory_ledger(batch_id);        -- for verify
-- triggers inventory_ledger_no_update / inventory_ledger_no_delete: RAISE(ABORT, 'inventory_ledger is append-only')

CREATE TABLE inventory_levels (product_id TEXT PRIMARY KEY, on_hand_milli INTEGER NOT NULL, updated_at INTEGER NOT NULL);
CREATE TABLE inventory_reorder_rules (product_id TEXT PRIMARY KEY, reorder_level_milli INTEGER NOT NULL, reorder_qty_milli INTEGER NOT NULL DEFAULT 0);
CREATE TABLE inventory_reasons (
  code TEXT PRIMARY KEY, name TEXT NOT NULL, direction TEXT NOT NULL CHECK (direction IN ('in','out','both')),
  is_system INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0,1)), active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1))
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
-- SQLite lets NULLs repeat in a primary key, so this keeps one whole-product line (batch_id NULL) too:
CREATE UNIQUE INDEX inventory_count_lines_one_each ON inventory_count_lines(count_id, product_id, IFNULL(batch_id, ''));
CREATE TABLE inventory_sequences (name TEXT PRIMARY KEY, next INTEGER NOT NULL);   -- 'count' → SC-000001

-- Published views (other modules read only these)
CREATE VIEW inventory_v_levels AS          -- product_id, on_hand_milli, reorder_level_milli, reorder_qty_milli
  SELECT l.product_id, l.on_hand_milli, r.reorder_level_milli, r.reorder_qty_milli
  FROM inventory_levels l LEFT JOIN inventory_reorder_rules r ON r.product_id = l.product_id
  UNION ALL   -- products with a reorder rule but no stock movement yet show with 0, so they appear as low stock
  SELECT r.product_id, 0, r.reorder_level_milli, r.reorder_qty_milli FROM inventory_reorder_rules r
  WHERE r.product_id NOT IN (SELECT product_id FROM inventory_levels);
CREATE VIEW inventory_v_ledger AS SELECT * FROM inventory_ledger;
CREATE VIEW inventory_v_batches AS SELECT * FROM inventory_batches WHERE on_hand_milli <> 0;
```

## Public API
Every function returns `Result<_, InventoryError>`; `InventoryError` turns into `AppError` (`?` works in `app` workflows). Functions that change data take `&Transaction`; readers take `&Connection`.
```rust
// Stock changes (record.rs, types.rs)
pub enum MoveKind { Sale, SaleVoid, Return, Receive, SupplierReturn, Adjust, Count, Opening }  // as_str, parse, allows(qty)
pub enum BatchChoice { Fefo, Specific(Id), None }
pub struct MoveCtx { pub at: Timestamp, pub business_date: BusinessDate, pub till_id: Id, pub user_id: Option<Id> }
pub struct Movement { pub product_id: Id, pub qty_delta: Qty, pub kind: MoveKind, pub batch: BatchChoice,
                      pub reason_code: Option<String>, pub note: Option<String>, pub ref_: Option<(String, Id)>,
                      pub unit_cost: Money, pub tracks_expiry: bool, pub ctx: MoveCtx }
impl Movement { pub fn new(product: &Id, qty_delta: Qty, kind: MoveKind, ctx: MoveCtx) -> Movement } // no batch/reason/ref, cost 0
pub struct RecordedRow { pub id: Id, pub batch_id: Option<Id>, pub qty_delta: Qty }
pub fn record(tx, m: Movement, allow_negative: bool) -> Result<Vec<RecordedRow>>;           // FEFO may split into several rows
pub fn reverse(tx, ref_type: &str, ref_id: &Id, kind: MoveKind, ctx: &MoveCtx) -> Result<Vec<RecordedRow>>; // exact opposite rows

// Batches (batches.rs)
pub struct BatchRow { id, product_id, batch_no: Option<String>, expiry_date: Option<NaiveDate>, unit_cost: Money, received_at: Timestamp, on_hand: Qty }
pub fn create_batch(tx, product: &Id, batch_no: Option<&str>, expiry: Option<NaiveDate>, cost: Money, now: Timestamp) -> Result<Id>;
pub fn batches_of(conn, product: &Id) -> Result<Vec<BatchRow>>;             // batches with stock ≠ 0, FEFO order
pub fn get_batch(conn, id: &Id) -> Result<BatchRow>;                       // any batch, even empty (history, C-M5); unknown → NotFound
pub fn near_expiry(conn, today: NaiveDate, days: u32) -> Result<Vec<BatchRow>>;

// Levels and reorder rules (levels.rs)
pub fn on_hand(conn, product: &Id) -> Result<Qty>;                          // 0 if never moved
pub fn on_hand_many(conn, ids: &[Id]) -> Result<HashMap<Id, Qty>>;          // every id asked for is in the answer
pub struct Availability { pub on_hand: Qty, pub has_expired_batch: bool }
pub fn availability(conn, product: &Id, today: NaiveDate) -> Result<Availability>;
pub struct ReorderRule { pub level: Qty, pub qty: Qty }
pub fn set_reorder_rule(tx, product: &Id, level: Qty, qty: Qty) -> Result<()>;
pub fn reorder_rule(conn, product: &Id) -> Result<Option<ReorderRule>>;
pub fn remove_reorder_rule(tx, product: &Id) -> Result<()>;
pub fn is_product_used(conn, product: &Id) -> Result<bool>;   // any ledger row or count line: archive only (CAT-08, C-M4)
pub struct Page { pub offset: u32, pub limit: u32 }                          // Page::first(limit)
pub struct LowStockRow { product_id, on_hand, reorder_level, reorder_qty }
pub fn low_stock(conn, page: Page) -> Result<Vec<LowStockRow>>;             // biggest shortfall first
pub fn low_stock_count(conn) -> Result<u32>;                                // the back-office badge

// History (history.rs)
pub struct LedgerRow { id, product_id, batch_id, qty_delta, kind, reason_code, note, ref_, unit_cost, at, business_date, till_id, user_id }
pub struct HistoryFilter { pub from: Option<BusinessDate>, pub to: Option<BusinessDate>, pub kinds: Vec<MoveKind>, pub page: Page }
pub struct LedgerRowWithBalance { pub entry: LedgerRow, pub balance_after: Qty }
pub fn history(conn, product: &Id, f: &HistoryFilter) -> Result<Vec<LedgerRowWithBalance>>;   // newest first

// Adjustment reasons (reasons.rs)
pub enum ReasonDirection { In, Out, Both }                                   // as_str, parse, fits(qty)
pub struct Reason { code, name, direction, is_system: bool, active: bool }
pub fn list_reasons(conn) -> Result<Vec<Reason>>;                           // built-in first, then by name
pub fn add_reason(tx, code: &str, name: &str, direction: ReasonDirection) -> Result<()>;
pub fn rename_reason(tx, code: &str, name: &str) -> Result<()>;
pub fn set_reason_active(tx, code: &str, active: bool) -> Result<()>;       // built-in reasons can't be switched off

// Stock counts (counts.rs, count_post.rs)
pub enum CountScope { Full, Partial }   pub enum CountStatus { Open, Posted, Cancelled }   // as_str, parse
pub struct NewCount { pub name: String, pub scope: CountScope, pub scope_details: serde_json::Value }
pub struct CountRef { pub id: Id, pub number: String }                       // "SC-000001"
pub struct CountRow { id, number, name, scope, scope_details, status, created_at, created_by, posted_at, posted_by }
pub struct CountEntry { pub product_id: Id, pub batch_id: Option<Id>, pub qty: Qty }
pub struct VarianceRow { product_id, batch_id, expected, counted, difference, cost_value: Money, was_counted: bool }
pub fn create_count(tx, c: &NewCount, now: Timestamp, user: &Id) -> Result<CountRef>;
pub fn get_count(conn, id: &Id) -> Result<CountRow>;
pub fn list_counts(conn, page: Page) -> Result<Vec<CountRow>>;              // newest first
pub fn record_count_line(tx, count: &Id, e: &CountEntry, now: Timestamp, user: &Id) -> Result<()>;
pub fn count_variances(conn, count: &Id, costs: &HashMap<Id, Money>) -> Result<Vec<VarianceRow>>;
pub fn post_count(tx, count: &Id, costs: &HashMap<Id, Money>, ctx: &MoveCtx) -> Result<Vec<RecordedRow>>;
pub fn cancel_count(tx, count: &Id) -> Result<()>;

pub struct Mismatch { pub product_id: Id, pub batch_id: Option<Id>, pub cached: Qty, pub ledger_sum: Qty }
pub fn verify(conn) -> Result<Vec<Mismatch>>;    // levels and batch caches vs ledger sums; never fixes anything
pub fn migrations() -> ModuleMigrations;
```

## Rules the code follows
| Rule | What the code does |
|---|---|
| Direction per kind | `sale`, `supplier_return` take stock out. `sale_void`, `return`, `receive`, `opening` bring it in. `adjust`, `count` go either way. Wrong way → `qty_wrong_sign`. Zero → `qty_zero` |
| Reasons (INV-02) | `adjust` needs a reason (`reason_needed`). Any reason given must exist and be active (`reason_unknown`) and fit its direction (`reason_wrong_direction`). Reason `other` needs a note (`note_needed`) |
| Negative stock (INV-04, ADR-020) | With `allow_negative` off, a stock-out that would leave the product below 0 is refused: `Conflict not_enough_stock` with `available_milli` (e.g. "3000" = 3 pieces). A chosen batch can't go below 0 either (`not_enough_stock_in_batch`). Nothing is written |
| FEFO (INV-05) | `BatchChoice::Fefo` takes from batches with stock: earliest expiry first, batches with no expiry last, then oldest delivery. What's left after the batches is one row with no batch. Expired batches are included (the till warns or blocks before this, using `availability`). FEFO is only for stock going out; stock coming in names its batch (`batch_needed`) |
| Expiry tracking (INV-05) | If `tracks_expiry` is set, stock coming in needs a batch with an expiry date (`expiry_needed`). `app` reads the flag from catalog |
| Batches | A batch must belong to the movement's product (`batch_of_other_product`). An unknown batch is `NotFound` |
| Expired | A batch is still good **on** its expiry date. `has_expired_batch` means stock in a batch whose date has passed |
| Near expiry | Batches with stock expiring on or before `today + days`, including ones already expired, soonest first. Batches without a date are left out |
| Low stock (INV-03) | A product is low when on hand is **below** its reorder level: level 10 and stock 10 is fine, 9 is low |
| Reverse | Writes the exact opposite of every row pointing at the document: same batches, cost and reason, new time and user, `kind` as given. The new rows point at the same document. A document with no rows (only kits or non-stock products) → nothing. Reversing twice → `Conflict already_reversed`. It doesn't check `allow_negative`: a correction must always go through |
| Counts (INV-06) | `SC-000001` numbers come from `inventory_sequences` in the same transaction (gapless). The first scan of an item keeps the stock at that moment as `expected`; more scans add up; a negative scan takes back a mistake but the total can't go below 0 (`counted_below_zero`). A product is counted **by batch or as a whole**, not both (`count_mixes_batch_and_whole`). Posting writes `count` rows of *counted − expected* (so later sales don't count as differences), pointing at `("count", id)`. A **full** count also sets every uncounted product with stock ≠ 0 to 0: each batch is emptied, then the rest outside batches. Only open counts can be changed, posted or cancelled (`count_not_open`) |
| Cost | `count_variances` and `post_count` take each product's current cost from `app` (catalog). A missing product is valued at 0 |
| Overflow | Levels and batch stock are added up in Rust with checked maths, never by SQLite (which would turn an overflow into a decimal) |

## Tests
| File | What it covers |
|---|---|
| `src/fefo.rs` (unit) | The pure FEFO split: whole batches in order, skipping empty ones, the rest without a batch, parts always add up |
| `tests/ledger.rs` | **INV-01** example (stock 10, sell 3 → 7 and a −3 `sale` row pointing at the invoice); kinds go their own way; **INV-05** FEFO across 3 batches with different expiries; undated batches last; **INV-04** running past the batches with negative stock allowed (rest without batch) vs refused (nothing saved, available shown); batch rules; expiry tracking |
| `tests/adjustments.rs` | **INV-02** reason rules and reason management; **RET-04** `reverse` writes the exact opposite rows on the same batches, twice is refused; **INV-09** a document with no rows |
| `tests/counts.rs` | **INV-06**: expected captured at first count; scans add up; sales after counting don't create a difference; posting creates `count` rows only for differences; full count zeroes uncounted products (with and without batches); partial count leaves others alone; counting by batch; no mixing; closed counts refuse changes |
| `tests/stock_lists.rs` | **INV-01** on hand; **INV-03** reorder level 10, stock 9 → low; rule-only products; paging; **INV-05** near expiry, availability, batch details |
| `tests/history_and_verify.rs` | **INV-07** running balance (true even when filtered), date filter, link to the document; **INV-10** `verify` reports wrong levels and batches without fixing them |
| `tests/append_only.rs` | **INV-01** append-only triggers refuse UPDATE and DELETE |
| `tests/properties.rs` | **INV-10** proptest: random sequences of receives, FEFO and plain sales, batch adjustments, voids and counts (with negative stock on and off) → `verify()` is empty and every level equals its ledger sum |
| `tests/contract.rs` | Migrations numbered from 1; every table, view, index and trigger has the `inventory_` prefix; the three views' columns; `inventory_v_levels` includes rule-only products; enum texts; `cat_08_a_product_that_moved_in_stock_is_used`; `get_batch` finds empty batches; the TypeScript types of `MoveKind`, `CountScope`, `CountStatus`, `ReasonDirection` |

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/inventory/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/inventory/` |
| **Screen folder** | `ui/src/modules/inventory/` |
| **Requirements** | [[Req - Inventory]] INV-01 to INV-10 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | The allow-negative and near-expiry settings, whether a product tracks expiry (`Movement.tracks_expiry`), current cost prices (`costs` for counts), kits already expanded into components, and `MoveCtx`, all passed in by `app` |
| **Used by** (re-run their tests when the public API changes) | Completing a sale, returns, voids, receiving goods, supplier returns, adjustments, stock-count posting, scanning (stock warning), saving and importing products (reorder rules), reports (through the views) |
| **Test it on its own** | `cargo nextest run -p inventory` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Every stock change goes through `record()` (or `reverse()`, or `post_count()`), which writes the ledger row **and** updates the cached level and batch in the same transaction. All three write through `ledger::write_row`.
- Never UPDATE or DELETE ledger rows. Corrections are new rows. The database triggers refuse it anyway.
- `reverse()` must create the exact opposite rows, same batches included.
- The randomized test "levels equal ledger sums" must stay green.
- If `record()` returns an error, it may already have written some rows of a FEFO split. The caller (the `app` workflow) rolls back the whole transaction, as it does for every error.
- Kits and products with "track stock" off must never reach `record()` (INV-09). `app` expands kits into their components first.
- Stock value (INV-08) is worked out by `reports` from `inventory_v_levels` and catalog's cost; this module has no value function.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Built in task B-inventory: Rust crate, migration, public API and tests.
- **Built in C-M5:** `app` wiring (`workflows/stock.rs`, `workflows/stock_counts.rs`, `commands/inventory.rs`) and the screens in `ui/src/modules/inventory/` (Stock, product stock with batches, adjust and history, Stock counts with counting by scanning and the variance review, stock alerts on the Home page). See [[Module - app]] for what is not built yet.
- **TypeScript types:** `MoveKind`, `CountScope`, `CountStatus`, `ReasonDirection` and `Reason` derive `ts_rs::TS`. The other structs (`BatchRow`, `LedgerRow`, `VarianceRow`, …) reach the screens through `app`'s view types, which add names and units.
- **`Page`** is defined here because `core` has no shared paging type yet. If one is added to `core` (catalog and audit also page lists), switch to it.
- Partial counts store their scope (category, brand, location note, product list) as JSON for display only. Choosing which products to count from that scope is for `app`/screens, since categories and brands belong to catalog. In C-M5 the screen shows the scope ("Dairy · Aisle 3") but any product can be counted.
- A whole-product count line of a product that has batches writes its difference with no batch; the batches keep their stock. The count screen (C-M5) asks for the batch whenever a scanned product has batches with stock, with "Whole product" as the first choice.
