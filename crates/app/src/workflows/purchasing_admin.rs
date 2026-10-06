//! Suppliers, purchase orders and what the shop owes (Req - Purchasing PUR-01, PUR-02,
//! PUR-05, PUR-06). Each change is one transaction with its audit event. Receiving goods
//! and supplier returns are in `receiving`.

use std::path::Path;

use chrono::NaiveDate;
use io::{XlsxCell, XlsxSheet};
use pos_core::{AppError, Clock, CurrencyFormat, Id, Money, Qty, business_date, mul_qty};
use purchasing::{
    AgingRow, GrnRow, NewOrder, NewOrderLine, Order, OrderRow, OrderStatus, PurchasingCtx,
    Statement, Supplier, SupplierInput,
};
use rusqlite::{Connection, Transaction};
use sales::PaymentKind;
use serde::{Deserialize, Serialize};
use settings::TillSettings;
use shifts::{MovementKind, ShiftMovement};
use ts_rs::TS;

use crate::audit_actions::{PURCHASE_ORDER_CHANGE, SUPPLIER_CHANGE, SUPPLIER_PAYMENT};
use crate::audit_log;
use crate::devices::Devices;
use crate::state::AppState;
use crate::workflows::checkout::{PaymentMethodView, takes_payment_now};
use crate::workflows::receipts::{self, PrintResult};
use crate::workflows::stock::field_error;
use crate::workflows::till::{conflict, till_shift};

/// The most low-stock products looked at for an order suggestion.
const SUGGEST_LIMIT: u32 = 1_000;

/// A new supplier (`id` None) or changes to one (PUR-01).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct SupplierSave {
    pub id: Option<Id>,
    pub input: SupplierInput,
}

/// One supplier in the list, with what the shop owes them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SupplierRow {
    pub supplier: Supplier,
    /// Positive: the shop owes the supplier. Negative: the supplier owes the shop.
    pub balance: Money,
}

/// The supplier list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SupplierList {
    pub rows: Vec<SupplierRow>,
    pub currency: CurrencyFormat,
}

/// One supplier's page: details, balance, deliveries and how a payment can be made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SupplierPage {
    pub supplier: Supplier,
    pub balance: Money,
    /// The newest deliveries first.
    pub grns: Vec<GrnRow>,
    /// Cash, card, wallet and other methods that are switched on.
    pub payment_methods: Vec<PaymentMethodView>,
    /// True when the user has the till's open shift: cash can come from the drawer.
    pub has_open_shift: bool,
    pub currency: CurrencyFormat,
}

/// A purchase order as the editor sends it (PUR-02). Names and SKUs come from the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct OrderSave {
    /// None: a new draft. Otherwise the draft to change.
    pub id: Option<Id>,
    pub supplier_id: Id,
    pub expected_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub lines: Vec<OrderLineInput>,
}

/// One line of the order editor.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct OrderLineInput {
    pub product_id: Id,
    pub qty: Qty,
    /// Expected cost for one unit.
    pub unit_cost: Money,
}

/// A purchase order with the values of its lines, worked out in Rust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct OrderView {
    pub order: Order,
    pub supplier_name: String,
    /// round(unit cost × quantity) for each line, in the same order as the lines.
    pub line_values: Vec<Money>,
    pub total: Money,
    pub currency: CurrencyFormat,
}

/// The purchase order list (PUR-02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct OrderList {
    pub rows: Vec<OrderRow>,
    pub currency: CurrencyFormat,
}

/// A payment to a supplier (PUR-05).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct SupplierPaymentInput {
    pub supplier_id: Id,
    pub amount: Money,
    /// Cash, card, wallet or other.
    pub method_id: Id,
    /// A cheque number, transaction id or similar.
    pub reference: Option<String>,
    /// Cash only: the money comes out of the till's drawer on the user's open shift.
    pub from_drawer: bool,
}

/// A recorded supplier payment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SupplierPaymentDone {
    /// What the shop owes the supplier now.
    pub balance: Money,
    /// The drawer, when the cash came out of it. None otherwise.
    pub drawer: Option<PrintResult>,
}

/// The aging report (PUR-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AgingView {
    pub rows: Vec<AgingRow>,
    pub today: NaiveDate,
    pub currency: CurrencyFormat,
}

/// Who and when, for the purchasing records, with the till's business day.
pub(crate) fn purchasing_ctx(
    conn: &Connection,
    clock: &dyn Clock,
    user: &Id,
) -> Result<PurchasingCtx, AppError> {
    let till: TillSettings = settings::get(conn)?;
    let now = clock.now();
    Ok(PurchasingCtx {
        at: now,
        business_date: business_date(
            now,
            clock.local_offset_minutes(),
            till.business_day_cutoff_hour,
        )?,
        user_id: user.clone(),
    })
}

/// Every supplier by name with their balance; archived ones only when asked (PUR-01).
pub fn suppliers(state: &AppState, include_archived: bool) -> Result<SupplierList, AppError> {
    state.db.read(|conn| {
        let mut rows = Vec::new();
        for supplier in purchasing::list_suppliers(conn, include_archived)? {
            let balance = purchasing::balance(conn, &supplier.id)?;
            rows.push(SupplierRow { supplier, balance });
        }
        Ok(SupplierList {
            rows,
            currency: settings::get(conn)?,
        })
    })
}

/// One supplier's page.
pub fn supplier_page(state: &AppState, user: &Id, id: &Id) -> Result<SupplierPage, AppError> {
    state.db.read(|conn| {
        let has_open_shift = till_shift(conn)?.is_some_and(|shift| shift.user_id == *user);
        let payment_methods = sales::payment_methods(conn)?
            .into_iter()
            .filter(|method| method.active && takes_payment_now(method.kind))
            .map(|method| PaymentMethodView {
                id: method.id,
                name: method.name,
                kind: method.kind.into(),
                requires_reference: method.requires_reference,
            })
            .collect();
        Ok(SupplierPage {
            supplier: purchasing::get_supplier(conn, id)?,
            balance: purchasing::balance(conn, id)?,
            grns: purchasing::list_grns(conn, Some(id))?,
            payment_methods,
            has_open_shift,
            currency: settings::get(conn)?,
        })
    })
}

/// Adds or changes a supplier, audited `supplier.change` (PUR-01).
pub fn save_supplier(state: &AppState, by: &Id, save: &SupplierSave) -> Result<Id, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let now = clock.now();
        let id = match &save.id {
            Some(id) => {
                purchasing::update_supplier(tx, id, &save.input, now)?;
                id.clone()
            }
            None => purchasing::create_supplier(tx, &save.input, now)?,
        };
        let change = if save.id.is_some() {
            "update"
        } else {
            "create"
        };
        audit_supplier(tx, clock, by, &id, &save.input.name, change)?;
        Ok(id)
    })
}

/// Archives a supplier: no new orders or deliveries; payments still work (PUR-01).
pub fn archive_supplier(state: &AppState, by: &Id, id: &Id) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        purchasing::archive_supplier(tx, id, clock.now())?;
        let name = purchasing::get_supplier(tx, id)?.name;
        audit_supplier(tx, clock, by, id, &name, "archive")
    })
}

/// The purchase orders, newest first; one status only when given (PUR-02).
pub fn orders(state: &AppState, status: Option<OrderStatus>) -> Result<OrderList, AppError> {
    state.db.read(|conn| {
        Ok(OrderList {
            rows: purchasing::list_orders(conn, status)?,
            currency: settings::get(conn)?,
        })
    })
}

/// One purchase order with its line values.
pub fn order(state: &AppState, id: &Id) -> Result<OrderView, AppError> {
    state.db.read(|conn| order_view(conn, id))
}

/// Saves a draft order with names and SKUs from the catalog, audited
/// `purchase_order.change` (PUR-02). A new one gets the next PO number.
pub fn save_order(state: &AppState, by: &Id, save: &OrderSave) -> Result<Id, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let mut lines = Vec::with_capacity(save.lines.len());
        for line in &save.lines {
            let product = catalog::get(tx, &line.product_id)?;
            lines.push(NewOrderLine {
                product_id: product.id,
                product_name: product.name,
                sku: product.sku,
                qty: line.qty,
                unit_cost: line.unit_cost,
            });
        }
        let order = NewOrder {
            supplier_id: save.supplier_id.clone(),
            expected_date: save.expected_date,
            notes: blank_to_none(save.notes.as_deref()),
            lines,
        };
        let (id, change) = match &save.id {
            Some(id) => {
                purchasing::update_order(tx, id, &order)?;
                (id.clone(), "update")
            }
            None => {
                let ctx = purchasing_ctx(tx, clock, by)?;
                (purchasing::create_order(tx, &order, &ctx)?.id, "create")
            }
        };
        audit_order(tx, clock, by, &id, change)?;
        Ok(id)
    })
}

/// "Suggest from low stock" (PUR-02): the low-stock products whose preferred supplier is
/// this one, each for its reorder quantity at today's cost. Saves nothing.
pub fn suggest(state: &AppState, supplier: &Id) -> Result<Vec<NewOrderLine>, AppError> {
    state.db.read(|conn| {
        let mut items = Vec::new();
        for row in inventory::low_stock(conn, inventory::Page::first(SUGGEST_LIMIT))? {
            let product = catalog::get(conn, &row.product_id)?;
            if product.archived_at.is_some() {
                continue;
            }
            items.push(purchasing::LowStockItem {
                product_id: product.id,
                product_name: product.name,
                sku: product.sku,
                preferred_supplier_id: product.preferred_supplier_id,
                reorder_qty: row.reorder_qty,
                unit_cost: product.cost_price,
            });
        }
        Ok(purchasing::suggest_order_lines(supplier, &items))
    })
}

/// Marks a draft order as sent (printed or handed over), audited.
pub fn mark_sent(state: &AppState, by: &Id, id: &Id) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        purchasing::mark_sent(tx, id, clock.now())?;
        audit_order(tx, clock, by, id, "send")
    })
}

/// Saves the order as an Excel file for the supplier and marks it sent (PUR-02).
pub fn export_order(state: &AppState, by: &Id, id: &Id, file: &Path) -> Result<(), AppError> {
    let view = order(state, id)?;
    let digits = view.currency.minor_digits;
    let mut rows = Vec::with_capacity(view.order.lines.len() + 1);
    for (line, value) in view.order.lines.iter().zip(&view.line_values) {
        rows.push(vec![
            XlsxCell::Text(line.sku.clone()),
            XlsxCell::Text(line.product_name.clone()),
            XlsxCell::Qty(line.qty),
            money_cell(line.unit_cost, digits),
            money_cell(*value, digits),
        ]);
    }
    rows.push(vec![
        XlsxCell::Empty,
        XlsxCell::Text(format!("{} {}", view.order.number, view.supplier_name)),
        XlsxCell::Empty,
        XlsxCell::Empty,
        money_cell(view.total, digits),
    ]);
    let sheet = XlsxSheet {
        name: view.order.number.clone(),
        headers: ORDER_HEADERS.iter().map(|text| text.to_string()).collect(),
        rows,
    };
    io::write_xlsx(file, &[sheet])?;
    if view.order.status == OrderStatus::Draft {
        mark_sent(state, by, id)?;
    }
    Ok(())
}

/// Cancels an order before anything arrived, audited (PUR-02).
pub fn cancel_order(state: &AppState, by: &Id, id: &Id) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        purchasing::cancel_order(tx, id)?;
        audit_order(tx, clock, by, id, "cancel")
    })
}

/// Records a payment to a supplier (PUR-05). Cash from the drawer goes on the user's open
/// shift as a supplier payment, and the drawer opens after the commit.
pub fn pay_supplier(
    state: &AppState,
    devices: &Devices,
    by: &Id,
    input: &SupplierPaymentInput,
) -> Result<SupplierPaymentDone, AppError> {
    let clock = state.clock.as_ref();
    let (balance, from_drawer) = state.db.write(|tx| {
        if input.amount.0 <= 0 {
            return Err(field_error("amount", "amount_not_positive"));
        }
        let method = sales::payment_method(tx, &input.method_id)?;
        if !method.active || !takes_payment_now(method.kind) {
            return Err(field_error("method_id", "payment_method_not_for_supplier"));
        }
        let reference = blank_to_none(input.reference.as_deref());
        if method.requires_reference && reference.is_none() {
            return Err(field_error("reference", "payment_reference_missing"));
        }
        let from_drawer = input.from_drawer && method.kind == PaymentKind::Cash;
        let ctx = purchasing_ctx(tx, clock, by)?;
        let kind = method.kind.as_str();
        let payment = purchasing::record_payment(
            tx,
            &input.supplier_id,
            input.amount,
            kind,
            reference.as_deref(),
            &ctx,
        )?;
        if from_drawer {
            let shift = till_shift(tx)?.ok_or_else(|| conflict("shift_not_open"))?;
            if shift.user_id != *by {
                return Err(conflict("shift_not_yours"));
            }
            shifts::record(
                tx,
                &shift.id,
                ShiftMovement {
                    kind: MovementKind::SupplierPayment,
                    method_id: Some(method.id.clone()),
                    method_kind: kind.to_string(),
                    amount: input.amount.checked_neg()?,
                    ref_type: Some("supplier_payment".into()),
                    ref_id: Some(payment.clone()),
                    reason: None,
                    at: clock.now(),
                    user_id: by.clone(),
                },
            )?;
        }
        let supplier = purchasing::get_supplier(tx, &input.supplier_id)?;
        audit_log::record(
            tx,
            clock,
            SUPPLIER_PAYMENT,
            serde_json::json!({
                "supplier_id": supplier.id, "name": supplier.name, "amount_minor": input.amount.0,
                "method": method.name, "reference": reference, "from_drawer": from_drawer,
            }),
            Some(by.clone()),
        )?;
        Ok((purchasing::balance(tx, &input.supplier_id)?, from_drawer))
    })?;
    Ok(SupplierPaymentDone {
        balance,
        drawer: from_drawer.then(|| receipts::open_drawer(state, devices)),
    })
}

/// The aging report on the till's business day (PUR-05).
pub fn aging(state: &AppState) -> Result<AgingView, AppError> {
    let clock = state.clock.as_ref();
    state.db.read(|conn| {
        let till: TillSettings = settings::get(conn)?;
        let today = business_date(
            clock.now(),
            clock.local_offset_minutes(),
            till.business_day_cutoff_hour,
        )?
        .0;
        Ok(AgingView {
            rows: purchasing::aging(conn, today)?,
            today,
            currency: settings::get(conn)?,
        })
    })
}

/// A supplier's statement for the days chosen, both included (PUR-06).
pub fn statement(
    state: &AppState,
    id: &Id,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Statement, AppError> {
    state
        .db
        .read(|conn| Ok(purchasing::statement(conn, id, from, to)?))
}

/// The statement as an Excel file (PUR-06). Gives the number of rows written.
pub fn export_statement(
    state: &AppState,
    id: &Id,
    from: NaiveDate,
    to: NaiveDate,
    file: &Path,
) -> Result<u32, AppError> {
    let found = statement(state, id, from, to)?;
    let currency: CurrencyFormat = state.db.read(|conn| Ok(settings::get(conn)?))?;
    let digits = currency.minor_digits;
    let mut rows = vec![vec![
        XlsxCell::Date(found.from),
        XlsxCell::Text(OPENING.into()),
        XlsxCell::Empty,
        XlsxCell::Empty,
        money_cell(found.opening, digits),
    ]];
    for line in &found.lines {
        rows.push(vec![
            XlsxCell::Date(line.business_date),
            XlsxCell::Text(line.kind.as_str().to_string()),
            XlsxCell::Text(line.reference.clone().unwrap_or_default()),
            money_cell(line.amount, digits),
            money_cell(line.balance_after, digits),
        ]);
    }
    rows.push(vec![
        XlsxCell::Date(found.to),
        XlsxCell::Text(CLOSING.into()),
        XlsxCell::Empty,
        XlsxCell::Empty,
        money_cell(found.closing, digits),
    ]);
    let written = u32::try_from(found.lines.len()).unwrap_or(u32::MAX);
    let sheet = XlsxSheet {
        name: found.supplier_name.clone(),
        headers: STATEMENT_HEADERS
            .iter()
            .map(|text| text.to_string())
            .collect(),
        rows,
    };
    io::write_xlsx(file, &[sheet])?;
    Ok(written)
}

/// Column headings of the exported order and statement. English constants, like the audit
/// export: they move next to `ReceiptLabels` when a second language comes (ADR-022).
const ORDER_HEADERS: [&str; 5] = ["SKU", "Product", "Quantity", "Unit cost", "Value"];
const STATEMENT_HEADERS: [&str; 5] = ["Date", "Kind", "Reference", "Amount", "Balance"];
const OPENING: &str = "Opening balance";
const CLOSING: &str = "Closing balance";

fn order_view(conn: &Connection, id: &Id) -> Result<OrderView, AppError> {
    let order = purchasing::get_order(conn, id)?;
    let supplier_name = purchasing::get_supplier(conn, &order.supplier_id)?.name;
    let mut line_values = Vec::with_capacity(order.lines.len());
    let mut total = Money::zero();
    for line in &order.lines {
        let value = mul_qty(line.unit_cost, line.qty)?;
        total = total.checked_add(value)?;
        line_values.push(value);
    }
    Ok(OrderView {
        order,
        supplier_name,
        line_values,
        total,
        currency: settings::get(conn)?,
    })
}

fn money_cell(amount: Money, minor_digits: u8) -> XlsxCell {
    XlsxCell::Money {
        amount,
        minor_digits,
    }
}

fn audit_supplier(
    tx: &Transaction,
    clock: &dyn Clock,
    by: &Id,
    id: &Id,
    name: &str,
    change: &str,
) -> Result<(), AppError> {
    audit_log::record(
        tx,
        clock,
        SUPPLIER_CHANGE,
        serde_json::json!({ "supplier_id": id, "name": name, "change": change }),
        Some(by.clone()),
    )
}

fn audit_order(
    tx: &Transaction,
    clock: &dyn Clock,
    by: &Id,
    id: &Id,
    change: &str,
) -> Result<(), AppError> {
    let order = purchasing::get_order(tx, id)?;
    audit_log::record(
        tx,
        clock,
        PURCHASE_ORDER_CHANGE,
        serde_json::json!({
            "order_id": id, "number": order.number, "status": order.status.as_str(),
            "change": change, "lines": order.lines.len(),
        }),
        Some(by.clone()),
    )
}

pub(crate) fn blank_to_none(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}
