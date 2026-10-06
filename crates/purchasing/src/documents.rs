//! Lists of saved documents for the purchasing screens: orders, deliveries (GRNs) and
//! supplier returns, newest first, and one delivery with its lines.

use chrono::NaiveDate;
use pos_core::{Id, Money, Qty, Timestamp, mul_qty};
use rusqlite::{Connection, OptionalExtension, Row, named_params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::sql::{read_date, read_id, read_optional_date, read_optional_id};
use crate::{OrderStatus, PurchasingError};

/// One purchase order in the list (PUR-02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrderRow {
    pub id: Id,
    /// Example: "PO-000001".
    pub number: String,
    pub supplier_id: Id,
    pub supplier_name: String,
    pub status: OrderStatus,
    pub expected_date: Option<NaiveDate>,
    pub created_at: Timestamp,
    pub line_count: u32,
    /// Σ round(expected unit cost × quantity) over the lines.
    pub total: Money,
}

/// One delivery in the list (PUR-03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GrnRow {
    pub id: Id,
    /// Example: "GRN-000001".
    pub number: String,
    pub supplier_id: Id,
    pub supplier_name: String,
    /// The order it filled, if any. Example: "PO-000003".
    pub order_number: Option<String>,
    pub supplier_invoice_no: Option<String>,
    pub invoice_total: Money,
    pub lines_total: Money,
    pub received_at: Timestamp,
    pub business_date: NaiveDate,
}

/// A saved delivery with its lines, for a supplier return against it (PUR-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Grn {
    pub row: GrnRow,
    pub notes: Option<String>,
    pub lines: Vec<GrnLine>,
}

/// One saved delivery line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GrnLine {
    pub id: Id,
    pub order_line_id: Option<Id>,
    pub product_id: Id,
    pub product_name: String,
    pub qty: Qty,
    pub unit_cost: Money,
    pub batch_no: Option<String>,
    pub expiry_date: Option<NaiveDate>,
    pub inventory_batch_id: Option<Id>,
}

/// One supplier return in the list (PUR-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReturnRow {
    pub id: Id,
    /// Example: "SR-000001".
    pub number: String,
    pub supplier_id: Id,
    pub supplier_name: String,
    /// The delivery the goods came in on, if one was picked. Example: "GRN-000002".
    pub grn_number: Option<String>,
    pub total: Money,
    pub reason: Option<String>,
    pub at: Timestamp,
    pub line_count: u32,
}

/// Purchase orders, newest first; only those with `status` when it is given.
pub fn list_orders(
    connection: &Connection,
    status: Option<OrderStatus>,
) -> Result<Vec<OrderRow>, PurchasingError> {
    let mut statement = connection.prepare(
        "SELECT o.id, o.number, o.supplier_id, s.name AS supplier_name, o.status,
                o.expected_date, o.created_at
         FROM purchasing_orders o JOIN purchasing_suppliers s ON s.id = o.supplier_id
         WHERE :status IS NULL OR o.status = :status
         ORDER BY o.created_at DESC, o.number DESC",
    )?;
    let rows = statement.query_map(
        named_params! { ":status": status.map(OrderStatus::as_str) },
        |row| {
            let status: String = row.get("status")?;
            Ok(OrderRow {
                id: read_id(row, "id")?,
                number: row.get("number")?,
                supplier_id: read_id(row, "supplier_id")?,
                supplier_name: row.get("supplier_name")?,
                status: OrderStatus::parse(&status)?,
                expected_date: read_optional_date(row, "expected_date")?,
                created_at: Timestamp(row.get("created_at")?),
                line_count: 0,
                total: Money::zero(),
            })
        },
    )?;
    let mut orders = Vec::new();
    for row in rows {
        let mut order = row?;
        let (count, total) = order_value(connection, &order.id)?;
        order.line_count = count;
        order.total = total;
        orders.push(order);
    }
    Ok(orders)
}

/// Deliveries, newest first; only one supplier's when `supplier` is given.
pub fn list_grns(
    connection: &Connection,
    supplier: Option<&Id>,
) -> Result<Vec<GrnRow>, PurchasingError> {
    let mut statement = connection.prepare(&format!(
        "{GRN_SELECT} WHERE :supplier IS NULL OR g.supplier_id = :supplier
         ORDER BY g.received_at DESC, g.number DESC"
    ))?;
    let rows = statement.query_map(
        named_params! { ":supplier": supplier.map(Id::as_str) },
        grn_row,
    )?;
    let mut found = Vec::new();
    for row in rows {
        found.push(row?);
    }
    Ok(found)
}

/// One delivery with its lines, in the order they were entered.
pub fn get_grn(connection: &Connection, grn: &Id) -> Result<Grn, PurchasingError> {
    let header = connection
        .query_row(
            &format!("{GRN_SELECT} WHERE g.id = :id"),
            named_params! { ":id": grn.as_str() },
            |row| Ok((grn_row(row)?, row.get::<_, Option<String>>("notes")?)),
        )
        .optional()?;
    let (row, notes) = header.ok_or_else(|| PurchasingError::not_found("grn", grn))?;
    let mut statement = connection
        .prepare("SELECT * FROM purchasing_grn_lines WHERE grn_id = :grn ORDER BY rowid")?;
    let lines = statement.query_map(named_params! { ":grn": grn.as_str() }, |row| {
        Ok(GrnLine {
            id: read_id(row, "id")?,
            order_line_id: read_optional_id(row, "order_line_id")?,
            product_id: read_id(row, "product_id")?,
            product_name: row.get("product_name")?,
            qty: Qty(row.get("qty_milli")?),
            unit_cost: Money(row.get("unit_cost_minor")?),
            batch_no: row.get("batch_no")?,
            expiry_date: read_optional_date(row, "expiry_date")?,
            inventory_batch_id: read_optional_id(row, "inventory_batch_id")?,
        })
    })?;
    let mut saved = Vec::new();
    for line in lines {
        saved.push(line?);
    }
    Ok(Grn {
        row,
        notes,
        lines: saved,
    })
}

/// Supplier returns, newest first.
pub fn list_returns(connection: &Connection) -> Result<Vec<ReturnRow>, PurchasingError> {
    let mut statement = connection.prepare(
        "SELECT r.id, r.number, r.supplier_id, s.name AS supplier_name, g.number AS grn_number,
                r.total_minor, r.reason, r.at,
                (SELECT count(*) FROM purchasing_return_lines l WHERE l.return_id = r.id) AS lines
         FROM purchasing_returns r
         JOIN purchasing_suppliers s ON s.id = r.supplier_id
         LEFT JOIN purchasing_grns g ON g.id = r.grn_id
         ORDER BY r.at DESC, r.number DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(ReturnRow {
            id: read_id(row, "id")?,
            number: row.get("number")?,
            supplier_id: read_id(row, "supplier_id")?,
            supplier_name: row.get("supplier_name")?,
            grn_number: row.get("grn_number")?,
            total: Money(row.get("total_minor")?),
            reason: row.get("reason")?,
            at: Timestamp(row.get("at")?),
            line_count: row.get("lines")?,
        })
    })?;
    let mut found = Vec::new();
    for row in rows {
        found.push(row?);
    }
    Ok(found)
}

const GRN_SELECT: &str = "SELECT g.id, g.number, g.supplier_id, s.name AS supplier_name,
        o.number AS order_number, g.supplier_invoice_no, g.invoice_total_minor,
        g.lines_total_minor, g.received_at, g.business_date, g.notes
     FROM purchasing_grns g
     JOIN purchasing_suppliers s ON s.id = g.supplier_id
     LEFT JOIN purchasing_orders o ON o.id = g.order_id";

fn grn_row(row: &Row) -> rusqlite::Result<GrnRow> {
    Ok(GrnRow {
        id: read_id(row, "id")?,
        number: row.get("number")?,
        supplier_id: read_id(row, "supplier_id")?,
        supplier_name: row.get("supplier_name")?,
        order_number: row.get("order_number")?,
        supplier_invoice_no: row.get("supplier_invoice_no")?,
        invoice_total: Money(row.get("invoice_total_minor")?),
        lines_total: Money(row.get("lines_total_minor")?),
        received_at: Timestamp(row.get("received_at")?),
        business_date: read_date(row, "business_date")?,
    })
}

/// How many lines an order has and their value, worked out in Rust (no SQLite sums of money).
fn order_value(connection: &Connection, order: &Id) -> Result<(u32, Money), PurchasingError> {
    let mut statement = connection.prepare(
        "SELECT qty_milli, unit_cost_minor FROM purchasing_order_lines WHERE order_id = :order",
    )?;
    let rows = statement.query_map(named_params! { ":order": order.as_str() }, |row| {
        Ok((Qty(row.get(0)?), Money(row.get(1)?)))
    })?;
    let mut count = 0u32;
    let mut total = Money::zero();
    for row in rows {
        let (qty, cost) = row?;
        count = count.saturating_add(1);
        total = total.checked_add(mul_qty(cost, qty)?)?;
    }
    Ok((count, total))
}
