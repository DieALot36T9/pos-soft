//! Carts: what they look like, and the functions that read them.
//! A cart holds product ids and quantities only. Prices are worked out by the
//! workflow every time, until the sale completes (Workflows, "Reprice cart").

use pos_core::{Bps, Id, Money, Qty, Timestamp};
use rusqlite::{Connection, OptionalExtension, Row, named_params};

use crate::SalesError;
use crate::sql::{get_enum, get_id, get_optional_id, get_optional_money, get_qty, get_timestamp};

/// Whether a cart is being rung up now, or parked for later (CHK-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CartStatus {
    /// The cashier is working on it.
    Open,
    /// Put aside so the next customer can be served.
    Held,
}

impl CartStatus {
    pub(crate) fn parse(text: &str) -> Option<CartStatus> {
        match text {
            "open" => Some(CartStatus::Open),
            "held" => Some(CartStatus::Held),
            _ => None,
        }
    }
}

/// How a line got into the cart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineSource {
    /// A barcode or SKU was scanned or typed.
    Scan,
    /// Picked from the search list.
    Search,
    /// A scale sticker with the weight or price in the barcode (CHK-08).
    ScaleLabel,
    /// Typed in by hand, for example a weight entered in the weight dialog.
    Manual,
}

impl LineSource {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            LineSource::Scan => "scan",
            LineSource::Search => "search",
            LineSource::ScaleLabel => "scale_label",
            LineSource::Manual => "manual",
        }
    }

    pub(crate) fn parse(text: &str) -> Option<LineSource> {
        match text {
            "scan" => Some(LineSource::Scan),
            "search" => Some(LineSource::Search),
            "scale_label" => Some(LineSource::ScaleLabel),
            "manual" => Some(LineSource::Manual),
            _ => None,
        }
    }
}

/// A discount typed by the cashier (CHK-12). Pricing turns it into an amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discount {
    /// A percentage. `Percent(Bps(500))` is 5%.
    Percent(Bps),
    /// A fixed amount off. `Amount(Money(1000))` is 10.00 off.
    Amount(Money),
}

impl Discount {
    /// The two columns a discount is stored in: kind and value.
    pub(crate) fn to_columns(discount: Option<Discount>) -> (Option<&'static str>, Option<i64>) {
        match discount {
            Some(Discount::Percent(rate)) => (Some("percent"), Some(i64::from(rate.0))),
            Some(Discount::Amount(amount)) => (Some("amount"), Some(amount.0)),
            None => (None, None),
        }
    }

    /// Reads a discount back from its kind and value columns.
    fn from_columns(
        row: &Row,
        kind_column: &str,
        value_column: &str,
    ) -> rusqlite::Result<Option<Discount>> {
        let kind: Option<String> = row.get(kind_column)?;
        let value: Option<i64> = row.get(value_column)?;
        let discount = match (kind.as_deref(), value) {
            (Some("percent"), Some(value)) => {
                Some(Discount::Percent(Bps(u32::try_from(value).unwrap_or(0))))
            }
            (Some("amount"), Some(value)) => Some(Discount::Amount(Money(value))),
            _ => None,
        };
        Ok(discount)
    }
}

/// A unit price typed in by someone with `PriceOverride` (CHK-13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceOverride {
    /// The new price for one unit.
    pub price: Money,
    /// Who changed it, for the audit log and reports.
    pub by: Id,
}

/// A cart in progress or on hold, with its lines in the order they were added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cart {
    pub id: Id,
    pub till_id: Id,
    pub shift_id: Id,
    pub user_id: Id,
    pub status: CartStatus,
    pub customer_id: Option<Id>,
    pub price_list_id: Option<Id>,
    /// The note typed when the cart was held. Example: "blue shirt, forgot rice".
    pub note: Option<String>,
    pub invoice_discount: Option<Discount>,
    /// Set when this cart is the new half of an exchange (RET-05).
    pub exchange_return_id: Option<Id>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub lines: Vec<CartLine>,
}

/// One product row in a cart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartLine {
    pub id: Id,
    /// 1, 2, 3, … in the order lines were added.
    pub seq: u32,
    pub product_id: Id,
    pub qty: Qty,
    pub source: LineSource,
    /// The price printed on a price-embedded scale sticker. The line costs exactly this.
    pub embedded_price: Option<Money>,
    pub price_override: Option<PriceOverride>,
    pub manual_discount: Option<Discount>,
    pub added_at: Timestamp,
}

/// What the workflow sends to add a product to the cart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCartLine {
    pub product_id: Id,
    /// Must be more than zero. Example: `Qty(1250)` is 1.250 kg.
    pub qty: Qty,
    pub source: LineSource,
    pub embedded_price: Option<Money>,
}

/// One row of the held carts list (CHK-10). The workflow adds the total by repricing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldCartRow {
    pub cart_id: Id,
    /// When the cart was held.
    pub held_at: Timestamp,
    /// The cashier who held it.
    pub user_id: Id,
    pub customer_id: Option<Id>,
    pub note: Option<String>,
    /// How many lines the cart has.
    pub line_count: u32,
}

const CART_COLUMNS: &str =
    "id, till_id, shift_id, user_id, status, customer_id, price_list_id, note,
    invoice_discount_kind, invoice_discount_value, exchange_return_id, created_at, updated_at";

/// Reads one cart and its lines.
pub fn get_cart(conn: &Connection, cart_id: &Id) -> Result<Cart, SalesError> {
    let sql = format!("SELECT {CART_COLUMNS} FROM sales_carts WHERE id = :id");
    let cart = conn
        .query_row(
            &sql,
            named_params! { ":id": cart_id.as_str() },
            read_cart_row,
        )
        .optional()?;
    let Some(mut cart) = cart else {
        return Err(SalesError::not_found("cart", cart_id.as_str()));
    };
    cart.lines = cart_lines(conn, cart_id)?;
    Ok(cart)
}

/// The cart this cashier is working on at this till, if there is one.
/// After a crash or restart this brings the cart back exactly as it was (CHK-19).
pub fn current_cart(
    conn: &Connection,
    till_id: &Id,
    user_id: &Id,
) -> Result<Option<Cart>, SalesError> {
    let cart_id: Option<String> = conn
        .query_row(
            "SELECT id FROM sales_carts WHERE till_id = :till AND user_id = :user AND status = 'open'",
            named_params! { ":till": till_id.as_str(), ":user": user_id.as_str() },
            |row| row.get(0),
        )
        .optional()?;
    match cart_id {
        Some(cart_id) => Ok(Some(get_cart(conn, &Id::parse(&cart_id)?)?)),
        None => Ok(None),
    }
}

/// The carts on hold at this till, oldest first (CHK-10).
pub fn held_carts(conn: &Connection, till_id: &Id) -> Result<Vec<HeldCartRow>, SalesError> {
    let mut statement = conn.prepare(
        "SELECT c.id, c.updated_at, c.user_id, c.customer_id, c.note,
                (SELECT count(*) FROM sales_cart_lines l WHERE l.cart_id = c.id) AS line_count
         FROM sales_carts c
         WHERE c.till_id = :till AND c.status = 'held'
         ORDER BY c.updated_at, c.id",
    )?;
    let rows = statement.query_map(named_params! { ":till": till_id.as_str() }, |row| {
        Ok(HeldCartRow {
            cart_id: get_id(row, "id")?,
            held_at: get_timestamp(row, "updated_at")?,
            user_id: get_id(row, "user_id")?,
            customer_id: get_optional_id(row, "customer_id")?,
            note: row.get("note")?,
            line_count: row.get("line_count")?,
        })
    })?;
    let mut held = Vec::new();
    for row in rows {
        held.push(row?);
    }
    Ok(held)
}

/// How many carts with at least one line are still open on this shift.
/// Closing the shift is refused while this is more than zero. An empty cart
/// doesn't count: there's nothing in it to hold or cancel.
pub fn open_carts_on_shift(conn: &Connection, shift_id: &Id) -> Result<u32, SalesError> {
    let count: u32 = conn.query_row(
        "SELECT count(*) FROM sales_carts c
         WHERE c.shift_id = :shift AND c.status = 'open'
           AND EXISTS (SELECT 1 FROM sales_cart_lines l WHERE l.cart_id = c.id)",
        named_params! { ":shift": shift_id.as_str() },
        |row| row.get(0),
    )?;
    Ok(count)
}

fn read_cart_row(row: &Row) -> rusqlite::Result<Cart> {
    Ok(Cart {
        id: get_id(row, "id")?,
        till_id: get_id(row, "till_id")?,
        shift_id: get_id(row, "shift_id")?,
        user_id: get_id(row, "user_id")?,
        status: get_enum(row, "status", CartStatus::parse)?,
        customer_id: get_optional_id(row, "customer_id")?,
        price_list_id: get_optional_id(row, "price_list_id")?,
        note: row.get("note")?,
        invoice_discount: Discount::from_columns(
            row,
            "invoice_discount_kind",
            "invoice_discount_value",
        )?,
        exchange_return_id: get_optional_id(row, "exchange_return_id")?,
        created_at: get_timestamp(row, "created_at")?,
        updated_at: get_timestamp(row, "updated_at")?,
        lines: Vec::new(),
    })
}

fn cart_lines(conn: &Connection, cart_id: &Id) -> Result<Vec<CartLine>, SalesError> {
    let mut statement = conn.prepare(
        "SELECT id, seq, product_id, qty_milli, source, embedded_price_minor, price_override_minor,
                override_by, manual_discount_kind, manual_discount_value, added_at
         FROM sales_cart_lines WHERE cart_id = :cart ORDER BY seq",
    )?;
    let rows = statement.query_map(named_params! { ":cart": cart_id.as_str() }, read_line_row)?;
    let mut lines = Vec::new();
    for row in rows {
        lines.push(row?);
    }
    Ok(lines)
}

fn read_line_row(row: &Row) -> rusqlite::Result<CartLine> {
    let override_price = get_optional_money(row, "price_override_minor")?;
    let override_by = get_optional_id(row, "override_by")?;
    let price_override = match (override_price, override_by) {
        (Some(price), Some(by)) => Some(PriceOverride { price, by }),
        _ => None,
    };
    Ok(CartLine {
        id: get_id(row, "id")?,
        seq: row.get("seq")?,
        product_id: get_id(row, "product_id")?,
        qty: get_qty(row, "qty_milli")?,
        source: get_enum(row, "source", LineSource::parse)?,
        embedded_price: get_optional_money(row, "embedded_price_minor")?,
        price_override,
        manual_discount: Discount::from_columns(
            row,
            "manual_discount_kind",
            "manual_discount_value",
        )?,
        added_at: get_timestamp(row, "added_at")?,
    })
}
