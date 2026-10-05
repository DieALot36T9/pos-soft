---
tags: [module]
crate: tax
milestone: M2
---
# Module: tax

**Purpose:** the shop's tax rates and the **only** tax calculation in the system.

**Requirements:** [[Req - Tax]]. **Design:** [[ADR-008 Tax model]], [[Money Tax and Rounding]].
**Depends on:** `core`, `rusqlite`, `serde`, `thiserror`, `ts-rs`. Tests also use `proptest` and `serde_json`.

## Tables
Migration `migrations/001_init.sql`:
```sql
CREATE TABLE tax_rates (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  rate_bps    INTEGER NOT NULL CHECK (rate_bps BETWEEN 0 AND 10000),
  active      INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  sort        INTEGER NOT NULL DEFAULT 0,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);
CREATE TABLE tax_rate_components (
  id           TEXT PRIMARY KEY,
  tax_rate_id  TEXT NOT NULL REFERENCES tax_rates(id),
  name         TEXT NOT NULL,
  rate_bps     INTEGER NOT NULL,
  sort         INTEGER NOT NULL
);
CREATE INDEX tax_rate_components_by_rate ON tax_rate_components (tax_rate_id, sort);
-- TAX-10: the database refuses to delete a rate, even with hand-written SQL.
CREATE TRIGGER tax_rates_never_deleted BEFORE DELETE ON tax_rates
BEGIN SELECT RAISE(ABORT, 'tax rates are never deleted, only deactivated'); END;
CREATE VIEW tax_v_rates AS SELECT id, name, rate_bps, active FROM tax_rates;
```
- "Components add up to the rate" is checked in code when saving.
- Names are trimmed. Two rates can't share a name, ignoring capitals ("Standard" and "standard" clash).
- Updating a rate replaces its components. Components are display details, not a ledger.

## Public API
Files: `calc.rs` (the pure calculation), `rates.rs` (saving and reading), `types.rs`, `error.rs`.
```rust
// TaxMode comes from core (pos_core::TaxMode), shared with settings and sales.
// types.rs: all derive serde and ts-rs (written to ui/src/generated/)
pub struct LineTax { pub net: Money, pub tax: Money, pub gross: Money }
pub struct RateComponent { pub name: String, pub rate_bps: Bps }
pub struct RateWithComponents { pub id: Id, pub name: String, pub rate_bps: Bps, pub active: bool,
                                pub sort: i64, pub components: Vec<RateComponent> }
pub struct NewRate    { pub name: String, pub rate_bps: Bps, pub components: Vec<RateComponent>, pub sort: i64 }
pub struct RateUpdate { pub name: String, pub rate_bps: Bps, pub components: Vec<RateComponent>, pub sort: i64 }

// calc.rs: pure, no database
pub fn calc_line(amount: Money, rate: Bps, mode: TaxMode) -> Result<LineTax>;
pub fn split_components(tax: Money, rate: &RateWithComponents) -> Result<Vec<(String, Money)>>;

// rates.rs
pub fn create_rate(tx: &Transaction, new_rate: NewRate, now: Timestamp) -> Result<Id>;
pub fn update_rate(tx: &Transaction, id: &Id, update: RateUpdate, now: Timestamp) -> Result<()>;
pub fn deactivate(tx: &Transaction, id: &Id, now: Timestamp) -> Result<()>;
pub fn get(conn: &Connection, id: &Id) -> Result<RateWithComponents>;            // active or not
pub fn get_many(conn: &Connection, ids: &[Id]) -> Result<HashMap<Id, RateWithComponents>>; // missing ids are left out
pub fn list(conn: &Connection, include_inactive: bool) -> Result<Vec<RateWithComponents>>; // by sort, then name
pub fn migrations() -> ModuleMigrations;

// error.rs
pub type Result<T> = std::result::Result<T, TaxError>;
pub enum TaxError { NameMissing, RateOutOfRange, ComponentNameMissing, ComponentsDoNotAddUp,
                    NameTaken, NotFound { id }, Sqlite(rusqlite::Error), Core(AppError) }
```
- "Can't delete a rate in use" is enforced by having **no delete function**, plus the database trigger above. Rates are only deactivated.
- `calc_line` and `split_components` return a `Result` (not a plain value as first planned), because `core` money helpers report an overflow as an error instead of wrapping.
- `split_components` gives each component `tax × comp_bps / rate_bps` rounded **towards zero**, and the last one gets the rest. For sales this is the same as "floor" in [[Money Tax and Rounding]]. For refunds (negative tax) it splits into the same sizes as the sale, only negative. A rate with no components gives one part named after the rate, so receipts always have a list to show.
- Times come in as `now: Timestamp` from `app` (which reads its `Clock`), so tests control time.

**Error codes** (`TaxError` → `AppError`):

| `TaxError` | `AppError` | code | field |
|---|---|---|---|
| `NameMissing` | Validation | `tax_rate_name_missing` | `name` |
| `RateOutOfRange` | Validation | `tax_rate_out_of_range` | `rate_bps` |
| `ComponentNameMissing` | Validation | `tax_component_name_missing` | `components` |
| `ComponentsDoNotAddUp` | Validation | `tax_components_do_not_add_up` | `components` |
| `NameTaken` | Conflict | `tax_rate_name_taken` | |
| `NotFound` | NotFound | entity `tax_rate`, key = the id | |
| `Sqlite`, `Core` | Internal (or the core error as it is) | | |

## Tests
Every test names its requirement ID. Run with `cargo nextest run -p tax`.
- `tests/calc.rs`: the TAX-05 table, one test per row (rows 1, 2 and 4 are examples E4, E5, E6), half-away rounding, negative (return) amounts, overflow. Example E7 and the other TAX-03 component splits, including refunds and rates with no components.
- `tests/properties.rs` (proptest): inclusive `net + tax == amount`; exclusive `gross == amount + tax`; tax has the amount's sign and is never bigger than it; component sums equal tax.
- `tests/rates.rs`: TAX-02 (create, trim, 0–100% range, unique names), TAX-03 (components saved in order, must add up, need names), TAX-07 (update replaces values, same checks as create, missing rate), TAX-10 (deactivate keeps the rate, list leaves it out, the database refuses DELETE), list order, `get_many`, error codes.
- `tests/contract.rs`: migrations labelled and numbered, every table/view/index/trigger starts with `tax_`, `tax_v_rates` columns and values, a saved rate feeding `calc_line` and `split_components`, JSON shape the UI receives.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/tax/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/tax/` |
| **Screen folder** | `ui/src/modules/tax/` |
| **Requirements** | [[Req - Tax]] TAX-01 to TAX-10 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Amounts, the tax mode and the current time (`now`), passed in by `app` |
| **Used by** (re-run their tests when the public API changes) | Cart pricing, completing a sale, returns, saving a product (checks that the rate exists), the setup wizard |
| **Test it on its own** | `cargo nextest run -p tax` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- This is the **only** place tax is calculated. Never calculate tax anywhere else.
- Rates are never deleted, only deactivated.
- Changing `calc_line` changes money everywhere. The TAX-05 table and examples E4–E7 must stay green.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Built in task B-tax: tables, calculation, rate functions and all tests above. No `app` commands and no screens yet; those come in Phase C (C-M2 setup wizard and rate screens, C-M6 checkout).
- For Phase C: add the UI text for the error codes above to `ui/src/shared/text/en/errors.ts`.
- TAX-01 (tax label), TAX-04 (the shop's mode), and TAX-08/TAX-09 (receipt and report) are stored or shown by other modules (`settings`, `printing`, `reports`). This module only gives them rates and `split_components`; `TaxMode` itself is in `core`.
- [[Money Tax and Rounding]] step 8 says components get `floor(…)`. The code rounds towards zero, which is the same for sales and keeps refunds symmetric (see Public API). The product owner may want that page's wording changed to match; it's outside this module's folders.
- There is no "reactivate a rate" function. Add one if the product owner asks for it.
