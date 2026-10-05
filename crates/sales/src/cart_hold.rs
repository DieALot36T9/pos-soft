//! Holding, resuming and cancelling carts (CHK-05, CHK-09, CHK-10).

use pos_core::{Id, Timestamp};
use rusqlite::{Transaction, named_params};

use crate::SalesError;
use crate::cart::{Cart, CartStatus, get_cart};
use crate::cart_edit::{line_count, open_cart_id, require_open};

/// Puts the cart aside with an optional note, so the next customer can be served (CHK-09).
/// The workflow then opens a new empty cart. An empty cart can't be held (`cart_empty`).
pub fn hold(
    tx: &Transaction,
    cart_id: &Id,
    note: Option<String>,
    now: Timestamp,
) -> Result<(), SalesError> {
    require_open(tx, cart_id)?;
    if line_count(tx, cart_id)? == 0 {
        return Err(SalesError::invalid("cart_empty"));
    }
    let note = note
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    tx.execute(
        "UPDATE sales_carts SET status = 'held', note = :note, updated_at = :now WHERE id = :id",
        named_params! { ":note": note, ":now": now.0, ":id": cart_id.as_str() },
    )?;
    Ok(())
}

/// Brings a held cart back for this cashier and shift (CHK-10). Any cashier with an
/// open shift on the same till may resume it. If the cashier's current cart is empty
/// it is thrown away; if it has lines, resuming is refused (`cart_in_progress`).
pub fn resume(
    tx: &Transaction,
    cart_id: &Id,
    user_id: &Id,
    shift_id: &Id,
    now: Timestamp,
) -> Result<(), SalesError> {
    let cart = get_cart(tx, cart_id)?;
    if cart.status != CartStatus::Held {
        return Err(SalesError::conflict("cart_not_held"));
    }
    if let Some(current_id) = open_cart_id(tx, &cart.till_id, user_id)? {
        if line_count(tx, &current_id)? > 0 {
            return Err(SalesError::conflict("cart_in_progress"));
        }
        delete_cart(tx, &current_id)?;
    }
    tx.execute(
        "UPDATE sales_carts SET status = 'open', user_id = :user, shift_id = :shift, updated_at = :now
         WHERE id = :id",
        named_params! {
            ":user": user_id.as_str(), ":shift": shift_id.as_str(), ":now": now.0, ":id": cart_id.as_str(),
        },
    )?;
    Ok(())
}

/// Throws a cart away, open or held (CHK-05). No invoice number is used.
/// Returns the cart as it was, so the workflow can write it to the audit log.
pub fn cancel(tx: &Transaction, cart_id: &Id) -> Result<Cart, SalesError> {
    let cart = get_cart(tx, cart_id)?;
    delete_cart(tx, cart_id)?;
    Ok(cart)
}

/// Deletes a cart and its lines. Used by cancel, resume and complete.
pub(crate) fn delete_cart(tx: &Transaction, cart_id: &Id) -> Result<(), SalesError> {
    // Lines are deleted first so this works even if foreign keys are switched off.
    tx.execute(
        "DELETE FROM sales_cart_lines WHERE cart_id = :id",
        named_params! { ":id": cart_id.as_str() },
    )?;
    tx.execute(
        "DELETE FROM sales_carts WHERE id = :id",
        named_params! { ":id": cart_id.as_str() },
    )?;
    Ok(())
}
