---
tags: [module]
crate: core
milestone: M0
---
# Module: core

**Purpose:** shared value types and helpers used by every crate. **No business logic, no database, no I/O.**

**Depends on:** `serde`, `thiserror`, `ulid`, `chrono`, `ts-rs`, `num-format` (digit grouping). **Used by:** every crate.

**Crate name:** the folder is `crates/core/`, but the package is **`pos-core`** and code writes **`pos_core::Money`**. Rust's standard library already has a crate called `core`, so ours can't use that name. Other pages that say `core::Money` mean `pos_core::Money`.

## Public API
Everything that can overflow returns `Result<_, AppError>`. An overflow is an `AppError::Internal`, never a wrap.
```rust
// money.rs
pub struct Money(pub i64);                 // minor units
impl Money {
  pub const fn zero() -> Money;
  pub fn checked_add(self, other: Money) -> Result<Money, AppError>;
  pub fn checked_sub(self, other: Money) -> Result<Money, AppError>;
  pub fn checked_neg(self) -> Result<Money, AppError>;
  pub fn sum(amounts: &[Money]) -> Result<Money, AppError>;
}
pub fn mul_qty(price: Money, qty: Qty) -> Result<Money, AppError>;        // round(price × qty / 1000)
pub fn percent_of(amount: Money, rate: Bps) -> Result<Money, AppError>;   // round(amount × bps / 10000)

// allocate.rs: largest remainder, ties → earliest line. All-zero weights split evenly.
pub fn allocate(total: Money, weights: &[Money]) -> Result<Vec<Money>, AppError>;

// rounding.rs
pub fn round_half_away(numer: i128, denom: i128) -> Result<i64, AppError>;
pub struct RoundingRule { pub increment_minor: Money, pub mode: RoundingMode, pub applies_to: RoundingAppliesTo }
pub enum RoundingMode { Nearest, Down, Up }            // Down/Up go by size: refunds round like sales
pub enum RoundingAppliesTo { AllPayments, CashOnly }   // used by sales; cash_round ignores it
pub fn cash_round(amount: Money, rule: &RoundingRule) -> Result<Money, AppError>; // the rounded amount; step 0 = off

// cost.rs
pub fn moving_average(on_hand: Qty, old_cost: Money, received: Qty, received_cost: Money) -> Result<Money, AppError>;

// qty.rs / bps.rs
pub struct Qty(pub i64);                   // thousandths of the product's unit
pub struct Bps(pub u32);                   // 1500 = 15%

// currency.rs (rule SET-05)
pub struct CurrencyFormat { code, symbol, symbol_before: bool, minor_digits: u8, decimal_sep, group_sep, grouping: Grouping }
pub enum Grouping { Western, Indian }
pub fn format_money(m: Money, f: &CurrencyFormat) -> String;      // "৳12,34,567.50"; symbol after → "12.50 ৳"; grouping by num-format
pub fn parse_money(s: &str, minor_digits: u8) -> Result<Money, ParseMoneyError>; // "123.45" only; rejects extra decimals
pub enum ParseMoneyError { Empty, NotANumber, TooManyDecimals { max_decimals }, TooLarge } // → AppError::Validation (money_* codes)

// id.rs, time.rs
pub struct Id(String);                     // ULID; Id::new(), Id::parse(&str), id.as_str()
pub struct Timestamp(pub i64);             // unix ms UTC
pub struct BusinessDate(pub NaiveDate);    // "YYYY-MM-DD"; BusinessDate::parse(&str)
pub trait Clock: Send + Sync { fn now(&self) -> Timestamp; fn local_offset_minutes(&self) -> i32; }
pub struct SystemClock;                    // the real clock
pub struct FixedClock;                     // tests: new(now, offset), set, advance_ms, set_offset_minutes
pub fn business_date(now: Timestamp, offset_min: i32, cutoff_hour: u8) -> Result<BusinessDate, AppError>;

// error.rs, permission.rs, secret.rs
pub enum AppError { Validation{field, code, params}, NotFound{entity, key}, Conflict{code, params},
                    PermissionDenied{permission}, OverrideRequired{permission}, Hardware{device, code, detail},
                    Internal{id, detail} }   // sent as {"kind": "validation", …}; Internal's detail is never sent
impl AppError { pub fn validation(code: &str) -> AppError; pub fn internal(detail) -> AppError; }
pub enum Permission { CheckoutSell, /* … all 42 from Roles and Permissions */ }  // Permission::ALL, as_str(), parse()
pub struct Secret<T>(T);                   // Debug prints ***; Secret::new, expose(); read from the UI, never sent back

// shared data shapes (live here so modules can exchange them without depending on each other)
pub struct ReportTable { title, params: ReportParams, columns: Vec<Column>, rows: Vec<Vec<Cell>>, totals, sections }
pub struct ReportParams { from, to, filters: Vec<ReportFilter> }   pub struct Column { key, title, kind: ColumnKind }
pub enum ColumnKind { Text, Money, Qty, Int, Percent, Date }
pub enum Cell { Empty, Text(String), Money(Money), Qty(Qty), Int(i64), Percent(Bps), Date(BusinessDate) }
pub struct Bitmap1 { pub width: u32, pub height: u32, pub bits: Vec<u8> } // rows of (width+7)/8 bytes, first pixel = high bit, 1 = black
pub struct Migration { pub version: u32, pub name: &'static str, pub sql: &'static str }   // run by db
pub struct ModuleMigrations { pub module: &'static str, pub migrations: &'static [Migration] }
```
**Rule for adding a type here:** only when two modules must exchange the same data shape. It must be plain data with no behaviour beyond simple constructors. `Migration` and `ModuleMigrations` live here (not in `db`) because every module returns them and modules can't depend on `db`.

**TypeScript:** every type the UI sees derives `ts_rs::TS` and is written to `ui/src/generated/` by `cargo xtask gen-types`. `Money`, `Qty` and other i64 values become a plain `number` (they stay below 2^53).

## Rules
- `float_arithmetic` is denied.
- All arithmetic is checked or done in `i128`. Overflow is an `AppError::Internal`, never a wrap.

## Tests
- `tests/examples.rs`: every example E1–E16 in [[Money Tax and Rounding]]. E7 (tax), E12–E13 (sales) and E16 (customers) belong to other modules, so here they test the core building blocks with the same numbers.
- `tests/money_vectors.rs` + `tests/money_vectors.json`: `format_money` and `parse_money` against the shared examples. The UI's `formatMoney`/`parseMoney` pass the same file.
- `tests/properties.rs` (proptest): `allocate` sums exactly; `round_half_away` matches a second reference rounding; cash rounding is never more than half the step; parse/format round trip for 0–3 digits.
- Unit tests in `currency.rs`: digit grouping (Western, Indian, none) through `num-format`.
- Unit tests: `business_date` across the cutoff, negative offsets and a DST change; ids; permissions; error JSON shape; `Secret` hides its value.

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
| **Test it on its own** | `cargo nextest run -p pos-core` · screens: `pnpm test src/shared/format` |

**Watch out for**
- Add a type here only when two modules must exchange the same data. Never put business rules here.
- No floating-point maths. `float_arithmetic` is denied.
- Digit grouping uses the `num-format` crate (`Grouping::Standard` / `Grouping::Indian`), not our own code ([[Reuse Map]]). `num-format` allows a group separator of at most 8 bytes; a longer one leaves the digits ungrouped.
- Money formatting exists twice: `core::format_money` (receipts) and `formatMoney` in the UI. Both are tested against the same file of examples, `crates/core/tests/money_vectors.json`, so they can't disagree.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Built in task A0. Task A1 replaced the hand-written digit grouping with `num-format`; every money test and `money_vectors.json` passed unchanged.
- For B-settings: `group_sep` should be limited to 8 bytes when the shop saves it, because `num-format` can't use a longer separator (it then prints the number ungrouped).
- `cargo-deny` excludes dev-dependencies for every crate, not only `reports` (cargo-deny can't do it per crate). So a dev-dependency from another module on a module crate is caught by review, not CI ([[Module Rules]] R1). Revisit if cargo-deny gains per-crate graph options.
