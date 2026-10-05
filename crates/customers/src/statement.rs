//! Customer credit statements for a date range (rule CUS-05).

use pos_core::{BusinessDate, Id, Money, Timestamp};
use rusqlite::{Connection, named_params};

use crate::rows::{load_balance, parse_id, parse_optional_id};
use crate::{CreditKind, CustomersError, Statement, StatementLine};

/// The credit rows between two business dates (both included), with the balance before
/// the first day, a running balance on each line, and the balance at the end.
pub fn statement(
    connection: &Connection,
    customer: &Id,
    from: BusinessDate,
    to: BusinessDate,
) -> Result<Statement, CustomersError> {
    if from > to {
        return Err(CustomersError::DateRangeInvalid);
    }
    load_balance(connection, customer)?; // refuses an unknown customer
    let opening: i64 = connection.query_row(
        "SELECT COALESCE(SUM(amount_minor), 0) FROM customers_credit_ledger
         WHERE customer_id = :customer_id AND business_date < :from",
        named_params! { ":customer_id": customer.as_str(), ":from": from.to_string() },
        |row| row.get(0),
    )?;
    let opening = Money(opening);
    let lines = lines_between(connection, customer, from, to, opening)?;
    let closing = lines.last().map_or(opening, |line| line.balance_after);
    Ok(Statement {
        customer_id: customer.clone(),
        from,
        to,
        opening,
        lines,
        closing,
    })
}

type RawLine = (
    String,
    i64,
    String,
    String,
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn lines_between(
    connection: &Connection,
    customer: &Id,
    from: BusinessDate,
    to: BusinessDate,
    opening: Money,
) -> Result<Vec<StatementLine>, CustomersError> {
    let mut statement = connection.prepare(
        "SELECT id, at, business_date, kind, amount_minor, method_kind, reference, ref_type, ref_id
         FROM customers_credit_ledger
         WHERE customer_id = :customer_id AND business_date BETWEEN :from AND :to
         ORDER BY business_date, at, rowid",
    )?;
    let rows = statement.query_map(
        named_params! {
            ":customer_id": customer.as_str(),
            ":from": from.to_string(),
            ":to": to.to_string(),
        },
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
            ))
        },
    )?;
    let mut balance = opening;
    let mut lines = Vec::new();
    for row in rows {
        let (id, at, day, kind, amount, method_kind, reference, ref_type, ref_id): RawLine = row?;
        let amount = Money(amount);
        balance = balance.checked_add(amount)?;
        lines.push(StatementLine {
            id: parse_id(&id)?,
            at: Timestamp(at),
            business_date: BusinessDate::parse(&day)?,
            kind: CreditKind::from_db(&kind)?,
            amount,
            method_kind,
            reference,
            ref_type,
            ref_id: parse_optional_id(ref_id)?,
            balance_after: balance,
        });
    }
    Ok(lines)
}
