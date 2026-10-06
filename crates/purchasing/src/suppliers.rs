//! PUR-01 Suppliers: add, change, archive and list.

use pos_core::{Id, Timestamp};
use rusqlite::{Connection, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::PurchasingError;
use crate::sql::{clean_text, read_id, read_optional_timestamp};

/// What the user types for a supplier. Only the name is required.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SupplierInput {
    pub name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub tax_number: Option<String>,
    /// Days the shop has to pay each invoice. 0 means pay on delivery.
    pub payment_terms_days: u32,
    pub notes: Option<String>,
}

/// A saved supplier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Supplier {
    pub id: Id,
    pub name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub tax_number: Option<String>,
    pub payment_terms_days: u32,
    pub notes: Option<String>,
    /// Set when the supplier was archived. Archived suppliers get no new orders.
    pub archived_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// Adds a new supplier and returns its id.
pub fn create_supplier(
    tx: &Transaction,
    input: &SupplierInput,
    now: Timestamp,
) -> Result<Id, PurchasingError> {
    let name = required_name(input)?;
    let id = Id::new();
    tx.execute(
        "INSERT INTO purchasing_suppliers (id, name, contact_person, phone, email, address,
           tax_number, payment_terms_days, notes, created_at, updated_at)
         VALUES (:id, :name, :contact_person, :phone, :email, :address,
           :tax_number, :payment_terms_days, :notes, :now, :now)",
        named_params! {
            ":id": id.as_str(),
            ":name": name,
            ":contact_person": clean_text(input.contact_person.as_deref()),
            ":phone": clean_text(input.phone.as_deref()),
            ":email": clean_text(input.email.as_deref()),
            ":address": clean_text(input.address.as_deref()),
            ":tax_number": clean_text(input.tax_number.as_deref()),
            ":payment_terms_days": input.payment_terms_days,
            ":notes": clean_text(input.notes.as_deref()),
            ":now": now.0,
        },
    )?;
    Ok(id)
}

/// Saves new details for an existing supplier.
pub fn update_supplier(
    tx: &Transaction,
    supplier: &Id,
    input: &SupplierInput,
    now: Timestamp,
) -> Result<(), PurchasingError> {
    let name = required_name(input)?;
    let changed = tx.execute(
        "UPDATE purchasing_suppliers SET name = :name, contact_person = :contact_person,
           phone = :phone, email = :email, address = :address, tax_number = :tax_number,
           payment_terms_days = :payment_terms_days, notes = :notes, updated_at = :now
         WHERE id = :id",
        named_params! {
            ":id": supplier.as_str(),
            ":name": name,
            ":contact_person": clean_text(input.contact_person.as_deref()),
            ":phone": clean_text(input.phone.as_deref()),
            ":email": clean_text(input.email.as_deref()),
            ":address": clean_text(input.address.as_deref()),
            ":tax_number": clean_text(input.tax_number.as_deref()),
            ":payment_terms_days": input.payment_terms_days,
            ":notes": clean_text(input.notes.as_deref()),
            ":now": now.0,
        },
    )?;
    if changed == 0 {
        return Err(PurchasingError::not_found("supplier", supplier));
    }
    Ok(())
}

/// Archives a supplier instead of deleting it, because old documents still point to it.
/// Archiving twice keeps the first archive time.
pub fn archive_supplier(
    tx: &Transaction,
    supplier: &Id,
    now: Timestamp,
) -> Result<(), PurchasingError> {
    let changed = tx.execute(
        "UPDATE purchasing_suppliers SET archived_at = COALESCE(archived_at, :now), updated_at = :now
         WHERE id = :id",
        named_params! { ":id": supplier.as_str(), ":now": now.0 },
    )?;
    if changed == 0 {
        return Err(PurchasingError::not_found("supplier", supplier));
    }
    Ok(())
}

/// Lists suppliers by name. Archived ones are included only when asked.
pub fn list_suppliers(
    connection: &Connection,
    include_archived: bool,
) -> Result<Vec<Supplier>, PurchasingError> {
    let mut statement = connection.prepare(
        "SELECT * FROM purchasing_suppliers
         WHERE :include_archived OR archived_at IS NULL
         ORDER BY name COLLATE NOCASE, id",
    )?;
    let rows = statement.query_map(
        named_params! { ":include_archived": include_archived },
        supplier_from_row,
    )?;
    let mut suppliers = Vec::new();
    for supplier in rows {
        suppliers.push(supplier?);
    }
    Ok(suppliers)
}

/// One supplier by id.
pub fn get_supplier(connection: &Connection, supplier: &Id) -> Result<Supplier, PurchasingError> {
    let mut statement = connection.prepare("SELECT * FROM purchasing_suppliers WHERE id = :id")?;
    let mut rows = statement.query_map(
        named_params! { ":id": supplier.as_str() },
        supplier_from_row,
    )?;
    match rows.next() {
        Some(found) => Ok(found?),
        None => Err(PurchasingError::not_found("supplier", supplier)),
    }
}

fn required_name(input: &SupplierInput) -> Result<String, PurchasingError> {
    clean_text(Some(&input.name)).ok_or(PurchasingError::Invalid {
        field: "name",
        code: "supplier_name_missing",
    })
}

fn supplier_from_row(row: &Row) -> rusqlite::Result<Supplier> {
    Ok(Supplier {
        id: read_id(row, "id")?,
        name: row.get("name")?,
        contact_person: row.get("contact_person")?,
        phone: row.get("phone")?,
        email: row.get("email")?,
        address: row.get("address")?,
        tax_number: row.get("tax_number")?,
        payment_terms_days: row.get("payment_terms_days")?,
        notes: row.get("notes")?,
        archived_at: read_optional_timestamp(row, "archived_at")?,
        created_at: Timestamp(row.get("created_at")?),
        updated_at: Timestamp(row.get("updated_at")?),
    })
}
