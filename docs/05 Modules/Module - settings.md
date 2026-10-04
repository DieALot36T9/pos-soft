---
tags: [module]
crate: settings
milestone: M2
---
# Module: settings

**Purpose:** typed, versioned settings for the shop and till, and first-run setup progress.

**Requirements:** [[Req - Setup and Settings]]. **Screens:** [[Setup Wizard]], [[Back Office Screens#Settings]].
**Depends on:** `core`, `rusqlite`, `serde_json`.

## Tables
```sql
CREATE TABLE settings_values (
  key         TEXT PRIMARY KEY,          -- e.g. 'shop_profile', 'currency', 'rounding'
  value_json  TEXT NOT NULL,
  updated_at  INTEGER NOT NULL,
  updated_by  TEXT                       -- user id (NULL during setup)
);
CREATE TABLE settings_setup_steps (
  step         TEXT PRIMARY KEY,         -- 'shop','currency','tax','admin','till','hardware','backup','products'
  completed_at INTEGER NOT NULL
);
```

## Typed settings (each is one row)
| Key | Struct | Main fields |
|---|---|---|
| `language` | `LanguageSettings` | language (only `en` in v1.0; kept so more languages can be added later) |
| `shop_profile` | `ShopProfile` | name, address_lines, phone, email, tax_reg_label, tax_reg_no, logo_path, receipt_footer |
| `currency` | `CurrencyFormat` (core) | code, symbol, position, minor_digits, separators, grouping |
| `rounding` | `RoundingRule` (core) | increment_minor, mode, applies_to |
| `tax` | `TaxSettings` | label, mode (inclusive/exclusive), default_rate_id |
| `till` | `TillSettings` | till_id, till_code, business_day_cutoff_hour |
| `receipt` | `ReceiptSettings` | paper_width, show_* flags, copies, header/footer lines |
| `hardware` | `HardwareSettings` | receipt printer, drawer pin, scale port + profile, label printer |
| `scale_barcode_rules` | `Vec<ScaleRule>` | name, pattern, kind (weight/price) |
| `internal_barcode` | `InternalBarcode` | prefix (default "200") |
| `backup` | `BackupSettings` | primary_dir, secondary_dir, interval_min, keep_last |
| `logging` | `LoggingSettings` | detailed: bool |
| `checkout` | `CheckoutSettings` | merge_scans, idle_lock_min, blind_close, sounds |
| `inventory` | `InventorySettings` | allow_negative, near_expiry_days, expired_policy, cost_method |
| `loyalty` | `LoyaltySettings` | enabled, spend_unit_minor, points_per_unit, value_per_point_minor, min_redeem |
| `credit` | `CreditSettings` | enabled, default_limit_minor |
| `denominations` | `Vec<Money>` | notes and coins |

## Public API
```rust
pub trait Setting: Serialize + DeserializeOwned + Default { const KEY: &'static str; fn validate(&self) -> Result<(), AppError>; }
pub fn get<S: Setting>(conn: &Connection) -> Result<S>;            // Default if missing
pub fn set<S: Setting>(tx: &Transaction, value: &S, by: Option<&Id>, now: Timestamp) -> Result<Option<S>>; // returns old value for audit
pub fn setup_status(conn: &Connection) -> Result<SetupStatus>;    // completed steps, next step, is_complete
pub fn complete_step(tx: &Transaction, step: SetupStep, now: Timestamp) -> Result<()>;
pub fn snapshot(conn: &Connection) -> Result<SettingsSnapshot>;   // all settings at once for workflows
pub fn migrations() -> ModuleMigrations;
```
Locking minor digits after the first sale is checked in the `app` workflow (it asks `sales::has_any_sale`).

## Tests
- Missing keys return defaults. Invalid values are rejected by `validate` (e.g. cutoff 24, minor digits 4).
- Round trip of every struct. `set` returns the old value.
- Setup status ordering and resume.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/settings/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/settings/` |
| **Screen folder** | `ui/src/modules/setup/` and `ui/src/modules/settings/` |
| **Requirements** | [[Req - Setup and Settings]] SET-01 to SET-17 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | The user id (for the audit), and whether any sale exists yet (to lock currency decimals), both passed in by `app` |
| **Used by** (re-run their tests when the public API changes) | Almost every workflow, through `settings::snapshot`. Also the setup wizard and the Settings screens |
| **Test it on its own** | `cargo nextest run -p settings` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- When adding a field to a settings struct, give it a default (`#[serde(default)]`). Otherwise settings saved by older versions won't load.
- Every settings struct has a `validate()`. Add a test for each new rule.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Not started (see [[Task Index]]).
