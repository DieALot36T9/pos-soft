//! Stock counts (INV-06): start a count, count by scanning, review the differences,
//! post them (after a `pre-count-post` backup) or cancel. Each change is one
//! transaction with its audit event ("Post stock count" in Workflows).

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};

use catalog::ProductPage;
use inventory::{CountEntry, CountRow, CountScope, CountStatus, NewCount, Page, VarianceRow};
use pos_core::{AppError, CurrencyFormat, Id, Money, Qty, Timestamp};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::audit_actions::{STOCK_COUNT_CANCEL, STOCK_COUNT_CREATE, STOCK_COUNT_POST};
use crate::audit_log;
use crate::state::AppState;
use crate::workflows::backup;
use crate::workflows::stock::{move_ctx, require_stocked, unit_of, user_names};

/// The biggest page of the count list.
const MAX_PAGE: u32 = 200;

/// A new count as typed on the "Start a count" form. A partial count may name a
/// category, a brand and a place; they are kept to show on the count, not to limit it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct CountInput {
    pub name: String,
    pub scope: CountScope,
    pub category_id: Option<Id>,
    pub brand_id: Option<Id>,
    /// Where in the shop. Example: "Aisle 3".
    pub location: Option<String>,
}

/// One count in the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CountSummary {
    pub id: Id,
    /// Example: "SC-000001".
    pub number: String,
    pub name: String,
    pub scope: CountScope,
    pub status: CountStatus,
    /// What a partial count covers. Example: "Dairy · Aisle 3".
    pub covers: String,
    pub created_at: Timestamp,
    pub created_by_name: Option<String>,
    pub posted_at: Option<Timestamp>,
    pub posted_by_name: Option<String>,
}

/// One counted line on the count screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CountedLine {
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub unit_symbol: String,
    pub unit_decimals: u8,
    pub batch_id: Option<Id>,
    pub batch_no: Option<String>,
    pub counted: Qty,
}

/// A count and what has been counted so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CountView {
    pub count: CountSummary,
    pub lines: Vec<CountedLine>,
}

/// One scan or typed quantity on the count screen. Counting the same product again adds
/// to it; a negative quantity takes back a wrong scan.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct CountLineInput {
    pub product_id: Id,
    /// Counts one batch instead of the whole product.
    pub batch_id: Option<Id>,
    pub qty: Qty,
}

/// One line of the variance report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct VarianceView {
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub unit_symbol: String,
    pub unit_decimals: u8,
    pub batch_id: Option<Id>,
    pub batch_no: Option<String>,
    pub expected: Qty,
    pub counted: Qty,
    /// Counted − expected. Negative means stock is missing.
    pub difference: Qty,
    /// The difference × the product's cost price.
    pub cost_value: Money,
    /// False for a product in a full count that nobody counted. Posting sets it to 0.
    pub was_counted: bool,
}

/// The variance report of a count (INV-06).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct VarianceReport {
    pub count: CountSummary,
    pub rows: Vec<VarianceView>,
    /// The sum of every row's cost value.
    pub total_value: Money,
    /// Products in a full count that nobody counted, but that hold stock.
    pub uncounted: u32,
    pub currency: CurrencyFormat,
}

/// What posting a count did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CountPosted {
    /// How many `count` ledger rows were written.
    pub rows_written: u32,
    /// The backup made just before posting.
    pub backup_file: String,
}

/// The counts, newest first.
pub fn list(state: &AppState, page: ProductPage) -> Result<Vec<CountSummary>, AppError> {
    state.db.read(|conn| {
        let page = Page {
            offset: page.offset,
            limit: page.limit.clamp(1, MAX_PAGE),
        };
        let names = user_names(conn)?;
        inventory::list_counts(conn, page)?
            .into_iter()
            .map(|count| summary(conn, &names, count))
            .collect()
    })
}

/// Starts a count and gives it the next number (SC-000001, …).
pub fn create(state: &AppState, by: &Id, input: &CountInput) -> Result<Id, AppError> {
    let clock = state.clock.as_ref();
    let now = clock.now();
    state.db.write(|tx| {
        if let Some(category) = &input.category_id {
            check_listed(
                catalog::list_categories(tx, true)?
                    .iter()
                    .any(|c| &c.id == category),
                "category",
                category,
            )?;
        }
        if let Some(brand) = &input.brand_id {
            check_listed(
                catalog::list_brands(tx, true)?
                    .iter()
                    .any(|b| &b.id == brand),
                "brand",
                brand,
            )?;
        }
        let location = input
            .location
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty());
        let scope_details = match input.scope {
            CountScope::Full => serde_json::json!({}),
            CountScope::Partial => serde_json::json!({
                "category_id": input.category_id,
                "brand_id": input.brand_id,
                "location": location,
            }),
        };
        let new = NewCount {
            name: input.name.clone(),
            scope: input.scope,
            scope_details,
        };
        let made = inventory::create_count(tx, &new, now, by)?;
        let details = serde_json::json!({
            "count_id": made.id,
            "number": made.number,
            "name": input.name.trim(),
            "scope": input.scope,
        });
        audit_log::record(tx, clock, STOCK_COUNT_CREATE, details, Some(by.clone()))?;
        Ok(made.id)
    })
}

/// A count and its counted lines, for the count screen.
pub fn view(state: &AppState, id: &Id) -> Result<CountView, AppError> {
    state.db.read(|conn| count_view(conn, id))
}

/// Adds one scan or typed quantity to an open count, and gives the count back.
pub fn count_line(
    state: &AppState,
    by: &Id,
    count: &Id,
    line: &CountLineInput,
) -> Result<CountView, AppError> {
    let now = state.clock.now();
    state.db.write(|tx| {
        let product = catalog::get(tx, &line.product_id)?;
        require_stocked(&product)?;
        let entry = CountEntry {
            product_id: line.product_id.clone(),
            batch_id: line.batch_id.clone(),
            qty: line.qty,
        };
        inventory::record_count_line(tx, count, &entry, now, by)?;
        count_view(tx, count)
    })
}

/// The variance report: expected, counted, difference and its cost value (INV-06).
pub fn variances(state: &AppState, id: &Id) -> Result<VarianceReport, AppError> {
    state.db.read(|conn| {
        let names = user_names(conn)?;
        let count = summary(conn, &names, inventory::get_count(conn, id)?)?;
        let costs = cost_prices(conn, id)?;
        let mut rows = Vec::new();
        let mut total_value = Money::zero();
        let mut uncounted = 0_u32;
        for row in inventory::count_variances(conn, id, &costs)? {
            total_value = total_value.checked_add(row.cost_value)?;
            if !row.was_counted {
                uncounted += 1;
            }
            rows.push(variance_view(conn, row)?);
        }
        Ok(VarianceReport {
            count,
            rows,
            total_value,
            uncounted,
            currency: settings::get(conn)?,
        })
    })
}

/// Posts an open count. A full count with products nobody counted needs
/// `confirm_uncounted`, because those products are set to 0 (INV-06). A backup is made
/// first (`pre-count-post`); then the `count` rows and the audit event are one transaction.
pub fn post(
    state: &AppState,
    by: &Id,
    id: &Id,
    confirm_uncounted: bool,
) -> Result<CountPosted, AppError> {
    let report = variances(state, id)?;
    if report.count.status != CountStatus::Open {
        return Err(AppError::validation("count_not_open"));
    }
    if report.uncounted > 0 && !confirm_uncounted {
        return Err(AppError::Conflict {
            code: "count_uncounted_needs_confirm".into(),
            params: BTreeMap::from([("uncounted".to_string(), report.uncounted.to_string())]),
        });
    }
    let backup = backup::make_backup(state, "pre-count-post")?;
    let clock = state.clock.as_ref();
    let rows_written = state.db.write(|tx| {
        let costs = cost_prices(tx, id)?;
        let written = inventory::post_count(tx, id, &costs, &move_ctx(tx, clock, by)?)?;
        let count = inventory::get_count(tx, id)?;
        let details = serde_json::json!({
            "count_id": id,
            "number": count.number,
            "name": count.name,
            "rows_written": written.len(),
            "uncounted_set_to_zero": report.uncounted,
            "value_minor": report.total_value.0,
        });
        audit_log::record(tx, clock, STOCK_COUNT_POST, details, Some(by.clone()))?;
        Ok(u32::try_from(written.len()).unwrap_or(u32::MAX))
    })?;
    Ok(CountPosted {
        rows_written,
        backup_file: backup.file,
    })
}

/// Cancels an open count. Nothing changes in stock.
pub fn cancel(state: &AppState, by: &Id, id: &Id) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        inventory::cancel_count(tx, id)?;
        let count = inventory::get_count(tx, id)?;
        let details = serde_json::json!({
            "count_id": id,
            "number": count.number,
            "name": count.name,
        });
        audit_log::record(tx, clock, STOCK_COUNT_CANCEL, details, Some(by.clone()))
    })
}

fn count_view(conn: &Connection, id: &Id) -> Result<CountView, AppError> {
    let names = user_names(conn)?;
    let count = summary(conn, &names, inventory::get_count(conn, id)?)?;
    let mut lines = Vec::new();
    for row in inventory::count_variances(conn, id, &HashMap::new())? {
        if !row.was_counted {
            continue;
        }
        let view = variance_view(conn, row)?;
        lines.push(CountedLine {
            product_id: view.product_id,
            name: view.name,
            sku: view.sku,
            unit_symbol: view.unit_symbol,
            unit_decimals: view.unit_decimals,
            batch_id: view.batch_id,
            batch_no: view.batch_no,
            counted: view.counted,
        });
    }
    Ok(CountView { count, lines })
}

fn variance_view(conn: &Connection, row: VarianceRow) -> Result<VarianceView, AppError> {
    let product = catalog::get(conn, &row.product_id)?;
    let (unit_symbol, unit_decimals) = unit_of(conn, &product.unit_id)?;
    let batch_no = match &row.batch_id {
        Some(batch) => inventory::get_batch(conn, batch)?.batch_no,
        None => None,
    };
    Ok(VarianceView {
        product_id: row.product_id,
        name: product.name,
        sku: product.sku,
        unit_symbol,
        unit_decimals,
        batch_id: row.batch_id,
        batch_no,
        expected: row.expected,
        counted: row.counted,
        difference: row.difference,
        cost_value: row.cost_value,
        was_counted: row.was_counted,
    })
}

/// The current cost price of every product in the count's report (from catalog).
fn cost_prices(conn: &Connection, id: &Id) -> Result<HashMap<Id, Money>, AppError> {
    let mut costs = HashMap::new();
    for row in inventory::count_variances(conn, id, &HashMap::new())? {
        if let Entry::Vacant(slot) = costs.entry(row.product_id) {
            let cost = catalog::get(conn, slot.key())?.cost_price;
            slot.insert(cost);
        }
    }
    Ok(costs)
}

fn summary(
    conn: &Connection,
    names: &HashMap<Id, String>,
    count: CountRow,
) -> Result<CountSummary, AppError> {
    Ok(CountSummary {
        covers: covers(conn, &count.scope_details)?,
        created_by_name: names.get(&count.created_by).cloned(),
        posted_by_name: count
            .posted_by
            .as_ref()
            .and_then(|id| names.get(id).cloned()),
        id: count.id,
        number: count.number,
        name: count.name,
        scope: count.scope,
        status: count.status,
        created_at: count.created_at,
        posted_at: count.posted_at,
    })
}

/// What a partial count covers, in words: "Dairy · Farm Fresh · Aisle 3".
fn covers(conn: &Connection, details: &serde_json::Value) -> Result<String, AppError> {
    let text = |key: &str| details.get(key).and_then(|value| value.as_str());
    let mut parts = Vec::new();
    if let Some(category) = text("category_id")
        && let Some(found) = catalog::list_categories(conn, true)?
            .into_iter()
            .find(|each| each.id.as_str() == category)
    {
        parts.push(found.name);
    }
    if let Some(brand) = text("brand_id")
        && let Some(found) = catalog::list_brands(conn, true)?
            .into_iter()
            .find(|each| each.id.as_str() == brand)
    {
        parts.push(found.name);
    }
    if let Some(location) = text("location") {
        parts.push(location.to_string());
    }
    Ok(parts.join(" · "))
}

fn check_listed(found: bool, entity: &str, id: &Id) -> Result<(), AppError> {
    if found {
        Ok(())
    } else {
        Err(AppError::NotFound {
            entity: entity.to_string(),
            key: id.to_string(),
        })
    }
}
