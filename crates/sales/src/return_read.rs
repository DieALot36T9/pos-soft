//! Reading a saved return back, for the return receipt (RET-06) and reprints.

use pos_core::{Bps, BusinessDate, Id, Money, Qty, Timestamp};
use rusqlite::{Connection, OptionalExtension, Row, named_params};

use crate::refund::LineAmounts;
use crate::sale_read::payments_of;
use crate::sql::{
    get_bool, get_bps, get_business_date, get_id, get_money, get_optional_id, get_qty,
    get_timestamp,
};
use crate::{Payment, SalesError};

/// A saved return with its lines and refunds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Return {
    pub id: Id,
    /// Example: "T1-R000045".
    pub return_no: String,
    /// The original sale. `None` for a return without a receipt.
    pub sale_id: Option<Id>,
    pub till_id: Id,
    pub shift_id: Id,
    pub user_id: Id,
    pub approved_by: Option<Id>,
    pub business_date: BusinessDate,
    pub at: Timestamp,
    pub customer_id: Option<Id>,
    pub reason: Option<String>,
    pub totals: ReturnTotals,
    pub lines: Vec<ReturnLine>,
    pub payments: Vec<Payment>,
}

/// A return's totals. total = gross + rounding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReturnTotals {
    pub net: Money,
    pub tax: Money,
    pub gross: Money,
    pub rounding: Money,
    pub total: Money,
    pub cost: Money,
}

/// One saved return line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnLine {
    pub id: Id,
    pub sale_line_id: Option<Id>,
    pub product_id: Id,
    pub name: String,
    pub qty: Qty,
    pub amounts: LineAmounts,
    pub tax_rate_id: Id,
    pub tax_bps: Bps,
    pub unit_cost: Money,
    pub restock: bool,
}

/// One return with its lines and refunds.
pub fn get_return(conn: &Connection, return_id: &Id) -> Result<Return, SalesError> {
    let found = conn
        .query_row(
            "SELECT * FROM sales_returns WHERE id = :id",
            named_params! { ":id": return_id.as_str() },
            read_header,
        )
        .optional()?;
    let Some(mut found) = found else {
        return Err(SalesError::not_found("return", return_id.as_str()));
    };
    let mut statement =
        conn.prepare("SELECT * FROM sales_return_lines WHERE return_id = :id ORDER BY rowid")?;
    let rows = statement.query_map(named_params! { ":id": return_id.as_str() }, read_line)?;
    for row in rows {
        found.lines.push(row?);
    }
    found.payments = payments_of(conn, "return", return_id)?;
    Ok(found)
}

fn read_header(row: &Row) -> rusqlite::Result<Return> {
    Ok(Return {
        id: get_id(row, "id")?,
        return_no: row.get("return_no")?,
        sale_id: get_optional_id(row, "sale_id")?,
        till_id: get_id(row, "till_id")?,
        shift_id: get_id(row, "shift_id")?,
        user_id: get_id(row, "user_id")?,
        approved_by: get_optional_id(row, "approved_by")?,
        business_date: get_business_date(row, "business_date")?,
        at: get_timestamp(row, "at")?,
        customer_id: get_optional_id(row, "customer_id")?,
        reason: row.get("reason")?,
        totals: ReturnTotals {
            net: get_money(row, "net_minor")?,
            tax: get_money(row, "tax_minor")?,
            gross: get_money(row, "gross_minor")?,
            rounding: get_money(row, "rounding_minor")?,
            total: get_money(row, "total_minor")?,
            cost: get_money(row, "cost_minor")?,
        },
        lines: Vec::new(),
        payments: Vec::new(),
    })
}

fn read_line(row: &Row) -> rusqlite::Result<ReturnLine> {
    Ok(ReturnLine {
        id: get_id(row, "id")?,
        sale_line_id: get_optional_id(row, "sale_line_id")?,
        product_id: get_id(row, "product_id")?,
        name: row.get("name")?,
        qty: get_qty(row, "qty_milli")?,
        amounts: LineAmounts {
            net: get_money(row, "net_minor")?,
            tax: get_money(row, "tax_minor")?,
            gross: get_money(row, "gross_minor")?,
            cost: get_money(row, "cost_minor")?,
        },
        tax_rate_id: get_id(row, "tax_rate_id")?,
        tax_bps: get_bps(row, "tax_bps")?,
        unit_cost: get_money(row, "unit_cost_minor")?,
        restock: get_bool(row, "restock")?,
    })
}
