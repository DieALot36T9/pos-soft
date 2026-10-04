---
tags: [module]
crate: core
milestone: M0
---
# Module: core

**Purpose:** shared value types and helpers used by every crate. **No business logic, no database, no I/O.**

**Depends on:** `serde`, `thiserror`, `ulid`, `chrono`, `ts-rs`. **Used by:** every crate.

## Public API
```rust
// money.rs
pub struct Money(pub i64);                 // minor units
impl Money { pub fn zero() -> Self; pub fn checked_add(self, o: Money) -> Option<Money>; /* sub, neg, sum */ }
pub fn mul_qty(price: Money, qty: Qty) -> Money;            // round(price × qty / 1000)
pub fn percent_of(amount: Money, rate: Bps) -> Money;       // round(amount × bps / 10000)
pub fn allocate(total: Money, weights: &[Money]) -> Vec<Money>; // largest remainder, ties → earliest
pub fn cash_round(amount: Money, rule: &RoundingRule) -> Money; // returns the rounded amount

// qty.rs / bps.rs
pub struct Qty(pub i64);                   // thousandths of the product's unit
pub struct Bps(pub u32);                   // 1500 = 15%

// rounding.rs
pub fn round_half_away(numer: i128, denom: i128) -> i64;

// currency.rs
pub struct CurrencyFormat { code, symbol, symbol_before: bool, minor_digits: u8, decimal_sep, grouping: Grouping }
pub fn format_money(m: Money, f: &CurrencyFormat) -> String;
pub fn parse_money(s: &str, minor_digits: u8) -> Result<Money, ParseError>; // rejects extra decimals

// id.rs, time.rs
pub struct Id(String);                     // ULID; Id::new()
pub struct Timestamp(pub i64);             // unix ms UTC
pub struct BusinessDate(NaiveDate);
pub trait Clock { fn now(&self) -> Timestamp; fn local_offset_minutes(&self) -> i32; }
pub fn business_date(now: Timestamp, offset_min: i32, cutoff_hour: u8) -> BusinessDate;
pub fn moving_average(on_hand: Qty, old_cost: Money, recv: Qty, recv_cost: Money) -> Money;

// error.rs, permission.rs
pub enum AppError { /* see Errors and Logging */ }
pub enum Permission { CheckoutSell, /* … full list in Roles and Permissions */ }
pub struct Secret<T>(T);                   // Debug prints ***

// shared data shapes (live here so modules can exchange them without depending on each other)
pub struct ReportTable { /* see Module - reports */ }   // produced by reports, consumed by io (xlsx) and printing (80 mm)
pub struct Bitmap1 { pub width: u32, pub height: u32, pub bits: Vec<u8> } // 1-bit image: produced by barcode, consumed by printing
```
**Rule for adding a type here:** only when two modules must exchange the same data shape. It must be plain data with no behaviour beyond simple constructors.

## Rules
- `float_arithmetic` is denied.
- All arithmetic is checked or done in `i128`. Overflow is an `AppError::Internal`, never a wrap.

## Tests
- Every example E1–E16 in [[Money Tax and Rounding]] that belongs to core (rounding, allocation, cash rounding, moving average, business date).
- proptest: `allocate` sums exactly; `round_half_away` matches a decimal reference; parse/format round trip for 0–3 digits.
- `business_date` across the cutoff and a DST change.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/core/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/core/` |
| **Screen folder** | `ui/src/shared/format/` (must match core's formatting) |
| **Requirements** | [[Money Tax and Rounding]] examples E1–E16 (the ones about rounding, splitting, cash rounding, cost, business date) |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Nothing. `core` depends on no other crate in this project |
| **Used by** (re-run their tests when the public API changes) | **Every crate.** A change here rebuilds and re-tests everything |
| **Test it on its own** | `cargo nextest run -p core` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Add a type here only when two modules must exchange the same data. Never put business rules here.
- No floating-point maths. `float_arithmetic` is denied.
- Money formatting exists twice: `core::format_money` (receipts) and `formatMoney` in the UI. Both are tested against the same file of examples, `crates/core/tests/money_vectors.json`, so they can't disagree.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.
