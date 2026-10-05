//! Payment methods: cash, card, mobile wallets and the system methods (SET-15).

use pos_core::Id;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};

use crate::SalesError;
use crate::sql::{get_bool, get_enum, get_id};

/// What kind of payment a method is. The kind decides the rules, not the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaymentKind {
    /// Notes and coins. The only kind that may be more than the amount due (change).
    Cash,
    Card,
    /// Mobile wallets. Example: bKash.
    Wallet,
    Other,
    /// Customer credit (baki). System method.
    Credit,
    /// Loyalty points. System method.
    Points,
    /// The refund from a return, spent on the new sale in an exchange. System method.
    ExchangeCredit,
}

impl PaymentKind {
    /// The text stored in the `kind` column.
    pub fn as_str(self) -> &'static str {
        match self {
            PaymentKind::Cash => "cash",
            PaymentKind::Card => "card",
            PaymentKind::Wallet => "wallet",
            PaymentKind::Other => "other",
            PaymentKind::Credit => "credit",
            PaymentKind::Points => "points",
            PaymentKind::ExchangeCredit => "exchange_credit",
        }
    }

    /// Reads a kind from the text stored in the `kind` column.
    pub fn parse(text: &str) -> Option<PaymentKind> {
        match text {
            "cash" => Some(PaymentKind::Cash),
            "card" => Some(PaymentKind::Card),
            "wallet" => Some(PaymentKind::Wallet),
            "other" => Some(PaymentKind::Other),
            "credit" => Some(PaymentKind::Credit),
            "points" => Some(PaymentKind::Points),
            "exchange_credit" => Some(PaymentKind::ExchangeCredit),
            _ => None,
        }
    }

    /// True for the kinds only the system methods have. The shop can't add more of these.
    pub fn is_system_only(self) -> bool {
        matches!(
            self,
            PaymentKind::Credit | PaymentKind::Points | PaymentKind::ExchangeCredit
        )
    }
}

/// One way to pay, as listed in Settings → Payment methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentMethod {
    pub id: Id,
    pub name: String,
    pub kind: PaymentKind,
    /// The cashier must type a reference, such as a transaction id (PAY-02). Never a card number.
    pub requires_reference: bool,
    pub active: bool,
    /// Position in the list, lowest first.
    pub sort: i64,
    /// Made by the app (cash, customer credit, points, exchange credit). Can't change kind.
    pub is_system: bool,
}

/// What the admin sends to add a method (`id: None`) or change one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentMethodInput {
    pub id: Option<Id>,
    pub name: String,
    pub kind: PaymentKind,
    pub requires_reference: bool,
    pub active: bool,
    pub sort: i64,
}

const COLUMNS: &str = "id, name, kind, requires_reference, active, sort, is_system";

/// Every payment method, active or not, in list order.
pub fn payment_methods(conn: &Connection) -> Result<Vec<PaymentMethod>, SalesError> {
    let sql = format!("SELECT {COLUMNS} FROM sales_payment_methods ORDER BY sort, name, id");
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map([], read_method)?;
    let mut methods = Vec::new();
    for row in rows {
        methods.push(row?);
    }
    Ok(methods)
}

/// One payment method by id.
pub fn payment_method(conn: &Connection, method_id: &Id) -> Result<PaymentMethod, SalesError> {
    let sql = format!("SELECT {COLUMNS} FROM sales_payment_methods WHERE id = :id");
    conn.query_row(
        &sql,
        named_params! { ":id": method_id.as_str() },
        read_method,
    )
    .optional()?
    .ok_or_else(|| SalesError::not_found("payment_method", method_id.as_str()))
}

/// The system method of a kind, for example the cash method or the exchange credit method.
pub fn system_payment_method(
    conn: &Connection,
    kind: PaymentKind,
) -> Result<PaymentMethod, SalesError> {
    let sql =
        format!("SELECT {COLUMNS} FROM sales_payment_methods WHERE kind = :kind AND is_system = 1");
    conn.query_row(&sql, named_params! { ":kind": kind.as_str() }, read_method)
        .optional()?
        .ok_or_else(|| SalesError::not_found("payment_method", kind.as_str()))
}

/// Adds or changes a payment method and returns its id (SET-15).
/// Methods are never deleted, because old sales refer to them; switch them off instead.
pub fn save_payment_method(tx: &Transaction, input: PaymentMethodInput) -> Result<Id, SalesError> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(SalesError::invalid("payment_method_name_missing"));
    }
    match &input.id {
        None => insert_method(tx, &input, &name),
        Some(method_id) => update_method(tx, method_id, &input, &name),
    }
}

fn insert_method(
    tx: &Transaction,
    input: &PaymentMethodInput,
    name: &str,
) -> Result<Id, SalesError> {
    if input.kind.is_system_only() {
        return Err(SalesError::invalid("payment_kind_reserved"));
    }
    let method_id = Id::new();
    tx.execute(
        "INSERT INTO sales_payment_methods (id, name, kind, requires_reference, active, sort, is_system)
         VALUES (:id, :name, :kind, :reference, :active, :sort, 0)",
        named_params! {
            ":id": method_id.as_str(), ":name": name, ":kind": input.kind.as_str(),
            ":reference": input.requires_reference, ":active": input.active, ":sort": input.sort,
        },
    )?;
    Ok(method_id)
}

fn update_method(
    tx: &Transaction,
    method_id: &Id,
    input: &PaymentMethodInput,
    name: &str,
) -> Result<Id, SalesError> {
    let existing = payment_method(tx, method_id)?;
    if existing.is_system && input.kind != existing.kind {
        return Err(SalesError::invalid("payment_kind_fixed"));
    }
    if !existing.is_system && input.kind.is_system_only() {
        return Err(SalesError::invalid("payment_kind_reserved"));
    }
    // Change is always given in cash, so the system cash method stays on.
    if existing.is_system && existing.kind == PaymentKind::Cash && !input.active {
        return Err(SalesError::invalid("payment_cash_required"));
    }
    tx.execute(
        "UPDATE sales_payment_methods
         SET name = :name, kind = :kind, requires_reference = :reference, active = :active, sort = :sort
         WHERE id = :id",
        named_params! {
            ":name": name, ":kind": input.kind.as_str(), ":reference": input.requires_reference,
            ":active": input.active, ":sort": input.sort, ":id": method_id.as_str(),
        },
    )?;
    Ok(method_id.clone())
}

fn read_method(row: &Row) -> rusqlite::Result<PaymentMethod> {
    Ok(PaymentMethod {
        id: get_id(row, "id")?,
        name: row.get("name")?,
        kind: get_enum(row, "kind", PaymentKind::parse)?,
        requires_reference: get_bool(row, "requires_reference")?,
        active: get_bool(row, "active")?,
        sort: row.get("sort")?,
        is_system: get_bool(row, "is_system")?,
    })
}
