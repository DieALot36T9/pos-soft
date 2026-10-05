//! Changing carts. Every change is saved straight away, so a crash loses nothing (CHK-19).
//! Only open carts can be changed; a held cart must be resumed first (see `cart_hold`).

use pos_core::{Id, Money, Qty, Timestamp};
use rusqlite::{OptionalExtension, Transaction, named_params};

use crate::cart::{CartStatus, Discount, NewCartLine, PriceOverride};
use crate::{SalesError, sql};

/// Starts a new, empty cart for this cashier on this till and shift.
/// Refused if the cashier already has an open cart here (`cart_already_open`).
pub fn open_cart(
    tx: &Transaction,
    till_id: &Id,
    shift_id: &Id,
    user_id: &Id,
    now: Timestamp,
) -> Result<Id, SalesError> {
    if open_cart_id(tx, till_id, user_id)?.is_some() {
        return Err(SalesError::conflict("cart_already_open"));
    }
    let cart_id = Id::new();
    tx.execute(
        "INSERT INTO sales_carts (id, till_id, shift_id, user_id, status, created_at, updated_at)
         VALUES (:id, :till, :shift, :user, 'open', :now, :now)",
        named_params! {
            ":id": cart_id.as_str(), ":till": till_id.as_str(), ":shift": shift_id.as_str(),
            ":user": user_id.as_str(), ":now": now.0,
        },
    )?;
    Ok(cart_id)
}

/// Adds a product to the cart and returns the line it went into.
/// With `merge` on, a repeat scan adds to the newest plain line for the same product
/// (CHK-01). A plain line has no scale price, price override or discount. The workflow
/// passes `merge = false` for weighed items and when the shop turned merging off.
pub fn add_line(
    tx: &Transaction,
    cart_id: &Id,
    line: NewCartLine,
    merge: bool,
    now: Timestamp,
) -> Result<Id, SalesError> {
    require_open(tx, cart_id)?;
    check_qty(line.qty)?;
    if let Some(price) = line.embedded_price {
        check_price(price)?;
    }
    let merge_into = if merge && line.embedded_price.is_none() {
        plain_line_for(tx, cart_id, &line.product_id)?
    } else {
        None
    };
    if let Some((line_id, qty)) = merge_into {
        let new_qty = qty
            .0
            .checked_add(line.qty.0)
            .ok_or_else(|| SalesError::invalid("qty_too_large"))?;
        update_qty(tx, &line_id, Qty(new_qty))?;
        touch(tx, cart_id, now)?;
        return Ok(line_id);
    }
    let line_id = Id::new();
    tx.execute(
        "INSERT INTO sales_cart_lines (id, cart_id, seq, product_id, qty_milli, source, embedded_price_minor, added_at)
         VALUES (:id, :cart,
                 (SELECT coalesce(max(seq), 0) + 1 FROM sales_cart_lines WHERE cart_id = :cart),
                 :product, :qty, :source, :embedded, :now)",
        named_params! {
            ":id": line_id.as_str(), ":cart": cart_id.as_str(), ":product": line.product_id.as_str(),
            ":qty": line.qty.0, ":source": line.source.as_str(),
            ":embedded": line.embedded_price.map(|price| price.0), ":now": now.0,
        },
    )?;
    touch(tx, cart_id, now)?;
    Ok(line_id)
}

/// Changes a line's quantity. Quantity 0 removes the line (CHK-04).
pub fn set_qty(tx: &Transaction, line_id: &Id, qty: Qty, now: Timestamp) -> Result<(), SalesError> {
    if qty.0 == 0 {
        return remove_line(tx, line_id, now);
    }
    check_qty(qty)?;
    let cart_id = open_cart_of_line(tx, line_id)?;
    update_qty(tx, line_id, qty)?;
    touch(tx, &cart_id, now)
}

/// Takes a line out of the cart (CHK-05).
pub fn remove_line(tx: &Transaction, line_id: &Id, now: Timestamp) -> Result<(), SalesError> {
    let cart_id = open_cart_of_line(tx, line_id)?;
    tx.execute(
        "DELETE FROM sales_cart_lines WHERE id = :id",
        named_params! { ":id": line_id.as_str() },
    )?;
    touch(tx, &cart_id, now)
}

/// Sets or clears a line's manual discount (CHK-12). The workflow checks the role's limit.
pub fn set_line_discount(
    tx: &Transaction,
    line_id: &Id,
    discount: Option<Discount>,
    now: Timestamp,
) -> Result<(), SalesError> {
    check_discount(discount)?;
    let cart_id = open_cart_of_line(tx, line_id)?;
    let (kind, value) = Discount::to_columns(discount);
    tx.execute(
        "UPDATE sales_cart_lines SET manual_discount_kind = :kind, manual_discount_value = :value WHERE id = :id",
        named_params! { ":kind": kind, ":value": value, ":id": line_id.as_str() },
    )?;
    touch(tx, &cart_id, now)
}

/// Sets or clears a line's typed-in unit price (CHK-13). The workflow checks `PriceOverride`.
pub fn set_price_override(
    tx: &Transaction,
    line_id: &Id,
    price_override: Option<PriceOverride>,
    now: Timestamp,
) -> Result<(), SalesError> {
    if let Some(price_override) = &price_override {
        check_price(price_override.price)?;
    }
    let cart_id = open_cart_of_line(tx, line_id)?;
    let price = price_override.as_ref().map(|value| value.price.0);
    let by = price_override.as_ref().map(|value| value.by.as_str());
    tx.execute(
        "UPDATE sales_cart_lines SET price_override_minor = :price, override_by = :by WHERE id = :id",
        named_params! { ":price": price, ":by": by, ":id": line_id.as_str() },
    )?;
    touch(tx, &cart_id, now)
}

/// Sets or clears the discount on the whole cart (CHK-12, `Shift+F7`).
pub fn set_invoice_discount(
    tx: &Transaction,
    cart_id: &Id,
    discount: Option<Discount>,
    now: Timestamp,
) -> Result<(), SalesError> {
    check_discount(discount)?;
    require_open(tx, cart_id)?;
    let (kind, value) = Discount::to_columns(discount);
    tx.execute(
        "UPDATE sales_carts SET invoice_discount_kind = :kind, invoice_discount_value = :value WHERE id = :id",
        named_params! { ":kind": kind, ":value": value, ":id": cart_id.as_str() },
    )?;
    touch(tx, cart_id, now)
}

/// Attaches a customer to the cart, or takes them off (CHK-11).
pub fn set_customer(
    tx: &Transaction,
    cart_id: &Id,
    customer_id: Option<&Id>,
    now: Timestamp,
) -> Result<(), SalesError> {
    set_cart_column(tx, cart_id, "customer_id", customer_id, now)
}

/// Switches the cart to another price list, or back to the normal prices (CHK-17).
pub fn set_price_list(
    tx: &Transaction,
    cart_id: &Id,
    price_list_id: Option<&Id>,
    now: Timestamp,
) -> Result<(), SalesError> {
    set_cart_column(tx, cart_id, "price_list_id", price_list_id, now)
}

/// Marks the cart as the new half of an exchange, linked to its return (RET-05).
pub fn set_exchange_return(
    tx: &Transaction,
    cart_id: &Id,
    return_id: Option<&Id>,
    now: Timestamp,
) -> Result<(), SalesError> {
    if let Some(return_id) = return_id {
        let found: Option<i64> = tx
            .query_row(
                "SELECT 1 FROM sales_returns WHERE id = :id",
                named_params! { ":id": return_id.as_str() },
                |row| row.get(0),
            )
            .optional()?;
        if found.is_none() {
            return Err(SalesError::not_found("return", return_id.as_str()));
        }
    }
    set_cart_column(tx, cart_id, "exchange_return_id", return_id, now)
}

fn set_cart_column(
    tx: &Transaction,
    cart_id: &Id,
    column: &str,
    value: Option<&Id>,
    now: Timestamp,
) -> Result<(), SalesError> {
    require_open(tx, cart_id)?;
    // `column` is always one of our own fixed names, never text from the user.
    let sql = format!("UPDATE sales_carts SET {column} = :value WHERE id = :id");
    tx.execute(
        &sql,
        named_params! { ":value": value.map(Id::as_str), ":id": cart_id.as_str() },
    )?;
    touch(tx, cart_id, now)
}

/// Refuses unless the cart exists and is open.
pub(crate) fn require_open(tx: &Transaction, cart_id: &Id) -> Result<(), SalesError> {
    let status = tx
        .query_row(
            "SELECT status FROM sales_carts WHERE id = :id",
            named_params! { ":id": cart_id.as_str() },
            |row| sql::get_enum(row, "status", CartStatus::parse),
        )
        .optional()?;
    match status {
        None => Err(SalesError::not_found("cart", cart_id.as_str())),
        Some(CartStatus::Held) => Err(SalesError::conflict("cart_not_open")),
        Some(CartStatus::Open) => Ok(()),
    }
}

/// The cart a line belongs to, refusing unless that cart is open.
fn open_cart_of_line(tx: &Transaction, line_id: &Id) -> Result<Id, SalesError> {
    let cart_id = tx
        .query_row(
            "SELECT cart_id FROM sales_cart_lines WHERE id = :id",
            named_params! { ":id": line_id.as_str() },
            |row| sql::get_id(row, "cart_id"),
        )
        .optional()?;
    let Some(cart_id) = cart_id else {
        return Err(SalesError::not_found("cart_line", line_id.as_str()));
    };
    require_open(tx, &cart_id)?;
    Ok(cart_id)
}

pub(crate) fn open_cart_id(
    tx: &Transaction,
    till_id: &Id,
    user_id: &Id,
) -> Result<Option<Id>, SalesError> {
    let cart_id = tx
        .query_row(
            "SELECT id FROM sales_carts WHERE till_id = :till AND user_id = :user AND status = 'open'",
            named_params! { ":till": till_id.as_str(), ":user": user_id.as_str() },
            |row| sql::get_id(row, "id"),
        )
        .optional()?;
    Ok(cart_id)
}

/// The newest line for this product that has no scale price, override or discount.
fn plain_line_for(
    tx: &Transaction,
    cart_id: &Id,
    product_id: &Id,
) -> Result<Option<(Id, Qty)>, SalesError> {
    let found = tx
        .query_row(
            "SELECT id, qty_milli FROM sales_cart_lines
             WHERE cart_id = :cart AND product_id = :product AND embedded_price_minor IS NULL
               AND price_override_minor IS NULL AND manual_discount_kind IS NULL
             ORDER BY seq DESC LIMIT 1",
            named_params! { ":cart": cart_id.as_str(), ":product": product_id.as_str() },
            |row| Ok((sql::get_id(row, "id")?, sql::get_qty(row, "qty_milli")?)),
        )
        .optional()?;
    Ok(found)
}

pub(crate) fn line_count(tx: &Transaction, cart_id: &Id) -> Result<u32, SalesError> {
    let count: u32 = tx.query_row(
        "SELECT count(*) FROM sales_cart_lines WHERE cart_id = :cart",
        named_params! { ":cart": cart_id.as_str() },
        |row| row.get(0),
    )?;
    Ok(count)
}

fn update_qty(tx: &Transaction, line_id: &Id, qty: Qty) -> Result<(), SalesError> {
    tx.execute(
        "UPDATE sales_cart_lines SET qty_milli = :qty WHERE id = :id",
        named_params! { ":qty": qty.0, ":id": line_id.as_str() },
    )?;
    Ok(())
}

/// Records that the cart changed just now.
fn touch(tx: &Transaction, cart_id: &Id, now: Timestamp) -> Result<(), SalesError> {
    tx.execute(
        "UPDATE sales_carts SET updated_at = :now WHERE id = :id",
        named_params! { ":now": now.0, ":id": cart_id.as_str() },
    )?;
    Ok(())
}

fn check_qty(qty: Qty) -> Result<(), SalesError> {
    if qty.0 <= 0 {
        return Err(SalesError::invalid("qty_not_positive"));
    }
    Ok(())
}

fn check_price(price: Money) -> Result<(), SalesError> {
    if price.0 < 0 {
        return Err(SalesError::invalid("price_negative"));
    }
    Ok(())
}

/// A percentage must be above 0% and at most 100%; an amount must be above zero.
fn check_discount(discount: Option<Discount>) -> Result<(), SalesError> {
    let fine = match discount {
        None => true,
        Some(Discount::Percent(rate)) => rate.0 > 0 && rate.0 <= 10_000,
        Some(Discount::Amount(amount)) => amount.0 > 0,
    };
    if fine {
        Ok(())
    } else {
        Err(SalesError::invalid("discount_invalid"))
    }
}
