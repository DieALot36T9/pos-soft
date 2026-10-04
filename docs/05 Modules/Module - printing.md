---
tags: [module]
crate: printing
milestone: M1 (test), M7
---
# Module: printing

**Purpose:** turn data into printable bytes: a simple **document model** → layouts for receipts, X/Z reports, labels and shelf tags → a 1-bit **raster image** → **ESC/POS** or **TSPL** bytes. Pure. It never talks to devices.

**Requirements:** [[Req - Hardware and Printing]], [[Req - Catalog]] CAT-12/13. **Design:** [[ADR-012 Receipts printed as raster images]].
**Depends on:** `core`, `cosmic-text`, `tiny-skia`, `qrcode`, `image` (logo dithering). One font in `fonts/` (Noto Sans, open licence). Fonts for other languages can be added later ([[ADR-022 Ready for more languages later]]).

## Document model
```rust
pub struct Doc { pub width_dots: u32, pub blocks: Vec<Block> }
pub enum Block {
  Text { text: String, size: TextSize, bold: bool, align: Align },
  Row { left: String, right: String, size: TextSize, bold: bool },      // "Milk 1L ×2      180.00"
  Columns { cells: Vec<(String, u16 /*weight*/, Align)> },
  Rule { style: RuleStyle },                                            // ---- or ====
  Image { bitmap: Bitmap1, align: Align },                              // logo
  Barcode { bitmap: Bitmap1 },                                          // core::Bitmap1, produced by the barcode crate and passed in by app
  Qr { data: String, size_dots: u32 },
  Feed { dots: u32 },
}
```

## Layouts (data structs are owned here; `app` maps sales to them)
```rust
pub fn receipt(d: &ReceiptData, l: &ReceiptLayout, labels: &ReceiptLabels /*every printed word comes from here*/) -> Doc;
pub fn return_receipt(d: &ReturnData, l: &ReceiptLayout, labels: &ReceiptLabels) -> Doc;
pub fn shift_report(r: &ReportTable, l: &ReceiptLayout, labels: &ReceiptLabels) -> Doc;   // X/Z, daily summary
pub fn label(d: &LabelData, tpl: &LabelTemplate) -> Doc;
pub fn shelf_tag(d: &ShelfTagData, tpl: &TagTemplate) -> Doc;
```

## Rendering and encoding
```rust
pub struct Fonts;  pub fn load_fonts() -> Fonts;                          // once at startup
pub fn rasterize(doc: &Doc, fonts: &Fonts) -> Bitmap1;                    // cosmic-text shaping + tiny-skia, threshold to 1-bit
pub fn to_png(b: &Bitmap1) -> Vec<u8>;                                    // previews in UI and golden tests
pub mod escpos { pub fn encode(b: &Bitmap1, opts: &EscPosOpts /*cut, drawer pin, band height*/) -> Vec<u8>;
                 pub fn drawer_pulse(pin: DrawerPin) -> Vec<u8>; }
pub mod tspl   { pub fn encode(b: &Bitmap1, label: &LabelSize, copies: u32) -> Vec<u8>; }
```
Receipt previews in Settings use `to_png`, so **the preview is exactly what prints**.

## Tests
- **Golden images:** receipt (with logo and currency symbol), return, void, Z report, labels at 58 and 80 mm. Compared to approved PNGs in `tests/golden/` (approved by the product owner in M1/M7). Any change needs a re-approval.
- ESC/POS encoding: header bytes, band splitting, cut and drawer commands (byte snapshots with `insta`).
- Long product names wrap; numbers right-aligned; a 200-line receipt renders in ≤ 200 ms.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/printing/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/printing/` |
| **Screen folder** | None of its own. Receipt previews are shown by the Settings and Hardware screens through `app` |
| **Requirements** | [[Req - Hardware and Printing]] HW-02 to HW-04, HW-06, HW-10; [[Req - Catalog]] CAT-12, CAT-13; [[Req - Shifts and Cash]] SHF-08 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Receipt, report and label data built by `app`, `ReceiptLabels` (every printed word), barcode pictures, fonts |
| **Used by** (re-run their tests when the public API changes) | Completing a sale (receipt), returns, voids, closing a shift (Z report), labels and shelf tags, test print, the preview printer |
| **Test it on its own** | `cargo nextest run -p printing` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Pure code: it never talks to devices. Sending bytes is `hardware`'s job.
- A change to a golden image needs the product owner's approval.
- Every printed word comes from `ReceiptLabels`, never typed into a layout ([[ADR-022 Ready for more languages later]]).

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Not started (see [[Task Index]]).
