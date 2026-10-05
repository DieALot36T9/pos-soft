//! Reading completed sales back: by id, by invoice number, the last one on a till.

use std::collections::HashMap;

use pos_core::{AppError, Id, Qty, TaxMode};
use rusqlite::{Connection, OptionalExtension, Row, named_params};

use crate::sql::{
    get_bps, get_business_date, get_enum, get_id, get_money, get_optional_id, get_optional_money,
    get_optional_timestamp, get_qty, get_timestamp,
};
use crate::{
    CustomerSnapshot, LineSnapshot, Payment, PaymentKind, PromotionSnapshot, Sale, SaleLine,
    SaleStatus, SaleTotals, SalesError, TaxPart, VoidInfo,
};

/// One sale with its lines and payments.
pub fn get_sale(conn: &Connection, sale_id: &Id) -> Result<Sale, SalesError> {
    find_sale(conn, "id = :key", sale_id.as_str())?
        .ok_or_else(|| SalesError::not_found("sale", sale_id.as_str()))
}

/// The sale with this invoice number, such as "T1-000123" (RET-01). Spaces around it are ignored.
pub fn find_by_invoice(conn: &Connection, invoice_no: &str) -> Result<Option<Sale>, SalesError> {
    find_sale(conn, "invoice_no = :key", invoice_no.trim())
}

/// The sale saved last on this till, for "reprint last receipt" (PAY-10).
pub fn last_sale(conn: &Connection, till_id: &Id) -> Result<Option<Sale>, SalesError> {
    let sale_id = conn
        .query_row(
            // rowid is the order sales were saved in. Sales are never deleted, so it only
            // grows. Ids made in the same millisecond don't sort by creation, so they can't be used.
            "SELECT id FROM sales_sales WHERE till_id = :till ORDER BY rowid DESC LIMIT 1",
            named_params! { ":till": till_id.as_str() },
            |row| get_id(row, "id"),
        )
        .optional()?;
    match sale_id {
        Some(sale_id) => Ok(Some(get_sale(conn, &sale_id)?)),
        None => Ok(None),
    }
}

/// True once any sale or return has been saved. Settings uses it to lock the
/// currency's number of decimals, because stored amounts would change meaning.
pub fn has_any_sale(conn: &Connection) -> Result<bool, SalesError> {
    let found: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sales_sales) OR EXISTS (SELECT 1 FROM sales_returns)",
        [],
        |row| row.get(0),
    )?;
    Ok(found)
}

/// What the backup restore list shows about one database (BAK-05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalesSummary {
    /// Every sale saved, voided ones included (they keep their number).
    pub sale_count: u32,
    /// The invoice number of the sale saved last. Example: "T1-000123".
    pub last_invoice_no: Option<String>,
}

/// The number of sales and the last invoice number, for the backup restore list (BAK-05).
/// `app` opens the backup with `db::open_read_only` and passes that connection here.
/// A backup made before the sales tables existed has no sales.
pub fn sales_summary(conn: &Connection) -> Result<SalesSummary, SalesError> {
    let has_sales_table: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'sales_sales')",
        [],
        |row| row.get(0),
    )?;
    if !has_sales_table {
        return Ok(SalesSummary {
            sale_count: 0,
            last_invoice_no: None,
        });
    }
    let sale_count: u32 =
        conn.query_row("SELECT count(*) FROM sales_sales", [], |row| row.get(0))?;
    // rowid is the order sales were saved in (see `last_sale`).
    let last_invoice_no: Option<String> = conn
        .query_row(
            "SELECT invoice_no FROM sales_sales ORDER BY rowid DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(SalesSummary {
        sale_count,
        last_invoice_no,
    })
}

/// For each line of a sale: the quantity sold minus what was already returned (RET-01).
pub fn returnable_qty(conn: &Connection, sale_id: &Id) -> Result<HashMap<Id, Qty>, SalesError> {
    // Make sure the sale exists, so a wrong id isn't mistaken for "nothing to return".
    get_sale_status(conn, sale_id)?;
    let mut statement = conn.prepare(
        "SELECT l.id, l.qty_milli - coalesce((SELECT sum(r.qty_milli) FROM sales_return_lines r
                                              WHERE r.sale_line_id = l.id), 0) AS left_milli
         FROM sales_lines l WHERE l.sale_id = :sale",
    )?;
    let rows = statement.query_map(named_params! { ":sale": sale_id.as_str() }, |row| {
        Ok((get_id(row, "id")?, get_qty(row, "left_milli")?))
    })?;
    let mut left = HashMap::new();
    for row in rows {
        let (line_id, qty) = row?;
        left.insert(line_id, qty);
    }
    Ok(left)
}

/// A sale's status, or NotFound.
pub(crate) fn get_sale_status(conn: &Connection, sale_id: &Id) -> Result<SaleStatus, SalesError> {
    conn.query_row(
        "SELECT status FROM sales_sales WHERE id = :id",
        named_params! { ":id": sale_id.as_str() },
        |row| get_enum(row, "status", SaleStatus::parse),
    )
    .optional()?
    .ok_or_else(|| SalesError::not_found("sale", sale_id.as_str()))
}

fn find_sale(conn: &Connection, condition: &str, key: &str) -> Result<Option<Sale>, SalesError> {
    // `condition` is always one of our own fixed strings, never text from the user.
    let sql = format!("SELECT * FROM sales_sales WHERE {condition}");
    let sale = conn
        .query_row(&sql, named_params! { ":key": key }, read_header)
        .optional()?;
    let Some(mut sale) = sale else {
        return Ok(None);
    };
    sale.lines = sale_lines(conn, &sale.id)?;
    sale.payments = payments_of(conn, "sale", &sale.id)?;
    Ok(Some(sale))
}

fn read_header(row: &Row) -> rusqlite::Result<Sale> {
    let customer = match get_optional_id(row, "customer_id")? {
        Some(id) => Some(CustomerSnapshot {
            id,
            name: row
                .get::<_, Option<String>>("customer_name")?
                .unwrap_or_default(),
            phone: row
                .get::<_, Option<String>>("customer_phone")?
                .unwrap_or_default(),
        }),
        None => None,
    };
    let void = match (
        get_optional_timestamp(row, "voided_at")?,
        get_optional_id(row, "voided_by")?,
    ) {
        (Some(at), Some(by)) => Some(VoidInfo {
            at,
            by,
            approved_by: get_optional_id(row, "void_approved_by")?,
            reason: row
                .get::<_, Option<String>>("void_reason")?
                .unwrap_or_default(),
        }),
        _ => None,
    };
    Ok(Sale {
        id: get_id(row, "id")?,
        invoice_no: row.get("invoice_no")?,
        till_id: get_id(row, "till_id")?,
        shift_id: get_id(row, "shift_id")?,
        user_id: get_id(row, "user_id")?,
        business_date: get_business_date(row, "business_date")?,
        completed_at: get_timestamp(row, "completed_at")?,
        customer,
        price_list_id: get_optional_id(row, "price_list_id")?,
        status: get_enum(row, "status", SaleStatus::parse)?,
        tax_mode: get_enum(row, "tax_mode", TaxMode::parse)?,
        exchange_return_id: get_optional_id(row, "exchange_return_id")?,
        totals: read_totals(row)?,
        void,
        lines: Vec::new(),
        payments: Vec::new(),
    })
}

fn read_totals(row: &Row) -> rusqlite::Result<SaleTotals> {
    Ok(SaleTotals {
        base: get_money(row, "base_minor")?,
        promo_discount: get_money(row, "promo_discount_minor")?,
        tier_discount: get_money(row, "tier_discount_minor")?,
        manual_discount: get_money(row, "manual_discount_minor")?,
        invoice_discount: get_money(row, "invoice_discount_minor")?,
        net: get_money(row, "net_minor")?,
        tax: get_money(row, "tax_minor")?,
        gross: get_money(row, "gross_minor")?,
        rounding: get_money(row, "rounding_minor")?,
        total: get_money(row, "total_minor")?,
        change: get_money(row, "change_minor")?,
        cost: get_money(row, "cost_minor")?,
    })
}

/// A sale's lines in order.
pub(crate) fn sale_lines(conn: &Connection, sale_id: &Id) -> Result<Vec<SaleLine>, SalesError> {
    let mut statement =
        conn.prepare("SELECT * FROM sales_lines WHERE sale_id = :sale ORDER BY seq")?;
    let rows = statement.query_map(named_params! { ":sale": sale_id.as_str() }, read_line)?;
    let mut lines = Vec::new();
    for row in rows {
        lines.push(row?);
    }
    Ok(lines)
}

fn read_line(row: &Row) -> rusqlite::Result<SaleLine> {
    let promotion = match get_optional_id(row, "promotion_id")? {
        Some(id) => Some(PromotionSnapshot {
            id,
            name: row
                .get::<_, Option<String>>("promotion_name")?
                .unwrap_or_default(),
        }),
        None => None,
    };
    let components_json: String = row.get("tax_components_json")?;
    let tax_components: Vec<TaxPart> = serde_json::from_str(&components_json).map_err(|error| {
        let error = AppError::internal(format!("tax_components_json: {error}"));
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(SaleLine {
        id: get_id(row, "id")?,
        seq: row.get("seq")?,
        cost: get_money(row, "cost_minor")?,
        snapshot: LineSnapshot {
            product_id: get_id(row, "product_id")?,
            name: row.get("name")?,
            sku: row.get("sku")?,
            unit_symbol: row.get("unit_symbol")?,
            category_id: get_id(row, "category_id")?,
            brand_id: get_optional_id(row, "brand_id")?,
            qty: get_qty(row, "qty_milli")?,
            unit_price: get_money(row, "unit_price_minor")?,
            retail_price: get_money(row, "retail_price_minor")?,
            base: get_money(row, "base_minor")?,
            promotion,
            promo_discount: get_money(row, "promo_discount_minor")?,
            tier_discount: get_money(row, "tier_discount_minor")?,
            manual_discount: get_money(row, "manual_discount_minor")?,
            invoice_discount: get_money(row, "invoice_discount_minor")?,
            net: get_money(row, "net_minor")?,
            tax: get_money(row, "tax_minor")?,
            gross: get_money(row, "gross_minor")?,
            tax_rate_id: get_id(row, "tax_rate_id")?,
            tax_name: row.get("tax_name")?,
            tax_bps: get_bps(row, "tax_bps")?,
            tax_components,
            unit_cost: get_money(row, "unit_cost_minor")?,
            price_overridden: row.get::<_, i64>("price_overridden")? != 0,
        },
    })
}

/// The payments of a sale or the refunds of a return, in the order they were saved.
pub(crate) fn payments_of(
    conn: &Connection,
    doc_type: &str,
    doc_id: &Id,
) -> Result<Vec<Payment>, SalesError> {
    let mut statement = conn.prepare(
        "SELECT * FROM sales_payments WHERE doc_type = :doc_type AND doc_id = :doc_id ORDER BY rowid",
    )?;
    let rows = statement.query_map(
        named_params! { ":doc_type": doc_type, ":doc_id": doc_id.as_str() },
        |row| {
            Ok(Payment {
                id: get_id(row, "id")?,
                method_id: get_id(row, "method_id")?,
                kind: get_enum(row, "kind", PaymentKind::parse)?,
                amount: get_money(row, "amount_minor")?,
                tendered: get_optional_money(row, "tendered_minor")?,
                reference: row.get("reference")?,
                at: get_timestamp(row, "at")?,
            })
        },
    )?;
    let mut payments = Vec::new();
    for row in rows {
        payments.push(row?);
    }
    Ok(payments)
}
