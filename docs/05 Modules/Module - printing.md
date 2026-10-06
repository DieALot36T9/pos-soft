---
tags: [module]
crate: printing
milestone: M1 (test), M7
---
# Module: printing

**Purpose:** turn data into printable bytes: a simple **document model** → layouts for receipts, X/Z reports, labels and shelf tags → a 1-bit **raster image** → **ESC/POS** or **TSPL** bytes. Pure. It never talks to devices.

**Requirements:** [[Req - Hardware and Printing]], [[Req - Catalog]] CAT-12/13. **Design:** [[ADR-012 Receipts printed as raster images]].
**Depends on:** `core`, `thiserror`, and these approved libraries:

| Library | What for |
|---|---|
| `cosmic-text` (features `std`, `swash`; no system fonts) | Shaping text, breaking it into lines, and drawing each letter (through `swash`) |
| `unicode-script` | Only to write our font-fallback list for `cosmic-text` (its `Fallback` trait names scripts with this crate's type) |
| `tiny-skia` | The drawing page (an 8-bit `Mask`: how dark each dot is) and filling the rule lines |
| `qrcode` | QR codes |
| `image` (features `png`, `jpeg`) | Reading and dithering the shop logo; writing PNG previews |
| `escpos` | ESC/POS start (`ESC @`) and cut (`GS V A 0`) commands, through an in-memory driver |
| `insta` (tests only) | Byte snapshots of ESC/POS and TSPL output |

**Fonts** (in `crates/printing/fonts/`, licence in `fonts/OFL.txt`): **Noto Sans** Regular and Bold (Google Fonts, SIL Open Font License). Noto Sans has every sign in the Unicode currency block (₹, €, ₦, …) but **not the taka sign ৳**, so two tiny cuts of **Noto Sans Bengali** (Regular and Bold, 1.5 KB each) hold just ৳ and ৲. Fonts for other languages can be added later ([[ADR-022 Ready for more languages later]]). The computer's own fonts are never used, so a printout is the same on every operating system.

**Tables:** none. **Views:** none. **Migrations:** none (pure code).

## Document model
```rust
pub struct Doc { pub width_dots: u32, pub blocks: Vec<Block> }
pub enum Block {
  Text { text: String, size: TextSize, bold: bool, align: Align },        // wraps; a too-long word is broken
  Row { left: String, right: String, size: TextSize, bold: bool },      // "Milk 1L      180.00": left wraps, right stays on line 1
  Columns { cells: Vec<(String, u16 /*weight*/, Align)>, size: TextSize, bold: bool }, // a table row; a cell's share = weight / sum
  Rule { style: RuleStyle },                                            // Single = dashed ----, Double = ====
  Image { bitmap: Bitmap1, align: Align },                              // logo
  Barcode { bitmap: Bitmap1 },                                          // centred; core::Bitmap1 from the barcode crate, passed in by app
  Qr { data: String, size_dots: u32 },                                  // centred; whole dots per square, so it may come out a bit smaller
  Feed { dots: u32 },
}
pub enum TextSize { Small /*18 dots*/, Normal /*22*/, Large /*30*/, Huge /*56*/ }   // line height = size × 5/4
pub enum Align { Left, Center, Right }   pub enum RuleStyle { Single, Double }
pub enum PaperWidth { Mm58, Mm80 }        // .dots() → 384 / 576 (rule HW-02)
pub struct LabelSize { pub width_mm: u32, pub height_mm: u32, pub gap_mm: u32 }   // .width_dots(), .height_dots(): 8 dots per mm (203 dpi)
```
Normal text fits about 32 characters on 58 mm paper and 48 on 80 mm.

## Layouts (data structs are owned here; `app` maps sales to them)
Layouts only arrange and format. Every amount, unit price and total arrives already worked out. Dates and times arrive as text already written the shop's way.
```rust
pub fn receipt(d: &ReceiptData, l: &ReceiptLayout, labels: &ReceiptLabels) -> Doc;          // sale; a void receipt when d.void_reason is set (RET-04)
pub fn return_receipt(d: &ReturnData, l: &ReceiptLayout, labels: &ReceiptLabels) -> Doc;    // RET-06
pub fn shift_report(r: &ReportTable, l: &ReceiptLayout, labels: &ReceiptLabels) -> Doc;     // X/Z, daily summary (SHF-08)
pub fn label(d: &LabelData, tpl: &LabelTemplate, labels: &ReceiptLabels) -> Doc;            // CAT-12
pub fn shelf_tag(d: &ShelfTagData, tpl: &TagTemplate, labels: &ReceiptLabels) -> Doc;       // CAT-13
```
| Struct | Fields |
|---|---|
| `ReceiptLayout` (rule SET-14) | `paper`, `currency: CurrencyFormat`, `shop: ShopHeader`, `logo: Option<Bitmap1>`, `header_lines`, `footer_lines`, `show_logo`, `show_tax_breakdown`, `show_tax_reg_no`, `show_invoice_barcode`, `show_customer_points`. Copies (1–3) are not here: `app` sends the bytes again |
| `ShopHeader` (rule SET-04) | `name`, `address_lines`, `phone`, `tax_reg_label` ("BIN"), `tax_reg_no`. Empty phone / number are not printed |
| `ReceiptData` | `invoice_no`, `printed_at`, `till`, `cashier`, `customer: Option<CustomerOnReceipt>`, `lines: Vec<ReceiptLine>`, `subtotal`, `discount_total`, `tax_summary: Vec<TaxSummaryLine>`, `tax_label` ("VAT"), `prices_include_tax`, `rounding`, `total`, `payments: Vec<PaymentLine>`, `change`, `void_reason: Option<String>`, `copy: bool` (a reprint: the copy heading at the top, PAY-10), `invoice_barcode: Option<Bitmap1>`, `qr_data: Option<String>`. Zero discount, rounding and change are left out |
| `ReturnData` | `return_no`, `original_invoice_no: Option<String>` (None for a return without receipt), `printed_at`, `till`, `cashier`, `customer`, `lines`, `tax_summary`, `tax_label`, `prices_include_tax`, `refund_total`, `refunds: Vec<PaymentLine>`, `reason: Option<String>` |
| `ReceiptLine` | `name`, `qty: Qty`, `unit` ("kg", or empty for pieces), `unit_price`, `amount` (before the line discount), `discount: Option<LineDiscount { label, amount }>` (amount as printed, negative for a saving). A "2 × 90.00" line is printed unless the line is exactly one piece |
| `TaxSummaryLine` | `name` ("VAT 15%"), `net`, `tax` |
| `PaymentLine` | `method`, `amount`, `reference: Option<String>` |
| `CustomerOnReceipt` | `name`, `points_balance: Option<i64>`, `credit_balance: Option<Money>` |
| `LabelTemplate` | `size: LabelSize`, `currency`, `show_name`, `show_price`, `show_unit_price`, `show_barcode`, `show_packed_on`, `show_expires_on` |
| `LabelData` | `name`, `price`, `unit_price: Option<UnitPrice { price, unit }>`, `barcode: Option<Bitmap1>`, `barcode_text`, `packed_on`, `expires_on` (`Option<String>`) |
| `TagTemplate` | `size: LabelSize`, `currency`, `show_unit_price`, `show_barcode` |
| `ShelfTagData` | `name`, `price` (normal), `unit_price` (for the big price shown), `barcode`, `barcode_text`, `promotion: Option<PromotionOnTag { price, from, to }>`. With a promotion the big price is the promotion price, with "Was …" and the offer dates under it |

**`ReceiptLabels`: every printed word** ([[ADR-022 Ready for more languages later]]). `app` fills it; layouts never type words of their own (a test checks this). Fields: `invoice_no`, `date`, `till`, `cashier`, `customer`, `points`, `credit_balance`, `subtotal`, `discount`, `rounding`, `total`, `change`, `reference`, `tax_summary`, `tax_net`, `tax_amount`, `void_heading`, `copy_heading` ("COPY", C-M7), `reason`, `return_heading`, `return_no`, `original_invoice`, `refund_total`, `period`, `packed_on`, `expires_on`, and these small templates: `prices_include_tax` ("Prices include {tax}"), `date_range` ("{from} to {to}"), `unit_price` ("{price} per {unit}"), `regular_price` ("Was {price}"), `offer_dates` ("Offer {from} to {to}"). The English words are in `crates/printing/tests/common/mod.rs` (`english_labels`), ready for `app` to copy.

Money uses `core::format_money`: the total, refund, label and tag prices with the symbol ("৳1,805.00"); line amounts and report columns without it ("1,805.00"). Quantities drop extra zeros ("2", "1.234 kg"); percentages too ("15%", "12.5%").

## Rendering and encoding
```rust
pub struct Fonts;  pub fn load_fonts() -> Fonts;                     // once at startup; Send + Sync, one printout drawn at a time
pub fn rasterize(doc: &Doc, fonts: &Fonts) -> Result<Bitmap1, PrintError>;   // cosmic-text shaping + tiny-skia page, threshold to 1-bit
pub fn to_png(b: &Bitmap1) -> Result<Vec<u8>, PrintError>;            // previews in UI and golden tests (8-bit grey: black 0, white 255)
pub fn logo_from_image(file: &[u8], max_width_dots: u32) -> Result<Bitmap1, PrintError>; // PNG/JPEG → shrink to fit (never enlarge) → dither
pub mod escpos { pub struct EscPosOpts { pub cut: bool, pub drawer: Option<DrawerPin>, pub band_height: u32 }  // Default: cut, no drawer, 256 rows
                 pub fn encode(b: &Bitmap1, opts: &EscPosOpts) -> Result<Vec<u8>, PrintError>;  // ESC @, GS v 0 strips, GS V A 0, ESC p
                 pub fn drawer_pulse(pin: DrawerPin) -> Result<Vec<u8>, PrintError>;            // ESC p m 25 250
                 pub enum DrawerPin { Pin2, Pin5 } }
pub mod tspl   { pub fn encode(b: &Bitmap1, label: &LabelSize, copies: u32) -> Vec<u8>; }   // SIZE, GAP, CLS, BITMAP, PRINT 1,copies
pub enum PrintError { LogoUnreadable(String), QrTooLong, TooBig { width, height }, BadPicture, Png(String), EscPos(String) }
```
- Receipt previews in Settings use `to_png`, so **the preview is exactly what prints**.
- `PrintError` → `AppError`: `LogoUnreadable` → `Validation { field: "logo", code: "logo_unreadable" }`; `QrTooLong` → `Validation { code: "qr_text_too_long" }`; the rest are bugs → `Internal`.
- How a picture is drawn: every block first gets its place; text and rules are drawn on a `tiny-skia` mask; dots at least half dark print black; then logo, barcode and QR pictures are copied in dot for dot. A printout is at most 4096 × 32,000 dots (`TooBig`).
- **ESC/POS:** the drawer pulse opens the drawer after the cut. A strip height of 0 counts as 1. Each picture strip is `GS v 0` with our rows as they are (they are already in the printer's format).
- **TSPL:** a 0 bit prints black (the opposite of ESC/POS), so the bits are flipped. Rows below the label's height are left out. `copies` of 0 prints one.

## Tests
| File | What it checks |
|---|---|
| `tests/golden.rs` | **Golden images:** receipt at 80 mm (`hw_03_…`) and 58 mm (`hw_02_…`) with logo and ৳; return (`ret_06_…`); void (`ret_04_…`); Z report at 80 and 58 mm (`shf_08_…`); labels at 58×40 and 80×50 mm (`cat_12_…`); shelf tag 80×40 mm with a promotion (`cat_13_…`). Compared dot for dot with the PNGs in `tests/golden/` |
| `tests/escpos_bytes.rs` | ESC/POS: header bytes, band splitting, the two-byte height, cut, drawer after the cut, drawer pulse bytes (HW-02, HW-04), with `insta` byte snapshots in `tests/snapshots/`; TSPL bytes, cropping and copies (HW-06) |
| `tests/drawing.rs` | Long product names wrap; a too-long word is broken; numbers right-aligned; centred text is centred; a 200-line receipt draws within 200 ms (`nfr_p4_…`; about 30 ms in a release build; unoptimised test builds get 600 ms) |
| `tests/layouts.rs` | What each layout prints: receipt header, lines, totals, payments (HW-02); tax summary and "Prices include VAT" (TAX-08); layout options (SET-14); void and return receipts (RET-04, RET-06); a reprint marked COPY (PAY-10); Z report (SHF-08); label and shelf tag fields (CAT-12, CAT-13); every printed word comes from `ReceiptLabels` (ADR-022) |
| `tests/contract.rs` | The public API: paper widths, picture width, empty doc, feed, size limits, huge column weights, ৳ ₹ € are drawn (HW-03), PNG preview matches the dots (HW-10), logo shrinking and dithering, errors into `AppError`, label sizes, drawer pulse and TSPL shapes |
| Unit tests | QR size and border; dots outside a picture; quantity and percent text; template filling |

**Changing a golden image:** a failing golden test saves the new picture as `tests/golden/<name>.new.png` (not committed) so you can compare. After the product owner approves it: `UPDATE_GOLDEN=1 cargo nextest run -p printing`. **Changing bytes:** `INSTA_UPDATE=always cargo nextest run -p printing` rewrites the snapshots; read every changed byte before committing.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/printing/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/printing/` |
| **Screen folder** | None of its own. Receipt previews are shown by the Settings and Hardware screens through `app` |
| **Requirements** | [[Req - Hardware and Printing]] HW-02 to HW-04, HW-06, HW-10; [[Req - Catalog]] CAT-12, CAT-13; [[Req - Shifts and Cash]] SHF-08; [[Req - Returns and Voids]] RET-04, RET-06; [[Req - Tax]] TAX-08; [[Req - Setup and Settings]] SET-14 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Receipt, report and label data built by `app`, `ReceiptLabels` (every printed word), barcode pictures, the logo file, the shop's currency format |
| **Used by** (re-run their tests when the public API changes) | Completing a sale (receipt), returns, voids, closing a shift (Z report), labels and shelf tags, test print, the preview printer, the logo preview in Settings |
| **Test it on its own** | `cargo nextest run -p printing` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Pure code: it never talks to devices. Sending bytes is `hardware`'s job.
- A change to a golden image needs the product owner's approval.
- Every printed word comes from `ReceiptLabels`, never typed into a layout ([[ADR-022 Ready for more languages later]]).
- Never load the computer's fonts (`FontSystem::new()`): golden images would then differ between machines. Fonts come only from `fonts/`, and the fallback list in `fonts.rs` is the same on every system.
- The `escpos` crate's `cash_drawer()` sends `ESC p m` without the two timing bytes. Don't use it; `drawer_pulse` writes the full command.
- Layouts never calculate money. If a printout needs a number that isn't in its data struct, add the field and let `app` fill it.
- `float_arithmetic` is only a warning in this crate (text drawing works in fractions of a dot), but `-D warnings` makes it an error. Convert to whole dots right away; money never touches floats.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- **Connected in C-M7:** `app::workflows::receipts` prints every sale's receipt (`receipt`), reprints with `copy: true`, and the X and Z reports (`shift_report`), drawing with `rasterize` and sending `escpos::encode` (drawer pulse with the first copy) or `drawer_pulse`. Golden pictures of real receipts and reports made by `app` are in `crates/app/tests/golden/`.
- Built in task B-printing. **Connected to `app` in C-M1:** "Test print" (`app::workflows::test_print`) draws a sample receipt with `receipt`, `rasterize` and `escpos::encode`; `app::receipt_words::english` fills `ReceiptLabels`; the Preview printer shows printouts with `to_png`. Its golden pictures are in `crates/app/tests/golden/` (not approved yet).
- **Golden images are not approved yet.** The pictures in `tests/golden/` were made in B-printing and are the current reference. C-M1 put the test receipt (same layout) on screen in the Preview printer for the product owner to approve; if they ask for changes, update the layouts and re-approve both sets.
- **Golden images on other systems:** they were made on Linux and match in debug and release builds. CI on Windows (and the Mac build) will show whether the font drawing is dot-for-dot the same there. If not, find the difference before re-approving anything.
- **Real printers (M13):** check on real hardware that (1) the `GS v 0` strips of 256 rows print without gaps, (2) the drawer opens with 50 ms / 500 ms pulses, (3) TSPL's flipped bits are right (0 = black) and the 203 dpi assumption holds (a 300 dpi label printer needs a `dots_per_mm` setting), (4) labels need a side margin if the label roll drifts (the layouts use the full label width).
- **Error words:** `logo_unreadable` and `qr_text_too_long` are in `ui/src/shared/text/en/errors.ts` (C-M1).
- `RUSTSEC-2026-0192`: `ttf-parser` (inside `fontdb` 0.23, inside `cosmic-text` 0.19) is unmaintained. It is ignored in `deny.toml` because it reads only our bundled fonts. Remove the ignore when a `cosmic-text` release drops it.
- Currency signs from other scripts (Thai ฿, Arabic ؋ ﷼, Khmer ៛) need a font cut like the taka one, added the same way, if a shop uses them.
- A4 label sheets and the system print dialog are not here: they go through the webview ([[ADR-013 A4 and PDF output through the webview]]).
