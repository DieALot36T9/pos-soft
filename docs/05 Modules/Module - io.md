---
tags: [module]
crate: io
milestone: M12 (report export in M11)
---
# Module: io

**Purpose:** generic reading and writing of CSV and Excel files, and parsing of cell values. **It knows nothing about products or customers.** Mapping rows to module inputs happens in `app/src/workflows/import_*.rs`.

**Requirements:** [[Req - Import and Export]], [[Req - Reports]] RPT-09. **Depends on:** `core`, `csv` (CSV, RFC 4180), `calamine` (reads .xlsx), `rust_xlsxwriter` (writes .xlsx), `chrono` (`NaiveDate`), `thiserror`. Tests also use `tempfile`.

**Tables and views:** none. `io` works on files only and never touches the database, so it has no migrations.

## Public API
```rust
pub struct Table { pub headers: Vec<String>, pub rows: Vec<Vec<String>> }   // raw text cells, row number in the file = index + 2
pub fn read_file(path: &Path, sheet: Option<&str>) -> Result<Table>;        // .csv (guess , ; or tab, strip BOM, must be UTF-8) / .xlsx (first or named sheet)
pub fn sheet_names(path: &Path) -> Result<Vec<String>>;                     // .xlsx tab names in order; a .csv has none
pub fn write_csv(path: &Path, t: &Table) -> Result<()>;                     // UTF-8 BOM, CRLF, formula guard
pub fn write_xlsx(path: &Path, sheets: &[XlsxSheet]) -> Result<()>;         // typed cells, bold header row
pub fn report_to_xlsx(path: &Path, r: &ReportTable, header: &ExportHeader) -> Result<()>;

pub struct XlsxSheet { pub name: String, pub headers: Vec<String>, pub rows: Vec<Vec<XlsxCell>> }  // name made safe for Excel
pub enum XlsxCell { Empty, Text(String), Int(i64), Money { amount: Money, minor_digits: u8 }, Qty(Qty), Percent(Bps), Date(NaiveDate) }
pub struct ExportHeader { pub shop_name: String, pub generated_at: NaiveDateTime /* shop local time */, pub money_minor_digits: u8, pub labels: ExportLabels }
pub struct ExportLabels { pub period: String, pub period_to: String, pub generated: String, pub total: String }  // Default = English

// Mapping helpers
pub struct FieldSpec { pub key: String, pub names: Vec<String> }            // FieldSpec::new("sale_price", &["Price", "Sale price", "MRP"])
pub fn auto_map(headers: &[String], fields: &[FieldSpec]) -> Vec<Option<usize>>;  // one column index (or None) per field
pub struct CellErr { pub row: usize, pub column: String, pub code: String, pub value: String }
impl CellErr { pub fn check_column(t: &Table, column: usize, check: impl Fn(&str) -> Result<(), String>) -> Vec<CellErr>; }
pub fn parse_money(cell: &str, minor_digits: u8) -> Result<Money, String>;
pub fn parse_qty(cell: &str, decimals: u8) -> Result<Qty, String>;          // decimals 0..=3 (the unit's decimals)
pub fn parse_bool(cell: &str) -> Result<bool, String>;                        // yes/no/true/false/1/0, any letter case
pub fn parse_date(cell: &str) -> Result<NaiveDate, String>;                   // 2026-03-01, 01/03/2026 (day first), Excel day number

pub enum IoError { UnknownFileType, FileUnreadable, FileNotUtf8, FileEmpty, SheetNotFound, FileNotSaved }  // → AppError::Validation
pub type Result<T> = std::result::Result<T, IoError>;
```

### Error codes
| Where | Codes |
|---|---|
| Files (`IoError` → `AppError::Validation`) | `io_unknown_file_type` (param `extension`), `io_file_unreadable`, `io_file_not_utf8`, `io_file_empty`, `io_sheet_not_found` (param `sheet`), `io_file_not_saved` |
| `parse_money` | `money_empty`, `money_not_a_number`, `money_has_separator`, `money_too_many_decimals`, `money_too_large` |
| `parse_qty` | `qty_empty`, `qty_not_a_number`, `qty_has_separator`, `qty_too_many_decimals`, `qty_too_large` |
| `parse_bool` | `bool_empty`, `bool_not_yes_no` |
| `parse_date` | `date_empty`, `date_not_a_date` |

The UI turns each code into plain English. The file errors keep the library's message in a `detail` for the log only.

### How it behaves
- **Reading CSV:** the separator is whichever of comma, semicolon or tab appears most in the header line (outside quotes; a tie goes to the comma). Short rows are filled with empty cells up to the number of headers. Blank rows at the end are dropped; blank rows in the middle stay, so row numbers still match the file. Headers are trimmed; cells are not (the parse functions trim).
- **Reading XLSX:** every cell becomes the text a person would type. Numbers use their shortest exact form (`12.5`, `40`), dates become `2026-03-01` (with ` HH:MM:SS` if they have a time), errors become `#N/A` and so on. Numbers stored as text stay as they are. The first used row is the header row.
- **Formula guard (CSV only):** a cell that starts with `=` `+` `-` `@`, a tab or a carriage return, and is not a plain number like `-12.50` or `+8801711000000`, is written with a leading apostrophe (OWASP "CSV Injection"). `read_file` takes that apostrophe off again, so an export reads back unchanged (IO-03). In .xlsx files text is always stored as text, so Excel never runs it as a formula and no apostrophe is needed.
- **Writing XLSX:** money is a real number with the currency's decimals (format `#,##0.00`), quantities use `#,##0.000`, percentages `0.00%`, dates `yyyy-mm-dd`. Numbers are made from their exact decimal text, never with floating-point maths. Sheet names have `[ ] : * ? / \` replaced, apostrophes at the ends removed, and are cut to 31 characters.
- **Report export (RPT-09):** one tab named after the report. Lines: shop name, report title, `Period: <from> to <to>` (left out with no dates), `Generated: <time>`, one `<name>: <value>` line per filter. Then a blank row, the column titles (bold), the rows, the totals (bold; an empty first cell gets the `Total` label), then each section after a blank row with its title. The words come from `ExportLabels`, so other languages can be added later (ADR-022).
- **Mapping:** `auto_map` compares headers with each field's key and names, ignoring case, spaces and punctuation (`Sale-price` = `sale price`). Each column is used once; fields earlier in the list choose first.

## Tests
- `tests/contract.rs` (IO-01, IO-04): CSV with BOM, semicolons, quoted newlines, short and blank rows; XLSX with numbers stored as numbers and as text, dates; first or chosen sheet; missing sheet, unknown type, missing file, empty file; `auto_map`; `CellErr`; money parsing rejects extra decimals and ambiguous thousands separators; quantities, yes/no and dates; errors become validation errors.
- `tests/export.rs` (IO-03, RPT-09): round trip write → read gives the same table (including symbols like ৳ and cells that look like formulas) for CSV and XLSX; CSV starts with a BOM; the formula guard; XLSX numbers are real numbers and text is never a formula; safe sheet names; report header lines, real numbers, totals, sections and labels.
- `tests/size.rs` (IO-05): 50k-row read ≤ 5 s for CSV and for XLSX. Test builds are not optimised, so they allow 10 s; `cargo nextest run -p io --release` checks the real 5 s limit (about 0.2 s for CSV and 1.3 s for XLSX on the cloud test machine).
- Unit tests: delimiter guessing, file types, the guard round trip, Excel day numbers around the 1900 mistake, decimal text, sheet names.

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
- `io` knows nothing about products or customers. Mapping rows to them is done in `app/src/workflows/import_*.rs`. The header names for each field (e.g. "MRP" for sale price) come from `app` in `FieldSpec`.
- Never round imported numbers silently. Too many decimals is an error. This includes an Excel cell that holds a calculated value such as `13.200000000000001`: it reads as that text and `parse_money` refuses it.
- Keep the export guard against spreadsheet formulas (cells starting with = + - @). `read_file` must keep taking the apostrophe off again, or exports stop round-tripping.
- Plain numbers are not guarded, so Excel may still show `+8801711000000` as a number when it opens a CSV. That is a display issue, not a safety one; the file itself is unchanged.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Built in task B-io (Rust only). The screens (`ui/src/modules/io/`) and the `app` import/export workflows come in Phase C.
- [[Workflows#Import products]] calls the read function `io::read_table`; the code (and this page) call it `read_file`. Fix the word in Workflows when the import workflow is built.
- An .xlsx sheet whose header row is not row 1 (blank rows above it) is read from its first used row, so row numbers in errors are off by the number of blank rows above. Templates always start at row 1. Revisit if shops send such files.
- If two `XlsxSheet` names become the same after they are made safe, `write_xlsx` fails with `io_file_not_saved`. Callers pass distinct names today.
- No types here are sent to the UI yet, so none derive `ts-rs`. Add the derives when the Phase C screens need them (probably `CellErr` and the mapping result).
