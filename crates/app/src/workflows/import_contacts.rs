//! Customer and supplier rows of an import (IO-02). A customer is matched by phone
//! number, a supplier by name; the rest are made new.

use std::collections::HashMap;

use customers::CustomerDetails;
use pos_core::{AppError, CurrencyFormat, Id};
use purchasing::SupplierInput;
use rusqlite::{OptionalExtension, Transaction};
use settings::CreditSettings;

use crate::workflows::customer_records::shop_country;
use crate::workflows::import_file::{Field, RowAction, RowCheck, RowCtx, name_key, sql_error};

/// The customer fields. The export adds "Points" and "Balance" after them; an
/// import leaves those columns alone.
pub(crate) const CUSTOMER_FIELDS: &[Field] = &[
    field(
        "phone",
        "Phone",
        &["Mobile", "Phone number", "Mobile number"],
        true,
    ),
    field("name", "Name", &["Customer name", "Customer"], true),
    field("email", "Email", &["E-mail"], false),
    field("address", "Address", &[], false),
    field("credit_limit", "Credit limit", &["Credit"], false),
    field(
        "loyalty_opt_out",
        "No loyalty points",
        &["Loyalty opt out"],
        false,
    ),
    field("notes", "Notes", &["Note"], false),
];

/// The supplier fields. The export adds "Balance" after them.
pub(crate) const SUPPLIER_FIELDS: &[Field] = &[
    field(
        "name",
        "Name",
        &["Supplier name", "Supplier", "Company"],
        true,
    ),
    field("contact_person", "Contact person", &["Contact"], false),
    field("phone", "Phone", &["Mobile", "Phone number"], false),
    field("email", "Email", &["E-mail"], false),
    field("address", "Address", &[], false),
    field(
        "tax_number",
        "Tax number",
        &["BIN", "VAT number", "TIN"],
        false,
    ),
    field(
        "payment_terms_days",
        "Payment terms (days)",
        &["Payment terms", "Terms"],
        false,
    ),
    field("notes", "Notes", &["Note"], false),
];

const fn field(
    key: &'static str,
    header: &'static str,
    other_names: &'static [&'static str],
    required: bool,
) -> Field {
    Field {
        key,
        header,
        other_names,
        required,
    }
}

/// What every customer row needs to know.
pub(crate) struct CustomerLookups {
    country: String,
    money_digits: u8,
    default_limit: pos_core::Money,
}

impl CustomerLookups {
    pub(crate) fn load(tx: &Transaction) -> Result<CustomerLookups, AppError> {
        let currency: CurrencyFormat = settings::get(tx)?;
        let credit: CreditSettings = settings::get(tx)?;
        Ok(CustomerLookups {
            country: shop_country(tx)?,
            money_digits: currency.minor_digits,
            default_limit: credit.default_limit_minor,
        })
    }
}

/// Saves one customer row: changes the customer with that phone number, or adds one.
pub(crate) fn save_customer(
    tx: &Transaction,
    ctx: &RowCtx,
    lookups: &mut CustomerLookups,
    check: &mut RowCheck,
) -> Result<RowAction, AppError> {
    let phone_text = check.required("phone");
    let name = check.required("name");
    let email = check.text("email");
    let address = check.text("address");
    let credit_limit = check.money("credit_limit", lookups.money_digits);
    let opt_out = check.flag("loyalty_opt_out");
    let notes = check.text("notes");
    let phone = match customers::normalize_phone(&phone_text, &lookups.country) {
        Ok(phone) => {
            check.show("phone", phone.clone());
            Some(phone)
        }
        Err(_) if phone_text.is_empty() => None,
        Err(_) => {
            check.fail("phone", "phone_invalid");
            None
        }
    };
    check.ready()?;
    let phone = phone.unwrap_or_default();

    let existing: Option<String> = tx
        .query_row(
            "SELECT id FROM customers_v_customers WHERE phone = ?1",
            [&phone],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_error)?;
    match existing {
        Some(id) => {
            let id = Id::parse(&id)?;
            let saved = customers::get(tx, &id)?.customer;
            let details = CustomerDetails {
                phone: saved.phone.clone(),
                name,
                email: email.or(saved.email.clone()),
                address: address.or(saved.address.clone()),
                tier_id: saved.tier_id.clone(),
                credit_limit: credit_limit.unwrap_or(saved.credit_limit),
                loyalty_opt_out: opt_out.unwrap_or(saved.loyalty_opt_out),
                notes: notes.or(saved.notes.clone()),
            };
            let unchanged = details.name == saved.name
                && details.email == saved.email
                && details.address == saved.address
                && details.credit_limit == saved.credit_limit
                && details.loyalty_opt_out == saved.loyalty_opt_out
                && details.notes == saved.notes;
            if unchanged {
                return Ok(RowAction::Skip);
            }
            customers::update(tx, &id, details, &lookups.country, ctx.now)?;
            Ok(RowAction::Update)
        }
        None => {
            let details = CustomerDetails {
                email,
                address,
                credit_limit: credit_limit.unwrap_or(lookups.default_limit),
                loyalty_opt_out: opt_out.unwrap_or(false),
                notes,
                ..CustomerDetails::quick(&phone, &name)
            };
            customers::create(tx, details, &lookups.country, ctx.now)?;
            Ok(RowAction::Create)
        }
    }
}

/// Every supplier by name, archived ones too.
pub(crate) struct SupplierLookups {
    by_name: HashMap<String, Id>,
}

impl SupplierLookups {
    pub(crate) fn load(tx: &Transaction) -> Result<SupplierLookups, AppError> {
        Ok(SupplierLookups {
            by_name: purchasing::list_suppliers(tx, true)?
                .into_iter()
                .map(|supplier| (name_key(&supplier.name), supplier.id))
                .collect(),
        })
    }
}

/// Saves one supplier row: changes the supplier with that name, or adds one.
pub(crate) fn save_supplier(
    tx: &Transaction,
    ctx: &RowCtx,
    lookups: &mut SupplierLookups,
    check: &mut RowCheck,
) -> Result<RowAction, AppError> {
    let name = check.required("name");
    let contact_person = check.text("contact_person");
    let phone = check.text("phone");
    let email = check.text("email");
    let address = check.text("address");
    let tax_number = check.text("tax_number");
    let terms = check.whole("payment_terms_days");
    let notes = check.text("notes");
    check.ready()?;

    match lookups.by_name.get(&name_key(&name)).cloned() {
        Some(id) => {
            let saved = purchasing::get_supplier(tx, &id)?;
            let input = SupplierInput {
                name: saved.name.clone(),
                contact_person: contact_person.or(saved.contact_person.clone()),
                phone: phone.or(saved.phone.clone()),
                email: email.or(saved.email.clone()),
                address: address.or(saved.address.clone()),
                tax_number: tax_number.or(saved.tax_number.clone()),
                payment_terms_days: terms.unwrap_or(saved.payment_terms_days),
                notes: notes.or(saved.notes.clone()),
            };
            let unchanged = input.contact_person == saved.contact_person
                && input.phone == saved.phone
                && input.email == saved.email
                && input.address == saved.address
                && input.tax_number == saved.tax_number
                && input.payment_terms_days == saved.payment_terms_days
                && input.notes == saved.notes;
            if unchanged {
                return Ok(RowAction::Skip);
            }
            purchasing::update_supplier(tx, &id, &input, ctx.now)?;
            Ok(RowAction::Update)
        }
        None => {
            let input = SupplierInput {
                name: name.clone(),
                contact_person,
                phone,
                email,
                address,
                tax_number,
                payment_terms_days: terms.unwrap_or(0),
                notes,
            };
            let id = purchasing::create_supplier(tx, &input, ctx.now)?;
            lookups.by_name.insert(name_key(&name), id);
            Ok(RowAction::Create)
        }
    }
}
