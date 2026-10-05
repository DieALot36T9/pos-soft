//! Customer records: create, edit, archive and read (rule CUS-01).

use pos_core::{Id, Money, Timestamp};
use rusqlite::{Connection, OptionalExtension, Transaction, named_params};

use crate::rows::{CUSTOMER_COLUMNS, customer_from_row, load_balance};
use crate::{CustomerDetails, CustomerWithBalances, CustomersError, normalize_phone};

/// Saves a new customer and gives back its id. The phone is cleaned with the shop's country.
pub fn create(
    tx: &Transaction,
    details: CustomerDetails,
    default_country: &str,
    now: Timestamp,
) -> Result<Id, CustomersError> {
    let details = clean(details, default_country)?;
    check_phone_is_free(tx, &details.phone, None)?;
    let id = Id::new();
    tx.execute(
        "INSERT INTO customers_customers (id, phone, name, email, address, tier_id,
           credit_limit_minor, loyalty_opt_out, notes, archived_at, created_at, updated_at)
         VALUES (:id, :phone, :name, :email, :address, :tier_id,
           :credit_limit, :opt_out, :notes, NULL, :now, :now)",
        named_params! {
            ":id": id.as_str(),
            ":phone": details.phone,
            ":name": details.name,
            ":email": details.email,
            ":address": details.address,
            ":tier_id": details.tier_id.as_ref().map(Id::as_str),
            ":credit_limit": details.credit_limit.0,
            ":opt_out": details.loyalty_opt_out,
            ":notes": details.notes,
            ":now": now.0,
        },
    )?;
    tx.execute(
        "INSERT INTO customers_balances (customer_id) VALUES (:id)",
        named_params! { ":id": id.as_str() },
    )?;
    tx.execute(
        "INSERT INTO customers_search (customer_id, name) VALUES (:id, :name)",
        named_params! { ":id": id.as_str(), ":name": details.name },
    )?;
    Ok(id)
}

/// Replaces a customer's details with what the user typed.
pub fn update(
    tx: &Transaction,
    customer: &Id,
    details: CustomerDetails,
    default_country: &str,
    now: Timestamp,
) -> Result<(), CustomersError> {
    let details = clean(details, default_country)?;
    check_phone_is_free(tx, &details.phone, Some(customer))?;
    let changed = tx.execute(
        "UPDATE customers_customers SET phone = :phone, name = :name, email = :email,
           address = :address, tier_id = :tier_id, credit_limit_minor = :credit_limit,
           loyalty_opt_out = :opt_out, notes = :notes, updated_at = :now
         WHERE id = :id",
        named_params! {
            ":id": customer.as_str(),
            ":phone": details.phone,
            ":name": details.name,
            ":email": details.email,
            ":address": details.address,
            ":tier_id": details.tier_id.as_ref().map(Id::as_str),
            ":credit_limit": details.credit_limit.0,
            ":opt_out": details.loyalty_opt_out,
            ":notes": details.notes,
            ":now": now.0,
        },
    )?;
    if changed == 0 {
        return Err(CustomersError::CustomerNotFound(customer.to_string()));
    }
    tx.execute(
        "UPDATE customers_search SET name = :name WHERE customer_id = :id",
        named_params! { ":id": customer.as_str(), ":name": details.name },
    )?;
    Ok(())
}

/// Archives a customer (hides them at the till) or brings them back. Nothing is deleted,
/// because sales and ledgers still point to the customer.
pub fn set_archived(
    tx: &Transaction,
    customer: &Id,
    archived: bool,
    now: Timestamp,
) -> Result<(), CustomersError> {
    let archived_at = if archived { Some(now.0) } else { None };
    let changed = tx.execute(
        "UPDATE customers_customers SET archived_at = :archived_at, updated_at = :now
         WHERE id = :id",
        named_params! { ":id": customer.as_str(), ":archived_at": archived_at, ":now": now.0 },
    )?;
    if changed == 0 {
        return Err(CustomersError::CustomerNotFound(customer.to_string()));
    }
    Ok(())
}

/// Reads one customer with their points and credit balance.
pub fn get(connection: &Connection, customer: &Id) -> Result<CustomerWithBalances, CustomersError> {
    let sql = format!("SELECT {CUSTOMER_COLUMNS} FROM customers_customers WHERE id = :id");
    let row = connection
        .query_row(
            &sql,
            named_params! { ":id": customer.as_str() },
            customer_from_row,
        )
        .optional()?
        .ok_or_else(|| CustomersError::CustomerNotFound(customer.to_string()))?;
    let balance = load_balance(connection, customer)?;
    Ok(CustomerWithBalances {
        customer: row.into_customer()?,
        points: balance.points,
        credit: balance.credit,
        oldest_unpaid_at: balance.oldest_unpaid_at,
    })
}

/// Checks and tidies what the user typed: a real phone, a name, no negative limit.
fn clean(
    details: CustomerDetails,
    default_country: &str,
) -> Result<CustomerDetails, CustomersError> {
    let phone = normalize_phone(&details.phone, default_country)?;
    let name = details.name.trim().to_string();
    if name.is_empty() {
        return Err(CustomersError::NameMissing);
    }
    if details.credit_limit < Money::zero() {
        return Err(CustomersError::CreditLimitNegative);
    }
    Ok(CustomerDetails {
        phone,
        name,
        email: tidy(details.email),
        address: tidy(details.address),
        tier_id: details.tier_id,
        credit_limit: details.credit_limit,
        loyalty_opt_out: details.loyalty_opt_out,
        notes: tidy(details.notes),
    })
}

/// Trims optional text. Empty text is stored as nothing.
fn tidy(text: Option<String>) -> Option<String> {
    let trimmed = text?.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Refuses a phone number another customer already has (archived customers count too).
fn check_phone_is_free(
    connection: &Connection,
    phone: &str,
    except: Option<&Id>,
) -> Result<(), CustomersError> {
    let owner: Option<(String, String)> = connection
        .query_row(
            "SELECT id, name FROM customers_customers WHERE phone = :phone",
            named_params! { ":phone": phone },
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match owner {
        Some((id, _)) if Some(id.as_str()) == except.map(Id::as_str) => Ok(()),
        Some((_, name)) => Err(CustomersError::PhoneInUse { name }),
        None => Ok(()),
    }
}
