---
tags: [module]
crate: settings
milestone: M2
---
# Module: settings

**Purpose:** typed, versioned settings for the shop and till, and first-run setup progress.

**Requirements:** [[Req - Setup and Settings]]. **Screens:** [[Setup Wizard]], [[Back Office Screens#Settings]].
**Depends on:** `core`, `rusqlite`, `serde`, `serde_json`, `thiserror`, `ts-rs` (types for the screens), `iso_currency` (ISO 4217 currency list, with its `iterator` feature; see [[Reuse Map]]).

## Tables
```sql
CREATE TABLE settings_values (
  key         TEXT PRIMARY KEY,          -- e.g. 'shop_profile', 'currency', 'rounding'
  value_json  TEXT NOT NULL CHECK (json_valid(value_json)),
  updated_at  INTEGER NOT NULL,
  updated_by  TEXT                       -- user id (NULL during setup)
);
CREATE TABLE settings_setup_steps (
  step         TEXT PRIMARY KEY CHECK (step IN
                 ('shop', 'currency', 'tax', 'admin', 'till', 'hardware', 'backup', 'products')),
  completed_at INTEGER NOT NULL
);
```
Migration: `migrations/001_init.sql`. No views: other modules read settings through `get` and `snapshot`, which `app` passes in.

## Typed settings (each is one row)
Each struct has `#[serde(default)]`, a default and a `validate()`. The validation codes are listed so the screens can turn them into messages.

| Key | Struct | Main fields (default) | `validate()` refuses (code) |
|---|---|---|---|
| `language` | `LanguageSettings` | language (`en`; only `en` in v1.0, kept so more languages can be added later) | anything but `en` (`language_not_supported`) |
| `shop_profile` | `ShopProfile` | name, address_lines, phone, email, tax_reg_label (`BIN`), tax_reg_no, logo_path, receipt_footer | blank name (`shop_name_missing`); email without `@` (`email_invalid`) |
| `currency` | `CurrencyFormat` (core) | code (`BDT`), symbol (`৳`), symbol_before (yes), minor_digits (2), decimal_sep (`.`), group_sep (`,`), grouping (Indian) | code not in ISO 4217 (`currency_code_unknown`); digits over 3 (`minor_digits_out_of_range`); empty decimal_sep (`decimal_sep_missing`); a digit or `-` in a separator (`separator_invalid`); both separators the same (`separators_same`); group_sep over 8 bytes (`group_sep_too_long`) |
| `rounding` | `RoundingRule` (core) | increment_minor (0 = off), mode (nearest), applies_to (cash only) | negative step (`rounding_step_negative`) |
| `tax` | `TaxSettings` | label (`VAT`), mode `TaxMode` from `core` (inclusive / exclusive; inclusive), default_rate_id (none) | blank label (`tax_label_missing`) |
| `till` | `TillSettings` | till_id (none until the till step; `app` sets it), till_code (`T1`), business_day_cutoff_hour (4) | code not 1–6 letters/digits (`till_code_invalid`); hour over 23 (`cutoff_hour_out_of_range`) |
| `receipt` | `ReceiptSettings` | paper_width `PaperWidth` (`mm58` / `mm80`; 80), show_logo, show_tax_breakdown, show_tax_reg_no, show_invoice_barcode, show_customer_balance (all yes), copies (1), header_lines, footer_lines | copies not 1–3 (`receipt_copies_out_of_range`) |
| `hardware` | `HardwareSettings` | receipt_printer `Option<PrinterConnection>`, drawer_pin `DrawerPin` (pin 2), scale `Option<ScaleSettings>` (port, baud 9600, profile name), label_printer. `PrinterConnection` mirrors `hardware::PrinterConfig` (windows_spooler, tcp, serial, cups, preview, file) | a printer missing its name/host/port/queue/path (`printer_connection_incomplete`); a scale without port or baud (`scale_port_missing`) or profile (`scale_profile_missing`) |
| `scale_barcode_rules` | `ScaleRules(Vec<ScaleRule>)` | name, pattern, kind `ScaleKind` (weight/price). Default: the two rules on [[Module - barcode]] (`21.....{NNDDD}` weight, `23.....{NNNDD}` price) | blank name (`scale_rule_name_missing`) or pattern (`scale_rule_pattern_missing`). `app` also calls `barcode::validate_rule` |
| `internal_barcode` | `InternalBarcode` | prefix (`200`) | not 2–3 digits starting with 2, the GS1 in-store range (`internal_prefix_invalid`) |
| `backup` | `BackupSettings` | primary_dir (empty = app data `backups/`, filled in by `app`), secondary_dir, interval_min (60), keep_last (50) | interval not 1–1440 (`backup_interval_out_of_range`); keep 0 (`backup_keep_out_of_range`). `app` checks the folders can be written |
| `logging` | `LoggingSettings` | detailed (no) | nothing |
| `checkout` | `CheckoutSettings` | merge_scans (yes), idle_lock_min (5), blind_close (yes), sounds (yes) | idle lock not 1–120 (`idle_lock_out_of_range`) |
| `inventory` | `InventorySettings` | allow_negative (yes), near_expiry_days (30), expired_policy `ExpiredPolicy` (warn / block; warn), cost_method `CostMethod` (moving_average / last_cost; moving average) | over 365 days (`near_expiry_days_out_of_range`) |
| `loyalty` | `LoyaltySettings` | enabled (no), spend_unit_minor (10000), points_per_unit (1), value_per_point_minor (100), min_redeem (0) | spend unit, points or point value not above 0 (`loyalty_spend_unit_not_positive`, `loyalty_points_not_positive`, `loyalty_point_value_not_positive`); negative min_redeem (`loyalty_min_redeem_negative`) |
| `credit` | `CreditSettings` | enabled (no), default_limit_minor (0) | negative limit (`credit_limit_negative`) |
| `denominations` | `Denominations(Vec<Money>)` | notes and coins (empty; the wizard fills it from `suggested_denominations`) | a value of 0 or less (`denomination_not_positive`); a repeated value (`denomination_repeated`) |

`ScaleRules` and `Denominations` are newtypes with `#[serde(transparent)]`, so they are stored as plain JSON lists.

## Public API
```rust
pub trait Setting: Serialize + DeserializeOwned {
    const KEY: &'static str;
    fn default_value() -> Self;                 // not `Default`: CurrencyFormat and RoundingRule live in core
    fn validate(&self) -> Result<(), AppError>; // AppError::Validation { field, code }
}
pub fn get<S: Setting>(conn: &Connection) -> Result<S>;            // default if missing
pub fn set<S: Setting>(tx: &Transaction, value: &S, by: Option<&Id>, now: Timestamp) -> Result<Option<S>>; // validates; returns old value for audit
pub fn setup_status(conn: &Connection) -> Result<SetupStatus>;    // { completed (wizard order), next_step, is_complete }
pub fn complete_step(tx: &Transaction, step: SetupStep, now: Timestamp) -> Result<()>; // again = no change
pub fn snapshot(conn: &Connection) -> Result<SettingsSnapshot>;   // all settings at once for workflows (one query)
pub fn migrations() -> ModuleMigrations;

pub enum SetupStep { Shop, Currency, Tax, Admin, Till, Hardware, Backup, Products } // ALL, as_str(), is_required()
pub struct SettingsSnapshot { language, shop_profile, currency, rounding, tax, till, receipt, hardware,
                              scale_barcode_rules, internal_barcode, backup, logging, checkout, inventory,
                              loyalty, credit, denominations }

// Currency helpers for the wizard (rules SET-05, SET-16)
pub fn currency_choices() -> Vec<CurrencyChoice>;                 // { code, name, minor_digits, symbol } from ISO 4217
pub fn currency_format_for(code: &str) -> Result<CurrencyFormat, AppError>; // ready-made format; BDT/INR/NPR use Indian grouping
pub fn suggested_denominations(code: &str, minor_digits: u8) -> Denominations; // BDT, INR, USD, EUR, GBP; others empty

pub enum SettingsError { Invalid(AppError), Database(rusqlite::Error), Unreadable { key, detail } } // → AppError
pub type Result<T> = std::result::Result<T, SettingsError>;
```
- `Invalid` turns into the `AppError::Validation` it holds. `Database` and `Unreadable` (a stored value that can't be read back) turn into `AppError::Internal`.
- **Setup steps:** `next_step` is the first unfinished step in wizard order. `is_complete` is true when every step except `Hardware` is done, because hardware can be finished later (SET-10). So `next_step` can be `Hardware` while `is_complete` is true.
- `currency_choices` leaves out funds, metals, replaced currencies and currencies with more than 3 decimals. A currency without a symbol in ISO 4217 shows its code.
- Locking minor digits after the first sale is checked in the `app` workflow (it asks `sales::has_any_sale`).

## Tests
- `tests/contract.rs`: missing keys return defaults (SET-13); invalid values are refused and not saved, e.g. cutoff 24 (SET-09) and minor digits 4 (SET-05); `set` returns the old value and records who and when (SET-13); fields missing from older saves get defaults; an unreadable stored value is an internal error; setup status ordering and resume (SET-02), completed steps in wizard order, setup complete without hardware (SET-10); `snapshot` holds every setting; the currency list comes from ISO 4217 (SET-05); suggested denominations (SET-16).
- `tests/round_trip.rs`: round trip of every struct (SET-13).
- Unit tests next to each struct: one test per `validate()` rule, named after its requirement (SET-03 to SET-17); step names match the table; printer connections are sent with their `kind`.

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
- A new setting needs: a struct with `#[serde(default)]` and `ts(export)`, a `Setting` impl with a new `KEY`, a field in `SettingsSnapshot`, and a round-trip test. Then run `cargo xtask gen-types`.
- Never change a `KEY` or rename a field: saved settings would silently fall back to defaults. Add a new field instead.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Built in task B-settings: all typed settings, `get`/`set`, setup progress, `snapshot`, the ISO 4217 currency list and suggested denominations.
- **Connected in C-M2** (`app::workflows::setup` and `settings_change`): the `minor_digits` lock after a sale (SET-05), writable backup folders (SET-11), `till_id` set once at the till step, the default `backups` folder filled in by `app` when `primary_dir` is empty (not saved), old and new values audited (SET-13). Screens: the setup wizard (`ui/src/modules/setup/`) and the Settings screen (`ui/src/modules/settings/`: shop, money and rounding, tax, till, receipt, backups, logs).
- **Still to do in `app`:** call `barcode::validate_rule` and `barcode::rules_overlap` before saving scale rules (no screen for them yet); Settings sections for hardware, payment methods, denominations, checkout, inventory, loyalty, credit and barcode rules; the logo upload (SET-04); the live receipt preview (SET-14).
- Payment methods (SET-15) are owned by `sales` (`sales_payment_methods`), not stored here.
- Text lengths (shop name, receipt lines) have no upper limit yet. Add one if the receipt layout needs it (B-printing).
- Loyalty defaults (spend unit 100.00, point value 1.00) assume 2 currency decimals. Scale them for 0, 1 or 3 decimals when the loyalty settings get their screen (not done in C-M2).
