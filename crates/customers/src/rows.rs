//! Reading this module's rows into Rust types, and writing the balances cache.

use pos_core::{AppError, Id, Money, Timestamp};
use rusqlite::{Connection, OptionalExtension, Row, named_params};

use crate::{Customer, CustomersError};

/// A row in the database that can't be read back. That's a bug, so it's an internal error.
pub(crate) fn corrupt(detail: String) -> CustomersError {
    CustomersError::App(AppError::internal(format!("bad customers row: {detail}")))
}

/// Reads an id stored as text.
pub(crate) fn parse_id(text: &str) -> Result<Id, CustomersError> {
    Id::parse(text).map_err(|_| corrupt(format!("id {text}")))
}

/// Reads an id that may be missing.
pub(crate) fn parse_optional_id(text: Option<String>) -> Result<Option<Id>, CustomersError> {
    match text {
        Some(text) => Ok(Some(parse_id(&text)?)),
        None => Ok(None),
    }
}

/// The columns of `customers_customers`, in the order `customer_from_row` reads them.
pub(crate) const CUSTOMER_COLUMNS: &str = "id, phone, name, email, address, tier_id, \
     credit_limit_minor, loyalty_opt_out, notes, archived_at, created_at, updated_at";

/// A customer row as SQLite gives it, before the ids are checked.
pub(crate) struct CustomerRow {
    id: String,
    phone: String,
    name: String,
    email: Option<String>,
    address: Option<String>,
    tier_id: Option<String>,
    credit_limit_minor: i64,
    loyalty_opt_out: bool,
    notes: Option<String>,
    archived_at: Option<i64>,
    created_at: i64,
    updated_at: i64,
}

/// Reads one row selected with `CUSTOMER_COLUMNS`.
pub(crate) fn customer_from_row(row: &Row) -> rusqlite::Result<CustomerRow> {
    Ok(CustomerRow {
        id: row.get(0)?,
        phone: row.get(1)?,
        name: row.get(2)?,
        email: row.get(3)?,
        address: row.get(4)?,
        tier_id: row.get(5)?,
        credit_limit_minor: row.get(6)?,
        loyalty_opt_out: row.get(7)?,
        notes: row.get(8)?,
        archived_at: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

impl CustomerRow {
    /// Checks the stored ids and gives back the customer.
    pub(crate) fn into_customer(self) -> Result<Customer, CustomersError> {
        Ok(Customer {
            id: parse_id(&self.id)?,
            phone: self.phone,
            name: self.name,
            email: self.email,
            address: self.address,
            tier_id: parse_optional_id(self.tier_id)?,
            credit_limit: Money(self.credit_limit_minor),
            loyalty_opt_out: self.loyalty_opt_out,
            notes: self.notes,
            archived_at: self.archived_at.map(Timestamp),
            created_at: Timestamp(self.created_at),
            updated_at: Timestamp(self.updated_at),
        })
    }
}

/// A customer's cached balances, plus the opt-out flag the points rules need.
pub(crate) struct Balance {
    pub points: i64,
    pub credit: Money,
    pub oldest_unpaid_at: Option<Timestamp>,
    pub loyalty_opt_out: bool,
}

/// Reads a customer's balances. Every customer has a balances row from the moment it's made.
pub(crate) fn load_balance(
    connection: &Connection,
    customer: &Id,
) -> Result<Balance, CustomersError> {
    connection
        .query_row(
            "SELECT b.points, b.credit_minor, b.oldest_unpaid_at, c.loyalty_opt_out
             FROM customers_balances b JOIN customers_customers c ON c.id = b.customer_id
             WHERE b.customer_id = :customer_id",
            named_params! { ":customer_id": customer.as_str() },
            |row| {
                Ok(Balance {
                    points: row.get(0)?,
                    credit: Money(row.get(1)?),
                    oldest_unpaid_at: row.get::<_, Option<i64>>(2)?.map(Timestamp),
                    loyalty_opt_out: row.get(3)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| CustomersError::CustomerNotFound(customer.to_string()))
}

/// Saves a new points balance in the cache.
pub(crate) fn save_points(
    connection: &Connection,
    customer: &Id,
    points: i64,
) -> Result<(), CustomersError> {
    connection.execute(
        "UPDATE customers_balances SET points = :points WHERE customer_id = :customer_id",
        named_params! { ":points": points, ":customer_id": customer.as_str() },
    )?;
    Ok(())
}

/// Saves a new credit balance and oldest unpaid time in the cache.
pub(crate) fn save_credit(
    connection: &Connection,
    customer: &Id,
    credit: Money,
    oldest_unpaid_at: Option<Timestamp>,
) -> Result<(), CustomersError> {
    connection.execute(
        "UPDATE customers_balances SET credit_minor = :credit, oldest_unpaid_at = :oldest
         WHERE customer_id = :customer_id",
        named_params! {
            ":credit": credit.0,
            ":oldest": oldest_unpaid_at.map(|at| at.0),
            ":customer_id": customer.as_str(),
        },
    )?;
    Ok(())
}
