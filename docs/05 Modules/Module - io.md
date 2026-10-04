---
tags: [module]
crate: io
milestone: M12 (report export in M11)
---
# Module: io

**Purpose:** generic reading and writing of CSV and Excel files, and parsing of cell values. **It knows nothing about products or customers.** Mapping rows to module inputs happens in `app/src/workflows/import_*.rs`.

**Requirements:** [[Req - Import and Export]], [[Req - Reports]] RPT-09. **Depends on:** `core`, `csv`, `calamine`, `rust_xlsxwriter`.

## Public API
```rust
pub struct Table { pub headers: Vec<String>, pub rows: Vec<Vec<String>> }   // raw text cells, row numbers = index + 2
pub fn read_file(path: &Path, sheet: Option<&str>) -> Result<Table>;        // .csv (detect delimiter, strip BOM) / .xlsx
pub fn sheet_names(path: &Path) -> Result<Vec<String>>;
pub fn write_csv(path: &Path, t: &Table) -> Result<()>;                     // UTF-8 BOM, formula-injection guard
pub fn write_xlsx(path: &Path, sheets: &[XlsxSheet]) -> Result<()>;         // typed cells: text, number, money (number format), date
pub fn report_to_xlsx(path: &Path, r: &ReportTable, header: &ExportHeader) -> Result<()>;

// Mapping helpers
pub fn auto_map(headers: &[String], fields: &[FieldSpec]) -> Vec<Option<usize>>;  // matches common header names, e.g. "Price" or "MRP"
pub struct CellErr { pub row: usize, pub column: String, pub code: String, pub value: String }
pub fn parse_money(cell: &str, minor_digits: u8) -> Result<Money, String>;
pub fn parse_qty(cell: &str, decimals: u8) -> Result<Qty, String>;
pub fn parse_bool(cell: &str) -> Result<bool, String>;                        // yes/no/true/false/1/0
pub fn parse_date(cell: &str) -> Result<NaiveDate, String>;                   // ISO and dd/mm/yyyy; Excel serial dates
```

## Tests
- CSV with BOM, semicolons, quoted newlines; XLSX with numbers stored as numbers and as text.
- Money parsing rejects extra decimals and thousands separators that are ambiguous.
- Round trip: write → read gives the same table (including symbols like ৳).
- 50k-row read ≤ 5 s (benchmark).

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/io/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/io/` |
| **Screen folder** | `ui/src/modules/io/` |
| **Requirements** | [[Req - Import and Export]] IO-01 to IO-05; [[Req - Reports]] RPT-09 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | File paths and the list of fields to map (from `app`) |
| **Used by** (re-run their tests when the public API changes) | Import and export workflows, report export to Excel |
| **Test it on its own** | `cargo nextest run -p io` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- `io` knows nothing about products or customers. Mapping rows to them is done in `app/src/workflows/import_*.rs`.
- Never round imported numbers silently. Too many decimals is an error.
- Keep the export guard against spreadsheet formulas (cells starting with = + - @).

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Not started (see [[Task Index]]).
