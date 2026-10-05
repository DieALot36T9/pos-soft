---
tags: [module]
crate: shifts
milestone: M6
---
# Module: shifts

**Purpose:** cashier shifts, every movement of money through the drawer, cash counts, and shift totals for X/Z reports.

**Requirements:** [[Req - Shifts and Cash]], plus RET-04 (voids reverse shift payments) from [[Req - Returns and Voids]]. **Depends on:** `core`, `rusqlite`, `thiserror`.

## Tables
Migration `migrations/001_init.sql`.
```sql
CREATE TABLE shifts_shifts (
  id TEXT PRIMARY KEY, till_id TEXT NOT NULL, user_id TEXT NOT NULL, business_date TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('open','closed')),
  opened_at INTEGER NOT NULL, opening_float_minor INTEGER NOT NULL,
  closed_at INTEGER, closed_by TEXT, forced INTEGER NOT NULL DEFAULT 0 CHECK (forced IN (0,1)),
  expected_cash_minor INTEGER, counted_cash_minor INTEGER, difference_minor INTEGER, close_note TEXT
);
CREATE UNIQUE INDEX shifts_one_open_per_till ON shifts_shifts(till_id) WHERE status = 'open';

CREATE TABLE shifts_movements (               -- append-only: triggers refuse UPDATE and DELETE
  id TEXT PRIMARY KEY, shift_id TEXT NOT NULL REFERENCES shifts_shifts(id),
  kind TEXT NOT NULL CHECK (kind IN ('float','sale_payment','refund','cash_in','cash_out','cash_drop','customer_payment','supplier_payment','void_reversal')),
  method_id TEXT, method_kind TEXT NOT NULL,     -- 'cash','card','wallet',...
  amount_minor INTEGER NOT NULL,                 -- signed: + into the drawer/till, − out
  ref_type TEXT, ref_id TEXT, reason TEXT, at INTEGER NOT NULL, user_id TEXT NOT NULL
);
CREATE INDEX shifts_movements_shift ON shifts_movements(shift_id);
CREATE INDEX shifts_movements_ref ON shifts_movements(shift_id, ref_type, ref_id);
-- triggers shifts_movements_no_update / shifts_movements_no_delete: RAISE(ABORT)
CREATE TABLE shifts_counts (
  shift_id TEXT NOT NULL REFERENCES shifts_shifts(id), stage TEXT NOT NULL CHECK (stage IN ('open','close')),
  denomination_minor INTEGER NOT NULL, count INTEGER NOT NULL,
  PRIMARY KEY (shift_id, stage, denomination_minor)
);
CREATE VIEW shifts_v_shifts AS SELECT * FROM shifts_shifts;
CREATE VIEW shifts_v_movements AS SELECT m.*, s.business_date, s.till_id FROM shifts_movements m JOIN shifts_shifts s ON s.id = m.shift_id;
```
`open` writes the opening float as a `float` movement (method `cash`), so expected cash is simply the sum of all cash movements.

## Public API
Every function returns `Result<_, ShiftsError>`. `ShiftsError` turns into `AppError` (`impl From<ShiftsError> for AppError`).
```rust
pub fn open(tx, till: &Id, user: &Id, float: Money, counts: &[(Money, u32)], business_date: BusinessDate, now: Timestamp) -> Result<Id>;
pub fn current(conn, till: &Id) -> Result<Option<Shift>>;
pub fn record(tx, shift: &Id, movement: ShiftMovement) -> Result<()>;
pub fn reverse_ref(tx, shift: &Id, ref_type: &str, ref_id: &Id, by: &Id, now: Timestamp) -> Result<()>;   // for voids
pub fn expected_cash(conn, shift: &Id) -> Result<Money>;
pub fn close(tx, shift: &Id, counts: &[(Money, u32)], by: &Id, forced: bool, note: Option<String>, now: Timestamp) -> Result<CloseResult>;
pub fn summary(conn, shift: &Id) -> Result<ShiftSummary>;     // totals by kind and method
pub fn migrations() -> ModuleMigrations;

pub const CASH: &str = "cash";                 // the method kind that counts toward expected cash
pub enum ShiftStatus { Open, Closed }
pub enum MovementKind { Float, SalePayment, Refund, CashIn, CashOut, CashDrop, CustomerPayment, SupplierPayment, VoidReversal }
pub struct ShiftMovement { kind, method_id: Option<Id>, method_kind: String, amount: Money, ref_type: Option<String>,
                           ref_id: Option<Id>, reason: Option<String>, at: Timestamp, user_id: Id }
pub struct Shift { id, till_id, user_id, business_date, status, opened_at, opening_float, closed_at, closed_by,
                   forced, expected_cash, counted_cash, difference, close_note }   // close figures are Option
pub struct CloseResult { expected_cash: Money, counted_cash: Option<Money>, difference: Option<Money> }
pub struct DenominationCount { denomination: Money, count: u32 }
pub struct MovementTotal { kind, method_kind: String, count: u32, total: Money }
pub struct MethodTotal { method_kind: String, method_id: Option<Id>, total: Money }
pub struct ShiftSummary { shift: Shift, totals: Vec<MovementTotal>, payments_by_method: Vec<MethodTotal>,
                          expected_cash: Money, open_counts: Vec<DenominationCount>, close_counts: Vec<DenominationCount> }
pub enum ShiftsError { Invalid(&'static str), AlreadyOpen { till }, Closed(Id), NotFound(Id),
                       NothingToReverse { ref_type, ref_id }, AlreadyReversed { ref_type, ref_id }, Core(AppError), Database(rusqlite::Error) }
```

**Rules the functions apply**
| Function | Rule |
|---|---|
| `open` | Float ≥ 0 (`float_negative`). Counts are optional; if given they must add up to the float (`float_count_mismatch`). Refuses a second open shift on the till (`AlreadyOpen`, also enforced by the unique index) |
| `record` | Shift must be open (`Closed`). Sign by kind: `sale_payment`, `cash_in`, `customer_payment` must be > 0; `refund`, `cash_out`, `cash_drop`, `supplier_payment` must be < 0 (`amount_wrong_sign`). `cash_in`/`cash_out`/`cash_drop` need a reason (`reason_missing`, SHF-03). `float` and `void_reversal` are refused (`movement_kind_not_allowed`): only `open` and `reverse_ref` write them. For a cash sale, record the amount kept, after change |
| `reverse_ref` | Shift must be open (RET-04: after close, use a return). Adds a `void_reversal` row with the opposite sign for every movement of that document in this shift, same method. Refuses a second reversal (`AlreadyReversed`) and a document with no movements (`NothingToReverse`) |
| `close` | Shift must be open. Counts required unless `forced` (`count_missing`, SHF-07). Saves expected, counted and difference = counted − expected (a shortage is negative). Doesn't check open carts or permissions: `app` does that first |
| `summary` | `totals`: every movement grouped by kind, then method kind. `payments_by_method`: `sale_payment` + `refund` + `void_reversal`, grouped by method kind and method id (SHF-08 "sales − refunds"). Counts listed smallest denomination first |
| counts | Each denomination > 0 (`denomination_invalid`) and listed once (`denomination_repeated`) |

Error codes sent to the UI: `Invalid(code)` → `validation` with that code; `AlreadyOpen` → conflict `shift_already_open`; `Closed` → conflict `shift_closed`; `AlreadyReversed` → conflict `already_reversed`; `NotFound` → not found `shift`; `NothingToReverse` → not found `shift_movement`; `Database` → internal.

## Tests
Unit tests in `src/rules.rs` and `src/error.rs`; contract tests in `tests/contract.rs` and `tests/close_and_void.rs` (in-memory database built by the module's own migrations, `tests/common/mod.rs`).
- One open shift per till (the unique index): `shf_02_a_second_open_shift_on_the_same_till_is_refused`, `shf_02_the_database_index_refuses_a_second_open_shift`, `shf_02_other_tills_and_a_closed_till_can_open_a_shift`.
- The SHF-06 example: expected vs counted difference: `shf_06_expected_12450_counted_12400_records_a_shortage_of_50`.
- Expected cash includes every cash kind with the right signs; non-cash methods are excluded: `shf_06_expected_cash_adds_every_cash_kind_with_its_sign_and_skips_other_methods`.
- `reverse_ref` makes the shift totals return to what they were before the voided sale: `ret_04_reverse_ref_returns_the_shift_totals_to_before_the_voided_sale` (plus only once, nothing to reverse, closed shift).
- Also: SHF-01 open (float as expected cash, counts kept, count mismatch, negative float), SHF-03 (reason needed, signs by kind), SHF-06 (count needed, closed shift is frozen), SHF-07 force close, SHF-08 summary totals, append-only movements, both views' columns, not found, migrations run on an empty database.

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
- Expected cash counts only movements whose method kind is `cash` (`shifts::CASH`).
- The opening float is stored twice: in `shifts_shifts.opening_float_minor` and as the first `float` movement. Expected cash uses the movement.
- `reverse_ref` works only in the shift that made the sale, while it is open (RET-04).

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Built in task B-shifts: tables, public API and tests. No `app` wiring and no screens yet (Phase C, task C-M6).
- For Phase C: the public structs have no `serde`/`ts-rs` derives yet. Add them (and approve `serde` and `ts-rs` on this page) when `app` sends shifts data to the UI.
- For `app` (C-M6): pass the business date from `core::business_date` with the settings cutoff, check open carts and permissions before `close`, write the audit events (cash in/out, no-sale drawer SHF-04, force close SHF-07). SHF-04 (no-sale drawer opening) moves no money, so it has no movement row here; it is only an audit event.
- For `reports`: counts of sales, returns, voids and cancelled carts (SHF-08) come from `sales`, not from this module.
