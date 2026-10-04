---
tags: [module]
crate: barcode
milestone: M4
---
# Module: barcode

**Purpose:** pure barcode logic: parse scanned codes (including scale labels), validate and compute check digits, generate internal codes, render barcode images. **No tables, no I/O.**

**Requirements:** [[Req - Checkout]] CHK-08, [[Req - Catalog]] CAT-04/11/12. **Depends on:** `core`, `barcoders` (bars and SVG; only its `std` and `svg` features). **Tests also use:** `rxing` (reads our pictures back).

## Scale-label rule patterns (Odoo-compatible)
A pattern describes every digit of the barcode **except the last one**, which is always the EAN check digit. So an EAN-13 pattern covers 12 digits and an EAN-8 pattern covers 7. Braces don't count as digits.
- digits = fixed digits (`21`). The pattern must **start** with at least one, so it can't catch ordinary product barcodes
- `.` = item code digit (matched against `catalog_products.scale_item_code`). At least one
- `{N…D…}` = the value, exactly once: `N` whole-number digits, then `D` decimal digits. `{NNNNN}` (no decimals) is allowed
- a sticker with a wrong check digit matches no rule and comes back as `Plain`

| Default rule | Pattern | Kind | Example |
|---|---|---|---|
| Weight, 3 decimals | `21.....{NNDDD}` | weight (in the product's unit) | `2100123012343` → item `00123`, 1.234 |
| Price, 2 decimals | `23.....{NNNDD}` | price (converted to the currency's minor digits) | `2300456150006` → item `00456`, 150.00 |

Both examples have correct EAN-13 check digits and are used as unit tests (they match [[Req - Checkout]] CHK-08).

**Converting the value.** A weight becomes `Qty` (thousandths). A price becomes `Money` in the currency's minor units: 150.00 is `Money(15000)` with 2 decimals, `Money(150)` with 0, `Money(150000)` with 3. If the sticker has more decimals than the target, the extra digits are rounded half away from zero.

**Rule order.** `parse` tries the rules in the order given and uses the first that fits. A rule that fails `validate_rule` is skipped, so one bad setting can't stop the till.

## Public API
```rust
pub enum ScaleKind { Weight, Price }
pub struct ScaleRule { pub name: String, pub pattern: String, pub kind: ScaleKind }
pub enum Scanned { Plain(String), Weighed { item_code: String, qty: Qty }, Priced { item_code: String, price: Money } }
pub fn default_rules() -> Vec<ScaleRule>;                  // "Weight" 21.....{NNDDD}, "Price" 23.....{NNNDD}
pub fn parse(code: &str, rules: &[ScaleRule], currency_minor_digits: u8) -> Scanned; // trims spaces/line endings; invalid check digit → Plain
pub fn validate_rule(rule: &ScaleRule) -> Result<(), AppError>;
pub fn rules_overlap(rules: &[ScaleRule], internal_prefix: &str) -> Option<String>; // name of the first clashing rule
pub fn ean13_check_digit(first12: &str) -> Option<u8>;     // None unless exactly 12 digits
pub fn is_valid_gtin(code: &str) -> bool;                  // EAN-8, UPC-A, EAN-13
pub fn internal_ean13(prefix: &str, n: u64) -> Result<String, AppError>; // pads, adds check digit
pub enum Symbology { Ean13, Ean8, UpcA, Code128 }
pub struct SvgOpts { pub module_width: u32, pub height: u32 } // Default: 2 × 60
pub fn svg(code: &str, sym: Symbology, opts: &SvgOpts) -> Result<String, AppError>;
pub fn bitmap(code: &str, sym: Symbology, width_dots: u32, height_dots: u32) -> Result<Bitmap1, AppError>; // for printing
```
*Changed from the first plan (B-barcode):* `ean13_check_digit`, `internal_ean13`, `svg` and `bitmap` return `Option`/`Result` instead of a plain value, because their input can be wrong (letters, a full number range, a code the symbology can't hold, a label too narrow) and module code must not panic. `default_rules`, `Symbology` and the `SvgOpts` fields were not written down before.

**Details**
- `rules_overlap` compares fixed digits position by position. Internal barcodes are EAN-13, so only 12-digit (EAN-13) rules can clash with the internal prefix. Two rules clash if they have the same length and no position where both ask for different fixed digits; the later rule's name is returned. An internal prefix that isn't digits clashes with nothing (`internal_ean13` refuses it).
- `internal_ean13("200", 1)` gives `2000000000015`. The number is padded with zeros to fill 12 digits.
- Pictures include the blank quiet zones scanners need on both sides: 11 modules for EAN-13, 7 for EAN-8, 9 for UPC-A, 10 for Code 128. EAN-13 is 95 modules + 22 = 117.
- `bitmap` makes every bar a whole number of dots wide (sharp print) and centres the barcode. The picture is exactly `width_dots × height_dots`.
- `svg` includes `xmlns`, so it works on its own as an `<img>`.
- Code 128 uses character set B: printable ASCII only (space to `~`). EAN-13, EAN-8 and UPC-A need the full code with a correct check digit.
- No digits are printed under the bars. The label layout (in `printing`) adds text.

**Error codes** (all `AppError::Validation`)
| Code | When |
|---|---|
| `scale_rule_name_missing` (field `name`) | The rule has no name |
| `scale_rule_pattern_character` (field `pattern`) | A character that isn't allowed there (`N`/`D` outside braces, `.` or a digit inside) |
| `scale_rule_pattern_braces` | `{` not closed, `}` without `{`, or `{` inside `{}` |
| `scale_rule_value_missing` | No `{…}`, or empty `{}` |
| `scale_rule_value_twice` | Two `{…}` groups |
| `scale_rule_value_order` | `N` after `D` |
| `scale_rule_pattern_length` | Not 12 (EAN-13) or 7 (EAN-8) digits |
| `scale_rule_prefix_missing` | Doesn't start with a fixed digit |
| `scale_rule_item_code_missing` | No `.` |
| `internal_prefix_invalid` | Prefix empty, not digits, or longer than 11 |
| `internal_barcode_range_full` | The number doesn't fit after the prefix |
| `barcode_not_drawable` | The code doesn't fit the symbology (wrong length, wrong check digit, character Code 128 set B can't hold) |
| `barcode_too_wide` | `bitmap` has fewer dots than the barcode has modules |
| `barcode_size_invalid` | A width or height of 0 |

## Tests
Every test names its requirement ID. `src/` files hold unit tests; `tests/contract.rs` checks the public API; `tests/round_trip.rs` reads pictures back with `rxing`.
- **Known GTINs valid; one-digit changes invalid:** `cat04_known_gtins_are_valid`, `cat04_changing_any_one_digit_makes_a_gtin_invalid`, `cat04_codes_that_are_not_gtins_are_invalid`, `cat04_ean13_check_digit_of_first_twelve_digits`, `cat04_check_digit_matches_gs1_examples`.
- **Rule parsing (default, custom, malformed):** `chk08_default_rules_match_odoo`, `chk08_weight_label_gives_item_code_and_quantity`, `chk08_price_label_gives_item_code_and_exact_price`, `chk08_price_label_follows_the_currency_decimals`, `chk08_label_with_a_wrong_check_digit_is_treated_as_a_plain_code`, `chk08_custom_rules_work`, `chk08_ean8_rule_works`, `chk08_first_matching_rule_wins_and_broken_rules_are_skipped`, `chk08_malformed_patterns_are_rejected`, `chk08_rule_without_a_name_is_rejected`, `chk08_extra_decimals_are_rounded_half_away_from_zero`.
- **Overlap between the internal prefix `200` and rule prefixes `2x`:** `cat11_internal_prefix_200_does_not_overlap_the_default_scale_rules`, `cat11_internal_prefix_inside_a_scale_prefix_overlaps`, `cat11_scale_rule_wider_than_the_internal_prefix_overlaps`, `chk08_two_scale_rules_for_the_same_codes_overlap`, `chk08_ean8_rules_never_overlap_ean13_codes`.
- **Internal barcodes:** `cat11_internal_barcode_pads_the_number_and_adds_the_check_digit`, `cat11_internal_barcode_refuses_a_number_that_does_not_fit`, `cat11_internal_barcode_refuses_a_bad_prefix`.
- **Generated barcodes decode correctly (round trip through `rxing`):** `cat12_bitmaps_scan_back_to_the_same_code`, `cat12_bitmaps_with_one_dot_per_bar_still_scan`, `cat12_svgs_scan_back_to_the_same_code` (EAN-13, EAN-8, UPC-A and Code 128).

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

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Built in task B-barcode. Nothing is wired into `app` yet (Phase C).
- `ScaleRule`, `ScaleKind` and `Scanned` have no `serde`/`ts-rs` derives, so they don't cross to the UI yet. If the Settings test field (Phase C) needs them on screen, either add the derives here (and run `cargo xtask gen-types`) or map them in `app`.
- Not supported yet, add only when a requirement asks: GS1-128 / GS1 DataBar stickers, ITF-14, EAN-13 add-ons, QR codes (QR on receipts comes from `escpos` in [[Module - printing]]).
