//! The import wizard (Req - Import and Export: IO-01, IO-02, IO-04, IO-05). A file is
//! read with `io`, its columns are matched to the fields of what is imported, and
//! every row is saved through the modules' own functions.
//!
//! The preview runs the whole import in a transaction that is then undone, so it
//! shows every error the real import would meet, with its row and column. The import
//! itself takes a `pre-import` backup first and saves everything in one transaction:
//! it all succeeds or nothing changes. Each row is saved inside a savepoint, so a row
//! the modules refuse can be skipped without losing the others.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use chrono::NaiveDate;
use io::{FieldSpec, Table};
use pos_core::{AppError, Id, Money, Qty};
use rusqlite::Transaction;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::audit_actions::DATA_IMPORT;
use crate::audit_log;
use crate::state::AppState;
use crate::workflows::{backup, import_contacts, import_products, import_stock};

/// How many rows the preview shows.
const PREVIEW_ROWS: usize = 50;

/// What a file holds (IO-01, IO-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ImportEntity {
    Products,
    Customers,
    Suppliers,
    /// Stock the shop has when it starts, saved as `opening` movements.
    OpeningStock,
    /// Prices for one price list.
    PriceListPrices,
    /// Extra barcodes for products that exist.
    Barcodes,
}

impl ImportEntity {
    /// The fields this entity takes from a file, in the order the template has them.
    pub(crate) fn fields(self) -> &'static [Field] {
        match self {
            ImportEntity::Products => import_products::FIELDS,
            ImportEntity::Customers => import_contacts::CUSTOMER_FIELDS,
            ImportEntity::Suppliers => import_contacts::SUPPLIER_FIELDS,
            ImportEntity::OpeningStock => import_stock::OPENING_FIELDS,
            ImportEntity::PriceListPrices => import_stock::PRICE_FIELDS,
            ImportEntity::Barcodes => import_stock::BARCODE_FIELDS,
        }
    }

    /// The name used in the audit log. Example: "opening_stock".
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            ImportEntity::Products => "products",
            ImportEntity::Customers => "customers",
            ImportEntity::Suppliers => "suppliers",
            ImportEntity::OpeningStock => "opening_stock",
            ImportEntity::PriceListPrices => "price_list_prices",
            ImportEntity::Barcodes => "barcodes",
        }
    }
}

/// How rows find the product they belong to (IO-01 step 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum MatchBy {
    Sku,
    Barcode,
}

/// Everything the wizard has chosen so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImportRequest {
    pub entity: ImportEntity,
    /// The full path of the .csv or .xlsx file.
    pub file: String,
    /// The .xlsx tab to read. Empty: the first one.
    pub sheet: Option<String>,
    /// One column number (from 0) per field, in the order of `ImportPreview.fields`.
    /// Empty: the columns are matched by their headers.
    pub mapping: Option<Vec<Option<u32>>>,
    pub match_by: MatchBy,
    /// Make categories, brands and units that don't exist yet (products only).
    pub create_missing: bool,
    /// Import the good rows and leave out the rows with errors.
    pub skip_errors: bool,
    /// The price list the prices go into (price list prices only).
    pub price_list_id: Option<Id>,
}

/// One field the import needs. The screen shows its name from `key`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ImportField {
    pub key: String,
    /// The header in templates and exports. Example: "Sale price".
    pub header: String,
    pub required: bool,
}

/// One bad cell. Example: row 812, "Sale price", `money_not_a_number`, "12,5O".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImportCellError {
    /// The row number in the file. The headers are row 1.
    pub row: u32,
    /// The column's header in the file.
    pub column: String,
    pub code: String,
    pub value: String,
}

/// What a row does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RowAction {
    Create,
    Update,
    /// Nothing to do: the row is already there, or it has errors.
    Skip,
}

/// One row of the preview, as it will be saved: one value per field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PreviewRow {
    pub row: u32,
    pub action: RowAction,
    pub cells: Vec<String>,
}

/// How many rows were made, changed and left out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ImportCounts {
    pub created: u32,
    pub updated: u32,
    pub skipped: u32,
}

/// The preview (IO-01 steps 2 to 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ImportPreview {
    /// The .xlsx tabs. Empty for a .csv file.
    pub sheets: Vec<String>,
    /// The file's column headers.
    pub headers: Vec<String>,
    pub fields: Vec<ImportField>,
    /// The column of each field, as used.
    pub mapping: Vec<Option<u32>>,
    /// Data rows in the file, blank ones included.
    pub total_rows: u32,
    /// The first 50 rows.
    pub rows: Vec<PreviewRow>,
    /// Every error. A required field without a column is an error on row 1.
    pub errors: Vec<ImportCellError>,
    pub counts: ImportCounts,
}

/// What the import did (IO-01 step 7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ImportDone {
    pub counts: ImportCounts,
    /// The backup taken just before.
    pub backup_file: String,
    /// The rows left out, for the error file.
    pub errors: Vec<ImportCellError>,
}

/// How far the running import is (IO-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ImportProgress {
    pub done: u32,
    pub total: u32,
    pub running: bool,
}

/// One line of the error file, with the problem already in words by the screen.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct ErrorLine {
    pub row: u32,
    pub column: String,
    pub value: String,
    pub problem: String,
}

/// The running import's progress and its cancel switch. One per app.
#[derive(Debug, Default)]
pub struct ImportJob {
    done: AtomicU32,
    total: AtomicU32,
    running: AtomicBool,
    cancel: AtomicBool,
}

impl ImportJob {
    pub fn progress(&self) -> ImportProgress {
        ImportProgress {
            done: self.done.load(Ordering::SeqCst),
            total: self.total.load(Ordering::SeqCst),
            running: self.running.load(Ordering::SeqCst),
        }
    }

    /// Asks the running import to stop. It stops at the next row and saves nothing.
    pub fn cancel(&self) {
        if self.running.load(Ordering::SeqCst) {
            self.cancel.store(true, Ordering::SeqCst);
        }
    }

    fn start(&self, total: usize) -> Result<(), AppError> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(conflict("import_running"));
        }
        self.cancel.store(false, Ordering::SeqCst);
        self.done.store(0, Ordering::SeqCst);
        self.total
            .store(u32::try_from(total).unwrap_or(u32::MAX), Ordering::SeqCst);
        Ok(())
    }

    fn step(&self) -> Result<(), AppError> {
        if self.cancel.load(Ordering::SeqCst) {
            return Err(conflict("import_cancelled"));
        }
        self.done.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn finish(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.cancel.store(false, Ordering::SeqCst);
    }
}

/// One field an entity takes from a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Field {
    pub key: &'static str,
    /// The header in templates and exports.
    pub header: &'static str,
    /// Other headers people use for it. Example: "MRP" for the sale price.
    pub other_names: &'static [&'static str],
    pub required: bool,
}

/// The fields of the screen.
pub fn field_list(entity: ImportEntity) -> Vec<ImportField> {
    entity
        .fields()
        .iter()
        .map(|field| ImportField {
            key: field.key.to_string(),
            header: field.header.to_string(),
            required: field.required,
        })
        .collect()
}

/// Reads the file and checks every row, saving nothing (IO-01 steps 2 to 5).
pub fn preview(
    state: &AppState,
    by: &Id,
    request: &ImportRequest,
) -> Result<ImportPreview, AppError> {
    let path = Path::new(&request.file);
    let sheets = io::sheet_names(path)?;
    let table = io::read_file(path, request.sheet.as_deref())?;
    let fields = request.entity.fields();
    let mapping = column_mapping(request, &table, fields);
    let mut preview = ImportPreview {
        sheets,
        headers: table.headers.clone(),
        fields: field_list(request.entity),
        mapping: mapping
            .iter()
            .map(|column| column.and_then(|index| u32::try_from(index).ok()))
            .collect(),
        total_rows: count(table.rows.len()),
        rows: Vec::new(),
        errors: Vec::new(),
        counts: ImportCounts::default(),
    };
    let unmapped = unmapped_fields(fields, &mapping);
    if !unmapped.is_empty() {
        preview.errors = unmapped;
        return Ok(preview);
    }
    let rows = Rows {
        table: &table,
        fields,
        mapping: &mapping,
    };
    let outcome = state
        .db
        .write_and_undo(|tx| save_rows(tx, state, by, request, &rows, None))?;
    preview.rows = outcome.shown;
    preview.errors = outcome.errors;
    preview.counts = outcome.counts;
    Ok(preview)
}

/// Imports the file: a backup first, then every row in one transaction (IO-01 step 6).
/// Refused while any row has an error, unless the rows with errors are to be skipped.
pub fn commit(state: &AppState, by: &Id, request: &ImportRequest) -> Result<ImportDone, AppError> {
    let path = Path::new(&request.file);
    let table = io::read_file(path, request.sheet.as_deref())?;
    let fields = request.entity.fields();
    let mapping = column_mapping(request, &table, fields);
    if let Some(missing) = unmapped_fields(fields, &mapping).first() {
        return Err(AppError::Validation {
            field: Some("mapping".into()),
            code: "import_field_not_mapped".into(),
            params: [("field".to_string(), missing.column.clone())].into(),
        });
    }
    let rows = Rows {
        table: &table,
        fields,
        mapping: &mapping,
    };
    let job = &state.import_job;
    job.start(table.rows.len())?;
    let result = commit_rows(state, by, request, &rows);
    job.finish();
    result
}

fn commit_rows(
    state: &AppState,
    by: &Id,
    request: &ImportRequest,
    rows: &Rows,
) -> Result<ImportDone, AppError> {
    let backup = backup::make_backup(state, "pre-import")?;
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let outcome = save_rows(tx, state, by, request, rows, Some(&state.import_job))?;
        if !outcome.errors.is_empty() && !request.skip_errors {
            return Err(AppError::Validation {
                field: None,
                code: "import_has_errors".into(),
                params: [("count".to_string(), outcome.errors.len().to_string())].into(),
            });
        }
        let file_name = Path::new(&request.file)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let details = serde_json::json!({
            "entity": request.entity.as_str(),
            "file": file_name,
            "created": outcome.counts.created,
            "updated": outcome.counts.updated,
            "skipped": outcome.counts.skipped,
            "backup": backup.file,
        });
        audit_log::record(tx, clock, DATA_IMPORT, details, Some(by.clone()))?;
        Ok(ImportDone {
            counts: outcome.counts,
            backup_file: backup.file.clone(),
            errors: outcome.errors,
        })
    })
}

/// Writes the error file (IO-01 step 7): row, column, value and the problem in words.
pub fn save_error_file(lines: &[ErrorLine], file: &Path) -> Result<(), AppError> {
    let table = Table {
        headers: ["Row", "Column", "Value", "Problem"]
            .iter()
            .map(|header| header.to_string())
            .collect(),
        rows: lines
            .iter()
            .map(|line| {
                vec![
                    line.row.to_string(),
                    line.column.clone(),
                    line.value.clone(),
                    line.problem.clone(),
                ]
            })
            .collect(),
    };
    Ok(io::write_csv(file, &table)?)
}

/// The chosen columns, or else the columns matched by their headers.
fn column_mapping(request: &ImportRequest, table: &Table, fields: &[Field]) -> Vec<Option<usize>> {
    match &request.mapping {
        Some(chosen) => (0..fields.len())
            .map(|index| {
                chosen
                    .get(index)
                    .copied()
                    .flatten()
                    .and_then(|column| usize::try_from(column).ok())
                    .filter(|column| *column < table.headers.len())
            })
            .collect(),
        None => {
            let specs: Vec<FieldSpec> = fields
                .iter()
                .map(|field| {
                    let mut names = vec![field.header];
                    names.extend_from_slice(field.other_names);
                    FieldSpec::new(field.key, &names)
                })
                .collect();
            io::auto_map(&table.headers, &specs)
        }
    }
}

/// A row-1 error for each required field without a column.
fn unmapped_fields(fields: &[Field], mapping: &[Option<usize>]) -> Vec<ImportCellError> {
    fields
        .iter()
        .zip(mapping)
        .filter(|(field, column)| field.required && column.is_none())
        .map(|(field, _)| ImportCellError {
            row: 1,
            column: field.header.to_string(),
            code: "column_not_mapped".into(),
            value: String::new(),
        })
        .collect()
}

/// The file's rows with their fields' columns.
pub(crate) struct Rows<'a> {
    table: &'a Table,
    fields: &'static [Field],
    mapping: &'a [Option<usize>],
}

/// What the rows did.
struct Outcome {
    counts: ImportCounts,
    shown: Vec<PreviewRow>,
    errors: Vec<ImportCellError>,
}

/// Saves each row in its own savepoint. A row with errors is undone and listed.
fn save_rows(
    tx: &Transaction,
    state: &AppState,
    by: &Id,
    request: &ImportRequest,
    rows: &Rows,
    job: Option<&ImportJob>,
) -> Result<Outcome, AppError> {
    let mut outcome = Outcome {
        counts: ImportCounts::default(),
        shown: Vec::new(),
        errors: Vec::new(),
    };
    let ctx = RowCtx {
        by,
        now: state.clock.now(),
        request,
    };
    let mut saver = Saver::new(tx, state, request, by)?;
    for (index, cells) in rows.table.rows.iter().enumerate() {
        if let Some(job) = job {
            job.step()?;
        }
        if cells.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }
        let row_number = count(index + 2);
        let mut check = RowCheck {
            fields: rows.fields,
            cells: rows
                .mapping
                .iter()
                .map(|column| {
                    column
                        .and_then(|column| cells.get(column))
                        .map(|cell| cell.trim().to_string())
                        .unwrap_or_default()
                })
                .collect(),
            shown: vec![String::new(); rows.fields.len()],
            errors: Vec::new(),
        };
        tx.execute_batch("SAVEPOINT import_row")
            .map_err(sql_error)?;
        let result = saver.save(tx, &ctx, &mut check);
        let action = match result {
            Ok(action) => {
                tx.execute_batch("RELEASE import_row").map_err(sql_error)?;
                action
            }
            Err(error) => {
                tx.execute_batch("ROLLBACK TO import_row; RELEASE import_row")
                    .map_err(sql_error)?;
                saver.after_undo(tx)?;
                if check.errors.is_empty() {
                    check.module_error(error)?;
                }
                for (field, code) in &check.errors {
                    outcome
                        .errors
                        .push(cell_error(rows, &check, row_number, field, code));
                }
                RowAction::Skip
            }
        };
        match action {
            RowAction::Create => outcome.counts.created += 1,
            RowAction::Update => outcome.counts.updated += 1,
            RowAction::Skip => outcome.counts.skipped += 1,
        }
        if outcome.shown.len() < PREVIEW_ROWS {
            outcome.shown.push(PreviewRow {
                row: row_number,
                action,
                cells: check.shown,
            });
        }
    }
    Ok(outcome)
}

/// The entity's saver, with the names it has looked up so far.
enum Saver {
    Products(import_products::Lookups),
    Customers(import_contacts::CustomerLookups),
    Suppliers(import_contacts::SupplierLookups),
    Stock(import_stock::Lookups),
}

impl Saver {
    fn new(
        tx: &Transaction,
        state: &AppState,
        request: &ImportRequest,
        by: &Id,
    ) -> Result<Saver, AppError> {
        Ok(match request.entity {
            ImportEntity::Products => Saver::Products(import_products::Lookups::load(tx)?),
            ImportEntity::Customers => {
                Saver::Customers(import_contacts::CustomerLookups::load(tx)?)
            }
            ImportEntity::Suppliers => {
                Saver::Suppliers(import_contacts::SupplierLookups::load(tx)?)
            }
            ImportEntity::OpeningStock | ImportEntity::PriceListPrices | ImportEntity::Barcodes => {
                Saver::Stock(import_stock::Lookups::load(tx, state, request, by)?)
            }
        })
    }

    /// A row was undone, so names it made (a new category, say) are gone again.
    fn after_undo(&mut self, tx: &Transaction) -> Result<(), AppError> {
        if let Saver::Products(lookups) = self {
            *lookups = import_products::Lookups::load(tx)?;
        }
        Ok(())
    }

    fn save(
        &mut self,
        tx: &Transaction,
        ctx: &RowCtx,
        check: &mut RowCheck,
    ) -> Result<RowAction, AppError> {
        match self {
            Saver::Products(lookups) => import_products::save_row(tx, ctx, lookups, check),
            Saver::Customers(lookups) => import_contacts::save_customer(tx, ctx, lookups, check),
            Saver::Suppliers(lookups) => import_contacts::save_supplier(tx, ctx, lookups, check),
            Saver::Stock(lookups) => match ctx.request.entity {
                ImportEntity::PriceListPrices => import_stock::save_price(tx, ctx, lookups, check),
                ImportEntity::Barcodes => import_stock::save_barcode(tx, ctx, lookups, check),
                _ => import_stock::save_opening(tx, ctx, lookups, check),
            },
        }
    }
}

/// Who imports, when, and what was chosen.
pub(crate) struct RowCtx<'a> {
    pub by: &'a Id,
    pub now: pos_core::Timestamp,
    pub request: &'a ImportRequest,
}

/// One row's cells by field, the values to show in the preview and the errors found.
pub(crate) struct RowCheck {
    fields: &'static [Field],
    cells: Vec<String>,
    shown: Vec<String>,
    errors: Vec<(String, String)>,
}

impl RowCheck {
    fn position(&self, key: &str) -> Option<usize> {
        self.fields.iter().position(|field| field.key == key)
    }

    /// The trimmed cell text. Empty when the field has no column.
    pub fn cell(&self, key: &str) -> &str {
        self.position(key)
            .and_then(|index| self.cells.get(index))
            .map(String::as_str)
            .unwrap_or("")
    }

    /// The value to show for this field in the preview.
    pub fn show(&mut self, key: &str, value: impl Into<String>) {
        if let Some(index) = self.position(key) {
            self.shown[index] = value.into();
        }
    }

    /// Notes an error on this field.
    pub fn fail(&mut self, key: &str, code: &str) {
        self.errors.push((key.to_string(), code.to_string()));
    }

    /// The text, or `None` when the cell is empty. Shown as it is.
    pub fn text(&mut self, key: &str) -> Option<String> {
        let text = self.cell(key).to_string();
        self.show(key, text.clone());
        (!text.is_empty()).then_some(text)
    }

    /// The text of a field that must be filled.
    pub fn required(&mut self, key: &str) -> String {
        let text = self.text(key);
        if text.is_none() {
            self.fail(key, "cell_empty");
        }
        text.unwrap_or_default()
    }

    /// Money with the currency's decimals (IO-04). `None` when empty or wrong.
    pub fn money(&mut self, key: &str, minor_digits: u8) -> Option<Money> {
        let text = self.cell(key).to_string();
        if text.is_empty() {
            return None;
        }
        match io::parse_money(&text, minor_digits) {
            Ok(amount) => {
                self.show(key, io::decimal_text(amount.0, minor_digits));
                Some(amount)
            }
            Err(code) => {
                self.show(key, text);
                self.fail(key, &code);
                None
            }
        }
    }

    /// A quantity with at most the unit's decimals (IO-04).
    pub fn qty(&mut self, key: &str, decimals: u8) -> Option<Qty> {
        let text = self.cell(key).to_string();
        if text.is_empty() {
            return None;
        }
        self.show(key, text.clone());
        match io::parse_qty(&text, decimals) {
            Ok(qty) => Some(qty),
            Err(code) => {
                self.fail(key, &code);
                None
            }
        }
    }

    /// yes/no, true/false or 1/0 (IO-04).
    pub fn flag(&mut self, key: &str) -> Option<bool> {
        let text = self.cell(key).to_string();
        if text.is_empty() {
            return None;
        }
        match io::parse_bool(&text) {
            Ok(value) => {
                self.show(key, yes_no(value));
                Some(value)
            }
            Err(code) => {
                self.show(key, text);
                self.fail(key, &code);
                None
            }
        }
    }

    pub fn date(&mut self, key: &str) -> Option<NaiveDate> {
        let text = self.cell(key).to_string();
        if text.is_empty() {
            return None;
        }
        match io::parse_date(&text) {
            Ok(date) => {
                self.show(key, date.to_string());
                Some(date)
            }
            Err(code) => {
                self.show(key, text);
                self.fail(key, &code);
                None
            }
        }
    }

    /// A whole number of 0 or more. Example: payment terms in days.
    pub fn whole(&mut self, key: &str) -> Option<u32> {
        let text = self.cell(key).to_string();
        if text.is_empty() {
            return None;
        }
        self.show(key, text.clone());
        match text.parse::<u32>() {
            Ok(number) => Some(number),
            Err(_) => {
                self.fail(key, "number_not_whole");
                None
            }
        }
    }

    /// Stops the row when any cell had an error, before anything is written.
    pub fn ready(&self) -> Result<(), AppError> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(AppError::validation("row_has_errors"))
        }
    }

    /// Turns a module's refusal into an error on the field it is about. Bugs and
    /// device errors stop the whole import.
    fn module_error(&mut self, error: AppError) -> Result<(), AppError> {
        let (field, code) = match &error {
            AppError::Validation { field, code, .. } => (field.clone(), code.clone()),
            AppError::Conflict { code, .. } => (None, code.clone()),
            AppError::NotFound { entity, .. } => {
                (Some(entity.clone()), format!("{entity}_not_found"))
            }
            _ => return Err(error),
        };
        let key = self.field_for(field.as_deref(), &code);
        self.fail(&key, &code);
        Ok(())
    }

    /// The field an error names, or whose name starts its code ("sku_in_use" → "sku").
    fn field_for(&self, field: Option<&str>, code: &str) -> String {
        let stem = |text: &str| -> String {
            text.split('_')
                .next()
                .unwrap_or("")
                .trim_end_matches('s')
                .to_string()
        };
        let named = field.and_then(|field| {
            self.fields
                .iter()
                .find(|each| each.key == field || stem(each.key) == stem(field))
        });
        let by_code = || self.fields.iter().find(|each| stem(each.key) == stem(code));
        named
            .or_else(by_code)
            .or(self.fields.first())
            .map(|each| each.key.to_string())
            .unwrap_or_default()
    }
}

fn cell_error(rows: &Rows, check: &RowCheck, row: u32, key: &str, code: &str) -> ImportCellError {
    let position = check.position(key);
    let column = position
        .and_then(|index| rows.mapping.get(index).copied().flatten())
        .and_then(|column| rows.table.headers.get(column).cloned())
        .or_else(|| position.map(|index| rows.fields[index].header.to_string()))
        .unwrap_or_default();
    ImportCellError {
        row,
        column,
        code: code.to_string(),
        value: check.cell(key).to_string(),
    }
}

/// "yes" or "no", as files and the preview write it.
pub(crate) fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

/// Lower-case with single spaces, for matching names typed in a file.
pub(crate) fn name_key(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn count(rows: usize) -> u32 {
    u32::try_from(rows).unwrap_or(u32::MAX)
}

fn conflict(code: &str) -> AppError {
    AppError::Conflict {
        code: code.into(),
        params: Default::default(),
    }
}

pub(crate) fn sql_error(error: rusqlite::Error) -> AppError {
    AppError::internal(format!("import: {error}"))
}
