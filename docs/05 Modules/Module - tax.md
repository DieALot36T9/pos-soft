---
tags: [module]
crate: tax
milestone: M2
---
# Module: tax

**Purpose:** the shop's tax rates and the **only** tax calculation in the system.

**Requirements:** [[Req - Tax]]. **Design:** [[ADR-008 Tax model]], [[Money Tax and Rounding]].
**Depends on:** `core`, `rusqlite`.

## Tables
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
CREATE VIEW tax_v_rates AS SELECT id, name, rate_bps, active FROM tax_rates;
```
"Components add up to the rate" is checked in code when saving.

## Public API
```rust
pub enum TaxMode { Inclusive, Exclusive }
pub struct LineTax { pub net: Money, pub tax: Money, pub gross: Money }
pub fn calc_line(amount: Money, rate: Bps, mode: TaxMode) -> LineTax;                // pure
pub fn split_components(tax: Money, rate: &RateWithComponents) -> Vec<(String, Money)>; // pure

pub fn create_rate(tx, NewRate) -> Result<Id>;
pub fn update_rate(tx, &Id, RateUpdate) -> Result<()>;
pub fn deactivate(tx, &Id) -> Result<()>;
pub fn get(conn, &Id) -> Result<RateWithComponents>;
pub fn get_many(conn, ids: &[Id]) -> Result<HashMap<Id, RateWithComponents>>;
pub fn list(conn, include_inactive: bool) -> Result<Vec<RateWithComponents>>;
pub fn migrations() -> ModuleMigrations;
```
"Can't delete a rate in use" is enforced by having **no delete function**. Rates are only deactivated.

## Tests
- TAX-05 table, E4–E7. proptest: inclusive `net + tax == amount`; exclusive `gross == amount + tax`; component sums equal tax.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/tax/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/tax/` |
| **Screen folder** | `ui/src/modules/tax/` |
| **Requirements** | [[Req - Tax]] TAX-01 to TAX-10 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Amounts and the tax mode, passed in by `app` |
| **Used by** (re-run their tests when the public API changes) | Cart pricing, completing a sale, returns, saving a product (checks that the rate exists), the setup wizard |
| **Test it on its own** | `cargo nextest run -p tax` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- This is the **only** place tax is calculated. Never calculate tax anywhere else.
- Rates are never deleted, only deactivated.
- Changing `calc_line` changes money everywhere. The TAX-05 table and examples E4–E7 must stay green.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.
