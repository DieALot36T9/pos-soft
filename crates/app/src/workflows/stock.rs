//! Stock on the back office (Req - Inventory): the stock levels list, one product's
//! stock and batches, its history, adjustments, low-stock and near-expiry alerts, and
//! the "Verify stock data" check. Each change is one transaction with its audit event
//! ("Adjust stock" in Workflows).

use std::collections::HashMap;

use catalog::{Product, ProductKind, ProductPage};
use chrono::{Days, NaiveDate};
use inventory::{BatchChoice, HistoryFilter, MoveCtx, MoveKind, Movement, Page, Reason};
use pos_core::{AppError, BusinessDate, Clock, CurrencyFormat, Id, Money, Qty, mul_qty};
use rusqlite::{Connection, Transaction, named_params};
use serde::{Deserialize, Serialize};
use settings::{InventorySettings, TillSettings};
use ts_rs::TS;

use crate::audit_actions::STOCK_ADJUST;
use crate::audit_log;
use crate::state::AppState;

/// The biggest page the stock lists return at once.
const MAX_PAGE: u32 = 500;
/// How many rows each dashboard alert list shows.
const ALERT_ROWS: u32 = 5;

/// Which products the stock levels list shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum StockShow {
    /// Every product that keeps stock.
    #[default]
    All,
    /// Below the reorder level (INV-03).
    Low,
    /// Below zero (INV-04).
    Negative,
    /// With a batch that expires within the near-expiry days, or has expired (INV-05).
    Expiring,
}

/// The filters on the stock levels list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StockFilter {
    /// Part of the name or SKU. Example: "mil".
    pub text: Option<String>,
    /// Products in this category or any category below it.
    pub category_id: Option<Id>,
    pub show: StockShow,
}

/// One row of the stock levels list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct StockRow {
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub category_name: String,
    pub unit_symbol: String,
    pub unit_decimals: u8,
    pub on_hand: Qty,
    pub reorder_level: Option<Qty>,
    pub cost_price: Money,
    /// On hand × cost price (INV-08).
    pub value: Money,
    pub low: bool,
    pub negative: bool,
    pub expiring: bool,
}

/// One page of the stock levels list, with the count and value of every matching row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct StockList {
    pub rows: Vec<StockRow>,
    #[ts(type = "number")]
    pub total: u64,
    pub total_value: Money,
    /// How the shop writes money, for the value column.
    pub currency: CurrencyFormat,
}

/// One batch of a product, as the stock panel shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct BatchView {
    pub id: Id,
    pub batch_no: Option<String>,
    pub expiry_date: Option<NaiveDate>,
    pub on_hand: Qty,
    pub unit_cost: Money,
    pub received_at: pos_core::Timestamp,
    /// The expiry date has passed. A batch is still good on its expiry date.
    pub expired: bool,
    /// Expires within the near-expiry days (or has expired).
    pub near_expiry: bool,
}

/// One product's stock: what the stock panel, the adjust form and the count screen need.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct StockProduct {
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub unit_symbol: String,
    pub unit_decimals: u8,
    pub track_expiry: bool,
    pub on_hand: Qty,
    pub reorder_level: Option<Qty>,
    pub reorder_qty: Option<Qty>,
    pub cost_price: Money,
    /// Batches that hold stock, first expiry first.
    pub batches: Vec<BatchView>,
}

/// Which batch an adjustment uses (INV-02, INV-05).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum AdjustBatch {
    /// Stock out: first expiry, first out. Stock in: no batch.
    Auto,
    /// This batch of the product.
    Existing { id: Id },
    /// A new batch for stock coming in. A product that tracks expiry needs the date.
    New {
        batch_no: Option<String>,
        expiry_date: Option<NaiveDate>,
    },
}

/// A stock adjustment as typed on the adjust form.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct AdjustInput {
    pub product_id: Id,
    /// Signed: negative takes stock out. Example: −2000 is 2 pieces out.
    pub qty_delta: Qty,
    pub reason_code: String,
    pub note: Option<String>,
    pub batch: AdjustBatch,
}

/// Which history rows to show (INV-07). Empty `kinds` means every kind.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct HistoryQuery {
    pub product_id: Id,
    pub from: Option<BusinessDate>,
    pub to: Option<BusinessDate>,
    pub kinds: Vec<MoveKind>,
    pub page: ProductPage,
}

/// One row of a product's stock history, with the stock right after it (INV-07).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct HistoryRow {
    pub id: Id,
    pub at: pos_core::Timestamp,
    pub business_date: BusinessDate,
    pub kind: MoveKind,
    pub qty_delta: Qty,
    pub balance_after: Qty,
    pub batch_no: Option<String>,
    pub batch_expiry: Option<NaiveDate>,
    pub reason_name: Option<String>,
    pub note: Option<String>,
    /// The document behind the change. Example: "count".
    pub ref_type: Option<String>,
    /// The document's number when the app knows it. Example: "SC-000001".
    pub ref_label: Option<String>,
    pub unit_cost: Money,
    pub user_name: Option<String>,
}

/// One product below its reorder level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct LowStockView {
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub unit_symbol: String,
    pub unit_decimals: u8,
    pub on_hand: Qty,
    pub reorder_level: Qty,
    pub reorder_qty: Qty,
}

/// One batch that expires soon or has expired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct NearExpiryView {
    pub batch_id: Id,
    pub product_id: Id,
    pub name: String,
    pub unit_symbol: String,
    pub unit_decimals: u8,
    pub batch_no: Option<String>,
    pub expiry_date: NaiveDate,
    pub on_hand: Qty,
    pub expired: bool,
}

/// The stock alerts on the dashboard (INV-03, INV-04, INV-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct StockAlerts {
    pub low_stock_count: u32,
    /// The biggest shortfalls first, at most 5.
    pub low_stock: Vec<LowStockView>,
    pub near_expiry_count: u32,
    /// Soonest first, at most 5.
    pub near_expiry: Vec<NearExpiryView>,
    pub negative_count: u32,
    pub near_expiry_days: u32,
}

/// A stock level that doesn't match its ledger (INV-10). Reported, never fixed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct MismatchView {
    pub product_id: Id,
    pub name: String,
    pub batch_id: Option<Id>,
    pub batch_no: Option<String>,
    pub cached: Qty,
    pub ledger_sum: Qty,
}

/// One page of the stock levels list (INV-01, INV-03, INV-04, INV-05, INV-08). Only
/// standard products and variants that keep stock; archived ones while they hold stock.
pub fn levels(
    state: &AppState,
    filter: &StockFilter,
    page: ProductPage,
) -> Result<StockList, AppError> {
    let today = today(state)?;
    state.db.read(|conn| {
        let options: InventorySettings = settings::get(conn)?;
        let soon = expiry_limit(today, options.near_expiry_days)?;
        let all = stock_rows(conn, filter, soon)?;
        let mut total_value = Money::zero();
        for row in &all {
            total_value = total_value.checked_add(row.value)?;
        }
        let limit = page.limit.clamp(1, MAX_PAGE);
        let rows = all
            .iter()
            .skip(usize::try_from(page.offset).unwrap_or(usize::MAX))
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .cloned()
            .collect();
        Ok(StockList {
            rows,
            total: all.len() as u64,
            total_value,
            currency: settings::get(conn)?,
        })
    })
}

/// One product's stock and batches.
pub fn product(state: &AppState, id: &Id) -> Result<StockProduct, AppError> {
    let today = today(state)?;
    state.db.read(|conn| {
        let product = catalog::get(conn, id)?;
        stock_product(conn, product, today)
    })
}

/// Finds a product by barcode or SKU for the count screen and the adjust form, and
/// gives its stock. A code that matches nothing is `NotFound`.
pub fn lookup(state: &AppState, code: &str) -> Result<StockProduct, AppError> {
    let today = today(state)?;
    state.db.read(|conn| {
        let found = catalog::find_by_code(conn, code)?.ok_or_else(|| AppError::NotFound {
            entity: "product".into(),
            key: code.trim().to_string(),
        })?;
        let product = catalog::get(conn, &found.id)?;
        stock_product(conn, product, today)
    })
}

/// The active adjustment reasons for the adjust form (INV-02).
pub fn reasons(state: &AppState) -> Result<Vec<Reason>, AppError> {
    state.db.read(|conn| {
        Ok(inventory::list_reasons(conn)?
            .into_iter()
            .filter(|reason| reason.active)
            .collect())
    })
}

/// Adjusts a product's stock with a reason, in one transaction with its audit event
/// (INV-02). Returns the product's stock afterwards.
pub fn adjust(state: &AppState, by: &Id, input: &AdjustInput) -> Result<Qty, AppError> {
    let clock = state.clock.as_ref();
    let now = clock.now();
    state.db.write(|tx| {
        let product = catalog::get(tx, &input.product_id)?;
        require_stocked(&product)?;
        let options: InventorySettings = settings::get(tx)?;
        let coming_in = input.qty_delta.0 > 0;
        let batch = match &input.batch {
            AdjustBatch::Auto if coming_in => BatchChoice::None,
            AdjustBatch::Auto => BatchChoice::Fefo,
            AdjustBatch::Existing { id } => BatchChoice::Specific(id.clone()),
            AdjustBatch::New {
                batch_no,
                expiry_date,
            } => {
                if !coming_in {
                    return Err(field_error("batch", "new_batch_only_for_stock_in"));
                }
                if product.track_expiry && expiry_date.is_none() {
                    return Err(field_error("expiry_date", "expiry_needed"));
                }
                BatchChoice::Specific(inventory::create_batch(
                    tx,
                    &product.id,
                    batch_no.as_deref(),
                    *expiry_date,
                    product.cost_price,
                    now,
                )?)
            }
        };
        let note = input
            .note
            .as_deref()
            .map(str::trim)
            .filter(|note| !note.is_empty())
            .map(str::to_string);
        let movement = Movement {
            batch,
            reason_code: Some(input.reason_code.clone()),
            note: note.clone(),
            unit_cost: product.cost_price,
            tracks_expiry: product.track_expiry,
            ..Movement::new(
                &product.id,
                input.qty_delta,
                MoveKind::Adjust,
                move_ctx(tx, clock, by)?,
            )
        };
        let written = inventory::record(tx, movement, options.allow_negative)?;
        let details = serde_json::json!({
            "product_id": product.id,
            "name": product.name,
            "sku": product.sku,
            "qty_delta_milli": input.qty_delta.0,
            "reason": input.reason_code,
            "note": note,
            "batches": written.iter().map(|row| &row.batch_id).collect::<Vec<_>>(),
        });
        audit_log::record(tx, clock, STOCK_ADJUST, details, Some(by.clone()))?;
        Ok(inventory::on_hand(tx, &product.id)?)
    })
}

/// A product's stock history, newest first, each row with the stock right after it (INV-07).
pub fn history(state: &AppState, query: &HistoryQuery) -> Result<Vec<HistoryRow>, AppError> {
    state.db.read(|conn| {
        catalog::get(conn, &query.product_id)?;
        let filter = HistoryFilter {
            from: query.from,
            to: query.to,
            kinds: query.kinds.clone(),
            page: Page {
                offset: query.page.offset,
                limit: query.page.limit.clamp(1, MAX_PAGE),
            },
        };
        let users = user_names(conn)?;
        let reasons: HashMap<String, String> = inventory::list_reasons(conn)?
            .into_iter()
            .map(|reason| (reason.code, reason.name))
            .collect();
        let mut rows = Vec::new();
        for found in inventory::history(conn, &query.product_id, &filter)? {
            let entry = found.entry;
            let batch = match &entry.batch_id {
                Some(id) => Some(inventory::get_batch(conn, id)?),
                None => None,
            };
            let (ref_type, ref_label) = match &entry.ref_ {
                Some((kind, id)) => (Some(kind.clone()), document_label(conn, kind, id)),
                None => (None, None),
            };
            rows.push(HistoryRow {
                id: entry.id,
                at: entry.at,
                business_date: entry.business_date,
                kind: entry.kind,
                qty_delta: entry.qty_delta,
                balance_after: found.balance_after,
                batch_no: batch.as_ref().and_then(|batch| batch.batch_no.clone()),
                batch_expiry: batch.as_ref().and_then(|batch| batch.expiry_date),
                reason_name: entry
                    .reason_code
                    .as_ref()
                    .map(|code| reasons.get(code).cloned().unwrap_or_else(|| code.clone())),
                note: entry.note,
                ref_type,
                ref_label,
                unit_cost: entry.unit_cost,
                user_name: entry.user_id.as_ref().and_then(|id| users.get(id).cloned()),
            });
        }
        Ok(rows)
    })
}

/// Products below their reorder level, the biggest shortfall first (INV-03).
pub fn low_stock(state: &AppState, page: ProductPage) -> Result<Vec<LowStockView>, AppError> {
    state.db.read(|conn| {
        let page = Page {
            offset: page.offset,
            limit: page.limit.clamp(1, MAX_PAGE),
        };
        low_stock_rows(conn, page)
    })
}

/// Batches with stock that expire within the near-expiry days, or have expired (INV-05).
pub fn near_expiry(state: &AppState) -> Result<Vec<NearExpiryView>, AppError> {
    let today = today(state)?;
    state.db.read(|conn| {
        let options: InventorySettings = settings::get(conn)?;
        near_expiry_rows(conn, today, options.near_expiry_days)
    })
}

/// The stock alerts for the dashboard at login (INV-03, INV-05).
pub fn alerts(state: &AppState) -> Result<StockAlerts, AppError> {
    let today = today(state)?;
    state.db.read(|conn| {
        let options: InventorySettings = settings::get(conn)?;
        let expiring = near_expiry_rows(conn, today, options.near_expiry_days)?;
        let negative_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM inventory_v_levels WHERE on_hand_milli < 0",
                [],
                |row| row.get(0),
            )
            .map_err(sql_error)?;
        Ok(StockAlerts {
            low_stock_count: inventory::low_stock_count(conn)?,
            low_stock: low_stock_rows(conn, Page::first(ALERT_ROWS))?,
            near_expiry_count: u32::try_from(expiring.len()).unwrap_or(u32::MAX),
            near_expiry: expiring.into_iter().take(ALERT_ROWS as usize).collect(),
            negative_count,
            near_expiry_days: options.near_expiry_days,
        })
    })
}

/// "Verify stock data" (INV-10): every level and batch that doesn't match its ledger.
/// Nothing is fixed.
pub fn verify(state: &AppState) -> Result<Vec<MismatchView>, AppError> {
    state.db.read(|conn| {
        let mut rows = Vec::new();
        for found in inventory::verify(conn)? {
            let name = catalog::get(conn, &found.product_id)
                .map(|product| product.name)
                .unwrap_or_default();
            let batch_no = match &found.batch_id {
                Some(id) => inventory::get_batch(conn, id)?.batch_no,
                None => None,
            };
            rows.push(MismatchView {
                product_id: found.product_id,
                name,
                batch_id: found.batch_id,
                batch_no,
                cached: found.cached,
                ledger_sum: found.ledger_sum,
            });
        }
        Ok(rows)
    })
}

/// Where and when a stock change happens and who makes it.
pub(crate) fn move_ctx(tx: &Transaction, clock: &dyn Clock, by: &Id) -> Result<MoveCtx, AppError> {
    let till: TillSettings = settings::get(tx)?;
    let till_id = till.till_id.ok_or_else(|| AppError::Conflict {
        code: "setup_not_done".into(),
        params: Default::default(),
    })?;
    let at = clock.now();
    Ok(MoveCtx {
        at,
        business_date: pos_core::business_date(
            at,
            clock.local_offset_minutes(),
            till.business_day_cutoff_hour,
        )?,
        till_id,
        user_id: Some(by.clone()),
    })
}

/// Kits, variant parents and products with "track stock" off never have stock (INV-09).
pub(crate) fn require_stocked(product: &Product) -> Result<(), AppError> {
    let holds_stock = matches!(product.kind, ProductKind::Standard | ProductKind::Variant);
    if holds_stock && product.track_stock {
        Ok(())
    } else {
        Err(field_error("product_id", "product_keeps_no_stock"))
    }
}

/// Every user's display name by id, for "who did it" columns.
pub(crate) fn user_names(conn: &Connection) -> Result<HashMap<Id, String>, AppError> {
    Ok(auth::list_users(conn)?
        .into_iter()
        .map(|user| (user.id, user.display_name))
        .collect())
}

/// Today's business date at this till.
pub(crate) fn today(state: &AppState) -> Result<NaiveDate, AppError> {
    let till: TillSettings = state.db.read(|conn| Ok(settings::get(conn)?))?;
    let clock = state.clock.as_ref();
    Ok(pos_core::business_date(
        clock.now(),
        clock.local_offset_minutes(),
        till.business_day_cutoff_hour,
    )?
    .0)
}

/// A product's unit symbol and decimals.
pub(crate) fn unit_of(conn: &Connection, unit_id: &Id) -> Result<(String, u8), AppError> {
    Ok(catalog::list_units(conn, true)?
        .into_iter()
        .find(|unit| &unit.id == unit_id)
        .map(|unit| (unit.symbol, unit.decimals))
        .unwrap_or_default())
}

fn stock_product(
    conn: &Connection,
    product: Product,
    today: NaiveDate,
) -> Result<StockProduct, AppError> {
    require_stocked(&product)?;
    let options: InventorySettings = settings::get(conn)?;
    let soon = expiry_limit(today, options.near_expiry_days)?;
    let (unit_symbol, unit_decimals) = unit_of(conn, &product.unit_id)?;
    let rule = inventory::reorder_rule(conn, &product.id)?;
    let batches = inventory::batches_of(conn, &product.id)?
        .into_iter()
        .map(|batch| BatchView {
            expired: batch.expiry_date.is_some_and(|date| date < today),
            near_expiry: batch.expiry_date.is_some_and(|date| date <= soon),
            id: batch.id,
            batch_no: batch.batch_no,
            expiry_date: batch.expiry_date,
            on_hand: batch.on_hand,
            unit_cost: batch.unit_cost,
            received_at: batch.received_at,
        })
        .collect();
    Ok(StockProduct {
        on_hand: inventory::on_hand(conn, &product.id)?,
        reorder_level: rule.map(|rule| rule.level),
        reorder_qty: rule.map(|rule| rule.qty),
        product_id: product.id,
        name: product.name,
        sku: product.sku,
        unit_symbol,
        unit_decimals,
        track_expiry: product.track_expiry,
        cost_price: product.cost_price,
        batches,
    })
}

/// The last date that counts as "near expiry": today + days.
fn expiry_limit(today: NaiveDate, days: u32) -> Result<NaiveDate, AppError> {
    today
        .checked_add_days(Days::new(u64::from(days)))
        .ok_or_else(|| AppError::internal(format!("near expiry: {today} + {days} days")))
}

/// Every row of the stock levels list that matches the filter, by name. Reads catalog's
/// and inventory's published views (Module Rules R2).
fn stock_rows(
    conn: &Connection,
    filter: &StockFilter,
    soon: NaiveDate,
) -> Result<Vec<StockRow>, AppError> {
    let text = filter
        .text
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_lowercase);
    let show = match filter.show {
        StockShow::All => "1",
        StockShow::Low => {
            "l.reorder_level_milli IS NOT NULL AND IFNULL(l.on_hand_milli, 0) < l.reorder_level_milli"
        }
        StockShow::Negative => "IFNULL(l.on_hand_milli, 0) < 0",
        StockShow::Expiring => "p.id IN (SELECT product_id FROM expiring)",
    };
    let sql = format!(
        "WITH RECURSIVE below(id) AS (
           SELECT :category UNION SELECT c.id FROM catalog_v_categories c JOIN below ON c.parent_id = below.id
         ),
         expiring AS (
           SELECT DISTINCT product_id FROM inventory_v_batches
           WHERE expiry_date IS NOT NULL AND expiry_date <= :soon AND on_hand_milli > 0
         )
         SELECT p.id, p.name, p.sku, p.category_name, p.unit_symbol, p.unit_decimals,
                IFNULL(l.on_hand_milli, 0), l.reorder_level_milli, p.cost_price_minor,
                p.id IN (SELECT product_id FROM expiring)
         FROM catalog_v_products p
         LEFT JOIN inventory_v_levels l ON l.product_id = p.id
         WHERE p.kind IN ('standard', 'variant') AND p.track_stock = 1
           AND (p.archived_at IS NULL OR IFNULL(l.on_hand_milli, 0) <> 0)
           AND (:text IS NULL OR instr(lower(p.name), :text) > 0 OR instr(lower(p.sku), :text) > 0)
           AND (:category IS NULL OR p.category_id IN (SELECT id FROM below))
           AND {show}
         ORDER BY p.name COLLATE NOCASE, p.sku"
    );
    let mut statement = conn.prepare(&sql).map_err(sql_error)?;
    let found = statement
        .query_map(
            named_params! {
                ":category": filter.category_id.as_ref().map(Id::as_str),
                ":soon": soon.format("%Y-%m-%d").to_string(),
                ":text": text,
            },
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, u8>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, bool>(9)?,
                ))
            },
        )
        .map_err(sql_error)?;
    let mut rows = Vec::new();
    for each in found {
        let (
            id,
            name,
            sku,
            category_name,
            unit_symbol,
            unit_decimals,
            on_hand,
            level,
            cost,
            expiring,
        ) = each.map_err(sql_error)?;
        let on_hand = Qty(on_hand);
        let cost_price = Money(cost);
        rows.push(StockRow {
            product_id: Id::parse(&id)?,
            name,
            sku,
            category_name,
            unit_symbol,
            unit_decimals,
            on_hand,
            reorder_level: level.map(Qty),
            cost_price,
            value: mul_qty(cost_price, on_hand)?,
            low: level.is_some_and(|level| on_hand.0 < level),
            negative: on_hand.0 < 0,
            expiring,
        });
    }
    Ok(rows)
}

fn low_stock_rows(conn: &Connection, page: Page) -> Result<Vec<LowStockView>, AppError> {
    let mut rows = Vec::new();
    for low in inventory::low_stock(conn, page)? {
        let product = catalog::get(conn, &low.product_id)?;
        let (unit_symbol, unit_decimals) = unit_of(conn, &product.unit_id)?;
        rows.push(LowStockView {
            product_id: low.product_id,
            name: product.name,
            sku: product.sku,
            unit_symbol,
            unit_decimals,
            on_hand: low.on_hand,
            reorder_level: low.reorder_level,
            reorder_qty: low.reorder_qty,
        });
    }
    Ok(rows)
}

fn near_expiry_rows(
    conn: &Connection,
    today: NaiveDate,
    days: u32,
) -> Result<Vec<NearExpiryView>, AppError> {
    let mut rows = Vec::new();
    for batch in inventory::near_expiry(conn, today, days)? {
        let Some(expiry_date) = batch.expiry_date else {
            continue;
        };
        let product = catalog::get(conn, &batch.product_id)?;
        let (unit_symbol, unit_decimals) = unit_of(conn, &product.unit_id)?;
        rows.push(NearExpiryView {
            batch_id: batch.id,
            product_id: batch.product_id,
            name: product.name,
            unit_symbol,
            unit_decimals,
            batch_no: batch.batch_no,
            expiry_date,
            on_hand: batch.on_hand,
            expired: expiry_date < today,
        });
    }
    Ok(rows)
}

/// A short name for the document behind a stock change, when the app knows it.
fn document_label(conn: &Connection, kind: &str, id: &Id) -> Option<String> {
    match kind {
        "count" => inventory::get_count(conn, id)
            .ok()
            .map(|count| count.number),
        _ => None,
    }
}

/// A database error while reading the published views. The details go to the log.
fn sql_error(error: rusqlite::Error) -> AppError {
    AppError::internal(format!("stock list: {error}"))
}

pub(crate) fn field_error(field: &str, code: &str) -> AppError {
    AppError::Validation {
        field: Some(field.to_string()),
        code: code.to_string(),
        params: Default::default(),
    }
}
