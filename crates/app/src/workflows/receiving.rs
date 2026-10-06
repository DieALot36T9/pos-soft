//! Receiving goods (GRN) and sending goods back to suppliers (Req - Purchasing PUR-03,
//! PUR-04; Workflows "Receive goods (GRN)", "Supplier return"). Each is one transaction:
//! the document with its number, the stock, the product's cost (ADR-018), the supplier's
//! balance and the audit event.

use catalog::Product;
use chrono::NaiveDate;
use pos_core::{AppError, CurrencyFormat, Id, Money, Qty, moving_average, mul_qty};
use purchasing::{
    Grn, GrnRow, NewGrn, NewGrnLine, NewReturnLine, NewSupplierReturn, OrderStatus, ReturnRef,
    ReturnRow,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use settings::{CostMethod, InventorySettings};
use ts_rs::TS;

use crate::audit_actions::{GRN_SAVE, SUPPLIER_RETURN_SAVE};
use crate::audit_log;
use crate::state::AppState;
use crate::workflows::purchasing_admin::{blank_to_none, purchasing_ctx};
use crate::workflows::stock::{field_error, move_ctx, require_stocked};

/// A delivery as the receiving screen sends it (PUR-03).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct GrnInput {
    pub supplier_id: Id,
    /// The purchase order it fills, if any.
    pub order_id: Option<Id>,
    pub supplier_invoice_no: Option<String>,
    /// The total printed on the supplier's invoice. The balance goes up by this amount.
    pub invoice_total: Money,
    pub notes: Option<String>,
    pub lines: Vec<GrnLineInput>,
}

/// One line of the receiving grid.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct GrnLineInput {
    /// The order line it fills, when receiving against an order.
    pub order_line_id: Option<Id>,
    pub product_id: Id,
    pub qty: Qty,
    /// Cost for one unit, as on the supplier's invoice.
    pub unit_cost: Money,
    pub batch_no: Option<String>,
    /// Needed when the product tracks expiry dates (INV-05).
    pub expiry_date: Option<NaiveDate>,
}

/// The receiving form for a purchase order: its lines with what is still to come.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GrnStart {
    pub supplier_id: Id,
    pub supplier_name: String,
    pub order_id: Id,
    /// Example: "PO-000001".
    pub order_number: String,
    pub lines: Vec<GrnStartLine>,
    pub currency: CurrencyFormat,
}

/// One order line ready to receive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GrnStartLine {
    pub order_line_id: Id,
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub ordered: Qty,
    pub received: Qty,
    /// What is still to come. The quantity field starts with this.
    pub remaining: Qty,
    /// The cost on the order.
    pub unit_cost: Money,
    /// The product needs an expiry date on each delivery.
    pub tracks_expiry: bool,
}

/// What a delivery would do, before it is saved: line values, the totals and how each
/// product's cost changes (ADR-018). Saves nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GrnPreview {
    pub lines: Vec<GrnLinePreview>,
    /// Σ round(unit cost × quantity).
    pub lines_total: Money,
    pub invoice_total: Money,
    /// invoice total − lines total. Not zero: the screen shows a warning (PUR-03).
    pub difference: Money,
}

/// One line of the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GrnLinePreview {
    pub product_id: Id,
    pub name: String,
    pub value: Money,
    pub tracks_expiry: bool,
    /// The product's cost now, and after this delivery.
    pub old_cost: Money,
    pub new_cost: Money,
}

/// A saved delivery, with the warnings to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct GrnDone {
    pub grn_id: Id,
    /// Example: "GRN-000001".
    pub number: String,
    pub lines_total: Money,
    pub invoice_total: Money,
    pub total_differs: bool,
    /// Products that now have more received than ordered (allowed, shown as a warning).
    pub over_received: Vec<String>,
    /// The order's new status, when the delivery filled an order.
    pub order_status: Option<OrderStatus>,
    /// Each product whose cost changed: name, old and new cost.
    pub cost_changes: Vec<CostChange>,
}

/// A product's cost before and after a delivery (ADR-018).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CostChange {
    pub product_id: Id,
    pub name: String,
    pub old_cost: Money,
    pub new_cost: Money,
}

/// Goods sent back to a supplier, as the screen sends them (PUR-04).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct SupplierReturnInput {
    pub supplier_id: Id,
    /// The delivery the goods came in on, if one was picked.
    pub grn_id: Option<Id>,
    pub reason: Option<String>,
    pub lines: Vec<SupplierReturnLineInput>,
}

/// One line sent back.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct SupplierReturnLineInput {
    pub product_id: Id,
    pub qty: Qty,
    /// The value of one unit sent back. None: the product's cost now.
    pub unit_cost: Option<Money>,
    /// The batch it comes out of. None: first expiry first.
    pub batch_id: Option<Id>,
}

/// The receiving form for a purchase order, pre-filled with what is still to come.
pub fn start(state: &AppState, order_id: &Id) -> Result<GrnStart, AppError> {
    state.db.read(|conn| {
        let order = purchasing::get_order(conn, order_id)?;
        let supplier = purchasing::get_supplier(conn, &order.supplier_id)?;
        let mut lines = Vec::new();
        for line in purchasing::order_for_receiving(conn, order_id)? {
            let product = catalog::get(conn, &line.product_id)?;
            lines.push(GrnStartLine {
                order_line_id: line.order_line_id,
                product_id: line.product_id,
                name: line.product_name,
                sku: line.sku,
                ordered: line.ordered,
                received: line.received,
                remaining: line.remaining,
                unit_cost: line.unit_cost,
                tracks_expiry: product.track_expiry,
            });
        }
        Ok(GrnStart {
            supplier_id: supplier.id,
            supplier_name: supplier.name,
            order_id: order.id,
            order_number: order.number,
            lines,
            currency: settings::get(conn)?,
        })
    })
}

/// The values, totals and cost changes a delivery would make. Saves nothing.
pub fn preview(state: &AppState, input: &GrnInput) -> Result<GrnPreview, AppError> {
    state.db.read(|conn| {
        let method = cost_method(conn)?;
        let mut planner = CostPlanner::default();
        let mut lines = Vec::with_capacity(input.lines.len());
        let mut lines_total = Money::zero();
        for line in &input.lines {
            let product = catalog::get(conn, &line.product_id)?;
            let value = mul_qty(line.unit_cost, line.qty)?;
            lines_total = lines_total.checked_add(value)?;
            let (old_cost, new_cost) = planner.next(conn, &product, line, method)?;
            lines.push(GrnLinePreview {
                product_id: product.id,
                name: product.name,
                value,
                tracks_expiry: product.track_expiry,
                old_cost,
                new_cost,
            });
        }
        Ok(GrnPreview {
            lines,
            lines_total,
            invoice_total: input.invoice_total,
            difference: input.invoice_total.checked_sub(lines_total)?,
        })
    })
}

/// Saves a delivery in one transaction (PUR-03): batches for dated goods, the GRN with
/// its number and the supplier's invoice, each product's new cost (moving average or
/// last cost, ADR-018), the stock (`receive` rows), and the audit event `grn.save`.
pub fn save(state: &AppState, by: &Id, input: &GrnInput) -> Result<GrnDone, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let method = cost_method(tx)?;
        let now = clock.now();
        let (products, lines) = prepare_lines(tx, input, now)?;
        let ctx = purchasing_ctx(tx, clock, by)?;
        let saved = purchasing::record_grn(
            tx,
            &NewGrn {
                supplier_id: input.supplier_id.clone(),
                order_id: input.order_id.clone(),
                supplier_invoice_no: blank_to_none(input.supplier_invoice_no.as_deref()),
                invoice_total: input.invoice_total,
                notes: blank_to_none(input.notes.as_deref()),
                lines: lines.clone(),
            },
            &ctx,
        )?;
        let stock_ctx = move_ctx(tx, clock, by)?;
        let cost_changes = receive_lines(tx, &stock_ctx, by, method, &products, &lines, &saved.id)?;
        let over_received = over_received_names(&lines, &saved.over_received);
        audit_log::record(
            tx,
            clock,
            GRN_SAVE,
            serde_json::json!({
                "grn_id": saved.id, "number": saved.number, "supplier_id": input.supplier_id,
                "invoice_total_minor": saved.invoice_total.0, "lines_total_minor": saved.lines_total.0,
                "cost_changes": cost_changes.iter().map(|change| serde_json::json!({
                    "product_id": change.product_id, "old_minor": change.old_cost.0,
                    "new_minor": change.new_cost.0,
                })).collect::<Vec<_>>(),
            }),
            Some(by.clone()),
        )?;
        Ok(GrnDone {
            grn_id: saved.id,
            number: saved.number,
            lines_total: saved.lines_total,
            invoice_total: saved.invoice_total,
            total_differs: saved.total_differs,
            over_received,
            order_status: saved.order_status,
            cost_changes,
        })
    })
}

/// Checks each line, makes a batch for dated goods, and copies the product's name.
fn prepare_lines(
    tx: &rusqlite::Transaction,
    input: &GrnInput,
    now: pos_core::Timestamp,
) -> Result<(Vec<Product>, Vec<NewGrnLine>), AppError> {
    let mut products = Vec::with_capacity(input.lines.len());
    let mut lines = Vec::with_capacity(input.lines.len());
    for line in &input.lines {
        let product = catalog::get(tx, &line.product_id)?;
        require_stocked(&product)?;
        if product.track_expiry && line.expiry_date.is_none() {
            return Err(field_error("expiry_date", "expiry_needed"));
        }
        let batch_no = blank_to_none(line.batch_no.as_deref());
        let batch = if batch_no.is_some() || line.expiry_date.is_some() {
            let made = inventory::create_batch(
                tx,
                &product.id,
                batch_no.as_deref(),
                line.expiry_date,
                line.unit_cost,
                now,
            )?;
            Some(made)
        } else {
            None
        };
        lines.push(NewGrnLine {
            order_line_id: line.order_line_id.clone(),
            product_id: product.id.clone(),
            product_name: product.name.clone(),
            qty: line.qty,
            unit_cost: line.unit_cost,
            batch_no,
            expiry_date: line.expiry_date,
            inventory_batch_id: batch,
        });
        products.push(product);
    }
    Ok((products, lines))
}

/// Each line's new cost (ADR-018), then its stock (`receive` rows pointing at the GRN).
/// Gives the products whose cost changed.
fn receive_lines(
    tx: &rusqlite::Transaction,
    stock_ctx: &inventory::MoveCtx,
    by: &Id,
    method: CostMethod,
    products: &[Product],
    lines: &[NewGrnLine],
    grn_id: &Id,
) -> Result<Vec<CostChange>, AppError> {
    let now = stock_ctx.at;
    let mut cost_changes: Vec<CostChange> = Vec::new();
    for (line, product) in lines.iter().zip(products) {
        // Read again: an earlier line of the same product may have changed the cost.
        let current = catalog::get(tx, &product.id)?;
        let on_hand = inventory::on_hand(tx, &product.id)?;
        let new_cost = new_cost(
            method,
            on_hand,
            current.cost_price,
            line.qty,
            line.unit_cost,
        )?;
        if new_cost != current.cost_price {
            catalog::set_cost(tx, &product.id, new_cost, "grn", by, now)?;
            match cost_changes
                .iter_mut()
                .find(|each| each.product_id == product.id)
            {
                Some(change) => change.new_cost = new_cost,
                None => cost_changes.push(CostChange {
                    product_id: product.id.clone(),
                    name: product.name.clone(),
                    old_cost: current.cost_price,
                    new_cost,
                }),
            }
        }
        let batch = match &line.inventory_batch_id {
            Some(id) => inventory::BatchChoice::Specific(id.clone()),
            None => inventory::BatchChoice::None,
        };
        let movement = inventory::Movement {
            batch,
            ref_: Some(("grn".to_string(), grn_id.clone())),
            unit_cost: line.unit_cost,
            tracks_expiry: product.track_expiry,
            ..inventory::Movement::new(
                &product.id,
                line.qty,
                inventory::MoveKind::Receive,
                stock_ctx.clone(),
            )
        };
        inventory::record(tx, movement, true)?;
    }
    Ok(cost_changes)
}

/// Every delivery, newest first.
pub fn grns(state: &AppState) -> Result<Vec<GrnRow>, AppError> {
    state.db.read(|conn| Ok(purchasing::list_grns(conn, None)?))
}

/// One delivery with its lines, to send goods back against it (PUR-04).
pub fn grn(state: &AppState, id: &Id) -> Result<Grn, AppError> {
    state.db.read(|conn| Ok(purchasing::get_grn(conn, id)?))
}

/// Every supplier return, newest first.
pub fn returns(state: &AppState) -> Result<Vec<ReturnRow>, AppError> {
    state.db.read(|conn| Ok(purchasing::list_returns(conn)?))
}

/// Sends goods back to a supplier in one transaction (PUR-04): the return with its SR
/// number and the supplier's credit (at the cost given, default the current cost), the
/// stock out (`supplier_return` rows, from the chosen batch or first expiry first), and
/// the audit event `supplier_return.save`.
pub fn send_back(
    state: &AppState,
    by: &Id,
    input: &SupplierReturnInput,
) -> Result<ReturnRef, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let rules: InventorySettings = settings::get(tx)?;
        let mut products: Vec<Product> = Vec::with_capacity(input.lines.len());
        let mut lines = Vec::with_capacity(input.lines.len());
        for line in &input.lines {
            let product = catalog::get(tx, &line.product_id)?;
            require_stocked(&product)?;
            let unit_cost = line.unit_cost.unwrap_or(product.cost_price);
            if unit_cost.0 < 0 {
                return Err(field_error("unit_cost", "cost_negative"));
            }
            lines.push(NewReturnLine {
                product_id: product.id.clone(),
                product_name: product.name.clone(),
                qty: line.qty,
                unit_cost,
                inventory_batch_id: line.batch_id.clone(),
            });
            products.push(product);
        }
        let ctx = purchasing_ctx(tx, clock, by)?;
        let reason = blank_to_none(input.reason.as_deref());
        let saved = purchasing::record_return(
            tx,
            &NewSupplierReturn {
                supplier_id: input.supplier_id.clone(),
                grn_id: input.grn_id.clone(),
                reason: reason.clone(),
                lines: lines.clone(),
            },
            &ctx,
        )?;
        let stock_ctx = move_ctx(tx, clock, by)?;
        for (line, product) in lines.iter().zip(&products) {
            let batch = match &line.inventory_batch_id {
                Some(id) => inventory::BatchChoice::Specific(id.clone()),
                None => inventory::BatchChoice::Fefo,
            };
            let movement = inventory::Movement {
                batch,
                ref_: Some(("supplier_return".to_string(), saved.id.clone())),
                unit_cost: line.unit_cost,
                tracks_expiry: product.track_expiry,
                ..inventory::Movement::new(
                    &product.id,
                    Qty(-line.qty.0),
                    inventory::MoveKind::SupplierReturn,
                    stock_ctx.clone(),
                )
            };
            inventory::record(tx, movement, rules.allow_negative)?;
        }
        audit_log::record(
            tx,
            clock,
            SUPPLIER_RETURN_SAVE,
            serde_json::json!({
                "return_id": saved.id, "number": saved.number, "supplier_id": input.supplier_id,
                "total_minor": saved.total.0, "reason": reason,
            }),
            Some(by.clone()),
        )?;
        Ok(saved)
    })
}

/// ADR-018: moving average by default, last purchase cost when the shop chose it.
fn new_cost(
    method: CostMethod,
    on_hand: Qty,
    old_cost: Money,
    received: Qty,
    received_cost: Money,
) -> Result<Money, AppError> {
    match method {
        CostMethod::MovingAverage => moving_average(on_hand, old_cost, received, received_cost),
        CostMethod::LastCost => Ok(received_cost),
    }
}

fn cost_method(conn: &Connection) -> Result<CostMethod, AppError> {
    let rules: InventorySettings = settings::get(conn)?;
    Ok(rules.cost_method)
}

/// Works out the cost changes of a preview line by line, as saving would: each line
/// adds its quantity to the stock and its cost to the average before the next.
#[derive(Default)]
struct CostPlanner {
    seen: Vec<(Id, Qty, Money)>,
}

impl CostPlanner {
    fn next(
        &mut self,
        conn: &Connection,
        product: &Product,
        line: &GrnLineInput,
        method: CostMethod,
    ) -> Result<(Money, Money), AppError> {
        let (on_hand, cost) = match self.seen.iter().find(|each| each.0 == product.id) {
            Some((_, on_hand, cost)) => (*on_hand, *cost),
            None => (inventory::on_hand(conn, &product.id)?, product.cost_price),
        };
        let new = new_cost(method, on_hand, cost, line.qty, line.unit_cost)?;
        let after = Qty(on_hand.0.saturating_add(line.qty.0));
        self.seen.retain(|each| each.0 != product.id);
        self.seen.push((product.id.clone(), after, new));
        Ok((cost, new))
    }
}

/// The names of the products whose order lines got more than ordered.
fn over_received_names(lines: &[NewGrnLine], over: &[Id]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| {
            line.order_line_id
                .as_ref()
                .is_some_and(|id| over.contains(id))
        })
        .map(|line| line.product_name.clone())
        .collect()
}
