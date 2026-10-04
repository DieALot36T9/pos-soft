---
tags: [module]
crate: shifts
milestone: M6
---
# Module: shifts

**Purpose:** cashier shifts, every movement of money through the drawer, cash counts, and shift totals for X/Z reports.

**Requirements:** [[Req - Shifts and Cash]]. **Depends on:** `core`, `rusqlite`.

## Tables
```sql
CREATE TABLE shifts_shifts (
  id TEXT PRIMARY KEY, till_id TEXT NOT NULL, user_id TEXT NOT NULL, business_date TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('open','closed')),
  opened_at INTEGER NOT NULL, opening_float_minor INTEGER NOT NULL,
  closed_at INTEGER, closed_by TEXT, forced INTEGER NOT NULL DEFAULT 0,
  expected_cash_minor INTEGER, counted_cash_minor INTEGER, difference_minor INTEGER, close_note TEXT
);
CREATE UNIQUE INDEX shifts_one_open_per_till ON shifts_shifts(till_id) WHERE status = 'open';

CREATE TABLE shifts_movements (               -- append-only
  id TEXT PRIMARY KEY, shift_id TEXT NOT NULL REFERENCES shifts_shifts(id),
  kind TEXT NOT NULL CHECK (kind IN ('float','sale_payment','refund','cash_in','cash_out','cash_drop','customer_payment','supplier_payment','void_reversal')),
  method_id TEXT, method_kind TEXT NOT NULL,     -- 'cash','card','wallet',...
  amount_minor INTEGER NOT NULL,                 -- signed: + into the drawer/till, − out
  ref_type TEXT, ref_id TEXT, reason TEXT, at INTEGER NOT NULL, user_id TEXT NOT NULL
);
CREATE INDEX shifts_movements_shift ON shifts_movements(shift_id);
CREATE TABLE shifts_counts (
  shift_id TEXT NOT NULL REFERENCES shifts_shifts(id), stage TEXT NOT NULL CHECK (stage IN ('open','close')),
  denomination_minor INTEGER NOT NULL, count INTEGER NOT NULL,
  PRIMARY KEY (shift_id, stage, denomination_minor)
);
CREATE VIEW shifts_v_shifts AS SELECT * FROM shifts_shifts;
CREATE VIEW shifts_v_movements AS SELECT m.*, s.business_date, s.till_id FROM shifts_movements m JOIN shifts_shifts s ON s.id = m.shift_id;
```

## Public API
```rust
pub fn open(tx, till: &Id, user: &Id, float: Money, counts: &[(Money, u32)], business_date, now) -> Result<Id>;
pub fn current(conn, till: &Id) -> Result<Option<Shift>>;
pub fn record(tx, shift: &Id, ShiftMovement) -> Result<()>;
pub fn reverse_ref(tx, shift: &Id, ref_type: &str, ref_id: &Id, by: &Id, now) -> Result<()>;   // for voids
pub fn expected_cash(conn, shift: &Id) -> Result<Money>;
pub fn close(tx, shift: &Id, counts: &[(Money, u32)], by: &Id, forced: bool, note: Option<String>, now) -> Result<CloseResult>;
pub fn summary(conn, shift: &Id) -> Result<ShiftSummary>;     // totals by kind and method
pub fn migrations() -> ModuleMigrations;
```

## Tests
- One open shift per till (the unique index).
- The SHF-06 example: expected vs counted difference.
- Expected cash includes every cash kind with the right signs; non-cash methods are excluded.
- `reverse_ref` makes the shift totals return to what they were before the voided sale.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/shifts/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/shifts/` |
| **Screen folder** | `ui/src/modules/shifts/` |
| **Requirements** | [[Req - Shifts and Cash]] SHF-01 to SHF-08 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | The business date and cash denominations (from settings), and money movements from workflows |
| **Used by** (re-run their tests when the public API changes) | Opening and closing shifts, completing a sale, returns, voids, cash in/out, customer payments, supplier cash payments, reports |
| **Test it on its own** | `cargo nextest run -p shifts` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Only one open shift per till (enforced by a database index).
- Amounts are signed: + means money into the drawer, − means money out.
- Expected cash counts only movements whose method is cash.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Not started (see [[Task Index]]).
