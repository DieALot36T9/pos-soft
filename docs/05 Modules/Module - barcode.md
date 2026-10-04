---
tags: [module]
crate: barcode
milestone: M4
---
# Module: barcode

**Purpose:** pure barcode logic: parse scanned codes (including scale labels), validate and compute check digits, generate internal codes, render barcode images. **No tables, no I/O.**

**Requirements:** [[Req - Checkout]] CHK-08, [[Req - Catalog]] CAT-04/11/12. **Depends on:** `core`, `barcoders`.

## Scale-label rule patterns (Odoo-compatible)
A pattern is 13 characters for EAN-13 (or 8 for EAN-8):
- digits = fixed prefix (`21`)
- `.` = item code digit (matched against `catalog_products.scale_item_code`)
- `{N…D…}` = value: `N` integer digits, `D` decimal digits
- the last character is always the EAN check digit
| Default rule | Pattern | Kind | Example |
|---|---|---|---|
| Weight, 3 decimals | `21.....{NNDDD}` | weight (in the product's unit) | `2100123012343` → item `00123`, 1.234 |
| Price, 2 decimals | `23.....{NNNDD}` | price (converted to the currency's minor digits) | `2300456150006` → item `00456`, 150.00 |

Both examples have correct EAN-13 check digits and are used as unit tests (they match [[Req - Checkout]] CHK-08).

## Public API
```rust
pub enum ScaleKind { Weight, Price }
pub struct ScaleRule { pub name: String, pub pattern: String, pub kind: ScaleKind }
pub enum Scanned { Plain(String), Weighed { item_code: String, qty: Qty }, Priced { item_code: String, price: Money } }
pub fn parse(code: &str, rules: &[ScaleRule], currency_minor_digits: u8) -> Scanned; // invalid check digit → Plain (not found later)
pub fn validate_rule(rule: &ScaleRule) -> Result<(), AppError>;
pub fn rules_overlap(rules: &[ScaleRule], internal_prefix: &str) -> Option<String>;
pub fn ean13_check_digit(first12: &str) -> u8;
pub fn is_valid_gtin(code: &str) -> bool;                  // EAN-8, UPC-A, EAN-13
pub fn internal_ean13(prefix: &str, n: u64) -> String;     // pads, adds check digit
pub fn svg(code: &str, sym: Symbology, opts: &SvgOpts) -> String;
pub fn bitmap(code: &str, sym: Symbology, width_dots: u32, height_dots: u32) -> Bitmap1; // for printing
```

## Tests
- Known GTINs valid; one-digit changes invalid.
- Rule parsing: every default rule, custom rules, malformed patterns rejected.
- Overlap detection between the internal prefix `200` and rule prefixes `2x`.
- Generated barcodes decode correctly (round trip through the `rxing` decoder in dev-dependencies).

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/barcode/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/barcode/` |
| **Screen folder** | None of its own. The barcode-rules test field in the Settings screens calls it through `app` |
| **Requirements** | [[Req - Checkout]] CHK-08; [[Req - Catalog]] CAT-04, CAT-11, CAT-12 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Scale-label rules and the internal prefix (from settings), the number of currency decimals |
| **Used by** (re-run their tests when the public API changes) | Scanning, adding a barcode (check digit), generating internal barcodes, labels |
| **Test it on its own** | `cargo nextest run -p barcode` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Pure code: no tables, no I/O.
- The default rules must stay compatible with Odoo's format.
- The barcode examples in CHK-08 and on this page are tests. Keep them correct (check digits included).

**Done when:** the [[Definition of Done]] is met, and this page matches the code.
