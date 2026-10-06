//! PUR-02 Purchase orders: Draft → Sent → Partially received → Received, or Cancelled.

use chrono::NaiveDate;
use pos_core::{Id, Money, Qty, Timestamp};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::sql::{
    check_line, clean_text, date_text, next_number, read_id, read_optional_date,
    read_optional_timestamp, require_active_supplier, require_lines,
};
use crate::{PurchasingCtx, PurchasingError};

/// Where a purchase order is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OrderStatus {
    Draft,
    Sent,
    Partial,
    Received,
    Cancelled,
}

impl OrderStatus {
    /// The value stored in `purchasing_orders.status`.
    pub fn as_str(self) -> &'static str {
        match self {
            OrderStatus::Draft => "draft",
            OrderStatus::Sent => "sent",
            OrderStatus::Partial => "partial",
            OrderStatus::Received => "received",
            OrderStatus::Cancelled => "cancelled",
        }
    }

    pub(crate) fn parse(text: &str) -> rusqlite::Result<OrderStatus> {
        match text {
            "draft" => Ok(OrderStatus::Draft),
            "sent" => Ok(OrderStatus::Sent),
            "partial" => Ok(OrderStatus::Partial),
            "received" => Ok(OrderStatus::Received),
            "cancelled" => Ok(OrderStatus::Cancelled),
            other => Err(rusqlite::Error::InvalidColumnType(
                0,
                format!("status {other}"),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

/// One line the user wants to order. Name and SKU are copied from the catalog by `app`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NewOrderLine {
    pub product_id: Id,
    pub product_name: String,
    pub sku: String,
    pub qty: Qty,
    /// Expected cost for one unit.
    pub unit_cost: Money,
}

/// A purchase order as the user fills it in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NewOrder {
    pub supplier_id: Id,
    pub expected_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub lines: Vec<NewOrderLine>,
}

/// The id and number of a saved order. Example number: "PO-000001".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrderRef {
    pub id: Id,
    pub number: String,
}

/// A saved order line, with how much has arrived so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrderLine {
    pub id: Id,
    pub product_id: Id,
    pub product_name: String,
    pub sku: String,
    pub qty: Qty,
    pub unit_cost: Money,
    pub received: Qty,
}

/// A saved purchase order with its lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Order {
    pub id: Id,
    pub number: String,
    pub supplier_id: Id,
    pub status: OrderStatus,
    pub expected_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub created_at: Timestamp,
    pub created_by: Id,
    pub sent_at: Option<Timestamp>,
    pub lines: Vec<OrderLine>,
}

/// One order line for the receiving screen. `remaining` is never below zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrderLineRemaining {
    pub order_line_id: Id,
    pub product_id: Id,
    pub product_name: String,
    pub sku: String,
    pub ordered: Qty,
    pub received: Qty,
    pub remaining: Qty,
    pub unit_cost: Money,
}

/// Saves a new draft purchase order with the next PO number.
pub fn create_order(
    tx: &Transaction,
    order: &NewOrder,
    ctx: &PurchasingCtx,
) -> Result<OrderRef, PurchasingError> {
    check_order(tx, order)?;
    let id = Id::new();
    let number = next_number(tx, "po", "PO")?;
    tx.execute(
        "INSERT INTO purchasing_orders (id, number, supplier_id, status, expected_date, notes,
           created_at, created_by)
         VALUES (:id, :number, :supplier_id, 'draft', :expected_date, :notes, :at, :user_id)",
        named_params! {
            ":id": id.as_str(),
            ":number": number,
            ":supplier_id": order.supplier_id.as_str(),
            ":expected_date": order.expected_date.map(date_text),
            ":notes": clean_text(order.notes.as_deref()),
            ":at": ctx.at.0,
            ":user_id": ctx.user_id.as_str(),
        },
    )?;
    insert_lines(tx, &id, &order.lines)?;
    Ok(OrderRef { id, number })
}

/// Replaces a draft order's supplier, date, notes and lines. Sent orders can't change.
pub fn update_order(
    tx: &Transaction,
    order: &Id,
    changes: &NewOrder,
) -> Result<(), PurchasingError> {
    let (_, status) = order_header(tx, order)?;
    if status != OrderStatus::Draft {
        return Err(PurchasingError::Conflict {
            code: "order_not_draft",
        });
    }
    check_order(tx, changes)?;
    tx.execute(
        "UPDATE purchasing_orders SET supplier_id = :supplier_id, expected_date = :expected_date,
           notes = :notes WHERE id = :id",
        named_params! {
            ":id": order.as_str(),
            ":supplier_id": changes.supplier_id.as_str(),
            ":expected_date": changes.expected_date.map(date_text),
            ":notes": clean_text(changes.notes.as_deref()),
        },
    )?;
    // A draft has received nothing, so its lines can simply be replaced.
    tx.execute(
        "DELETE FROM purchasing_order_lines WHERE order_id = :id",
        named_params! { ":id": order.as_str() },
    )?;
    insert_lines(tx, order, &changes.lines)
}

/// Marks a draft order as sent (printed, saved as PDF or exported).
/// Sending again later is allowed and changes nothing.
pub fn mark_sent(tx: &Transaction, order: &Id, now: Timestamp) -> Result<(), PurchasingError> {
    let (_, status) = order_header(tx, order)?;
    match status {
        OrderStatus::Draft => {
            tx.execute(
                "UPDATE purchasing_orders SET status = 'sent', sent_at = :now WHERE id = :id",
                named_params! { ":id": order.as_str(), ":now": now.0 },
            )?;
            Ok(())
        }
        OrderStatus::Sent | OrderStatus::Partial | OrderStatus::Received => Ok(()),
        OrderStatus::Cancelled => Err(PurchasingError::Conflict {
            code: "order_cancelled",
        }),
    }
}

/// Cancels an order that has received nothing yet.
pub fn cancel_order(tx: &Transaction, order: &Id) -> Result<(), PurchasingError> {
    let (_, status) = order_header(tx, order)?;
    match status {
        OrderStatus::Draft | OrderStatus::Sent => {
            tx.execute(
                "UPDATE purchasing_orders SET status = 'cancelled' WHERE id = :id",
                named_params! { ":id": order.as_str() },
            )?;
            Ok(())
        }
        OrderStatus::Partial | OrderStatus::Received => Err(PurchasingError::Conflict {
            code: "order_has_receipts",
        }),
        OrderStatus::Cancelled => Ok(()),
    }
}

/// One purchase order with its lines, in the order they were entered.
pub fn get_order(connection: &Connection, order: &Id) -> Result<Order, PurchasingError> {
    let header = connection
        .query_row(
            "SELECT * FROM purchasing_orders WHERE id = :id",
            named_params! { ":id": order.as_str() },
            order_from_row,
        )
        .optional()?;
    let mut saved = header.ok_or_else(|| PurchasingError::not_found("order", order))?;
    saved.lines = order_lines(connection, order)?;
    Ok(saved)
}

/// The order's lines with what is still to come, to pre-fill the receiving screen.
pub fn order_for_receiving(
    connection: &Connection,
    order: &Id,
) -> Result<Vec<OrderLineRemaining>, PurchasingError> {
    order_header(connection, order)?;
    let mut remaining = Vec::new();
    for line in order_lines(connection, order)? {
        let left = line.qty.0.saturating_sub(line.received.0).max(0);
        remaining.push(OrderLineRemaining {
            order_line_id: line.id,
            product_id: line.product_id,
            product_name: line.product_name,
            sku: line.sku,
            ordered: line.qty,
            received: line.received,
            remaining: Qty(left),
            unit_cost: line.unit_cost,
        });
    }
    Ok(remaining)
}

/// The order's supplier and status, or "not found".
pub(crate) fn order_header(
    connection: &Connection,
    order: &Id,
) -> Result<(Id, OrderStatus), PurchasingError> {
    connection
        .query_row(
            "SELECT supplier_id, status FROM purchasing_orders WHERE id = :id",
            named_params! { ":id": order.as_str() },
            |row| {
                let status: String = row.get("status")?;
                Ok((read_id(row, "supplier_id")?, OrderStatus::parse(&status)?))
            },
        )
        .optional()?
        .ok_or_else(|| PurchasingError::not_found("order", order))
}

/// Adds received quantity to one line of `order`. Returns true when the line now has
/// more than was ordered (allowed, but the GRN flags it).
pub(crate) fn add_received(
    tx: &Transaction,
    order: &Id,
    order_line: &Id,
    qty: Qty,
) -> Result<bool, PurchasingError> {
    let over: Option<bool> = tx
        .query_row(
            "UPDATE purchasing_order_lines SET received_milli = received_milli + :qty
             WHERE id = :line AND order_id = :order
             RETURNING received_milli > qty_milli",
            named_params! { ":qty": qty.0, ":line": order_line.as_str(), ":order": order.as_str() },
            |row| row.get(0),
        )
        .optional()?;
    over.ok_or(PurchasingError::Invalid {
        field: "order_line_id",
        code: "order_line_not_on_order",
    })
}

/// Sets the status from the received quantities: Received when every line is fully
/// received, otherwise Partial. Called after a GRN.
pub(crate) fn refresh_status(tx: &Transaction, order: &Id) -> Result<OrderStatus, PurchasingError> {
    let lines_short: i64 = tx.query_row(
        "SELECT COUNT(*) FROM purchasing_order_lines
         WHERE order_id = :order AND received_milli < qty_milli",
        named_params! { ":order": order.as_str() },
        |row| row.get(0),
    )?;
    let status = if lines_short == 0 {
        OrderStatus::Received
    } else {
        OrderStatus::Partial
    };
    tx.execute(
        "UPDATE purchasing_orders SET status = :status WHERE id = :order",
        named_params! { ":status": status.as_str(), ":order": order.as_str() },
    )?;
    Ok(status)
}

fn check_order(connection: &Connection, order: &NewOrder) -> Result<(), PurchasingError> {
    require_active_supplier(connection, &order.supplier_id)?;
    require_lines(order.lines.len())?;
    for line in &order.lines {
        check_line(line.qty, line.unit_cost)?;
    }
    Ok(())
}

fn insert_lines(
    tx: &Transaction,
    order: &Id,
    lines: &[NewOrderLine],
) -> Result<(), PurchasingError> {
    let mut statement = tx.prepare(
        "INSERT INTO purchasing_order_lines (id, order_id, product_id, product_name, sku,
           qty_milli, unit_cost_minor)
         VALUES (:id, :order_id, :product_id, :product_name, :sku, :qty, :unit_cost)",
    )?;
    for line in lines {
        statement.execute(named_params! {
            ":id": Id::new().as_str(),
            ":order_id": order.as_str(),
            ":product_id": line.product_id.as_str(),
            ":product_name": line.product_name,
            ":sku": line.sku,
            ":qty": line.qty.0,
            ":unit_cost": line.unit_cost.0,
        })?;
    }
    Ok(())
}

fn order_lines(connection: &Connection, order: &Id) -> Result<Vec<OrderLine>, PurchasingError> {
    // SQLite's rowid goes up with each insert, so this keeps the order the lines were entered in.
    let mut statement = connection
        .prepare("SELECT * FROM purchasing_order_lines WHERE order_id = :order ORDER BY rowid")?;
    let rows = statement.query_map(named_params! { ":order": order.as_str() }, |row| {
        Ok(OrderLine {
            id: read_id(row, "id")?,
            product_id: read_id(row, "product_id")?,
            product_name: row.get("product_name")?,
            sku: row.get("sku")?,
            qty: Qty(row.get("qty_milli")?),
            unit_cost: Money(row.get("unit_cost_minor")?),
            received: Qty(row.get("received_milli")?),
        })
    })?;
    let mut lines = Vec::new();
    for line in rows {
        lines.push(line?);
    }
    Ok(lines)
}

fn order_from_row(row: &Row) -> rusqlite::Result<Order> {
    let status: String = row.get("status")?;
    Ok(Order {
        id: read_id(row, "id")?,
        number: row.get("number")?,
        supplier_id: read_id(row, "supplier_id")?,
        status: OrderStatus::parse(&status)?,
        expected_date: read_optional_date(row, "expected_date")?,
        notes: row.get("notes")?,
        created_at: Timestamp(row.get("created_at")?),
        created_by: read_id(row, "created_by")?,
        sent_at: read_optional_timestamp(row, "sent_at")?,
        lines: Vec::new(),
    })
}
