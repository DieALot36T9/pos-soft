//! Changing how the cart is priced at the till (Req - Checkout): manual discounts on a
//! line or the whole cart (CHK-12, PRC-07), a typed unit price (CHK-13), the customer
//! whose tier applies (CHK-11, PRC-03) and the price list (CHK-17). Each change is one
//! transaction and ends with the repriced cart.

use auth::SessionUser;
use pos_core::{AppError, Bps, Id, Money, Permission};
use pricing::ManualDiscount;
use rusqlite::Transaction;
use sales::{Discount, PriceOverride};

use crate::audit_actions::{DISCOUNT_OVER_LIMIT, PRICE_OVERRIDE};
use crate::audit_log;
use crate::session;
use crate::state::AppState;
use crate::workflows::cart_view::{self, CartView};
use crate::workflows::checkout::own_cart;

/// Which discount the cashier is changing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscountOn {
    /// One line (`F7`).
    Line(Id),
    /// The whole cart (`Shift+F7`).
    Invoice,
}

/// Sets or clears a manual discount (CHK-12). A discount above the user's limit needs an
/// admin's approval token for `DiscountApply` (PRC-07, USR-05); the approving admin's own
/// limit must cover it. A change that doesn't raise the cart's biggest discount, such as
/// taking a discount off, never needs one. Approved discounts are audited.
pub fn set_discount(
    state: &AppState,
    user: &SessionUser,
    on: &DiscountOn,
    discount: Option<ManualDiscount>,
    approval: Option<&str>,
) -> Result<CartView, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let cart = own_cart(tx, clock, &user.id)?;
        if let DiscountOn::Line(line_id) = on
            && !cart.lines.iter().any(|line| line.id == *line_id)
        {
            return Err(not_found_line(line_id));
        }
        let before = biggest_discount(tx, state, cart.clone())?;
        let stored = discount.map(sales_discount);
        match on {
            DiscountOn::Line(line_id) => {
                sales::set_line_discount(tx, line_id, stored, clock.now())?;
            }
            DiscountOn::Invoice => sales::set_invoice_discount(tx, &cart.id, stored, clock.now())?,
        }
        let changed = sales::get_cart(tx, &cart.id)?;
        let after = biggest_discount(tx, state, changed.clone())?;
        if after > before && after > own_limit(user) {
            let approved_by = approver(tx, state, user, after, approval)?;
            let details = serde_json::json!({
                "cart_id": cart.id,
                "on": match on { DiscountOn::Line(_) => "line", DiscountOn::Invoice => "invoice" },
                "line_id": match on { DiscountOn::Line(id) => Some(id.clone()), DiscountOn::Invoice => None },
                "discount": discount,
                "discount_bps": after.0,
                "limit_bps": own_limit(user).0,
            });
            audit_log::record_approved(
                tx,
                clock,
                DISCOUNT_OVER_LIMIT,
                details,
                Some(user.id.clone()),
                Some(approved_by),
            )?;
        }
        cart_view::cart_view(tx, clock, changed)
    })
}

/// Sets or clears a line's typed unit price (CHK-13). The command checked
/// `PriceOverride` or took an approval. The old and new prices go into the audit log.
pub fn override_price(
    state: &AppState,
    user: &Id,
    approved_by: Option<&Id>,
    line_id: &Id,
    price: Option<Money>,
) -> Result<CartView, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let cart = own_cart(tx, clock, user)?;
        let priced = cart_view::price_cart(tx, clock, cart.clone())?;
        let line = priced
            .lines
            .iter()
            .find(|line| line.line.id == *line_id)
            .ok_or_else(|| not_found_line(line_id))?;
        let details = serde_json::json!({
            "cart_id": cart.id,
            "line_id": line_id,
            "product_id": line.product.id,
            "name": line.product.name,
            "old_unit_price_minor": line.price.unit_price.0,
            "retail_price_minor": line.retail_price.0,
            "new_unit_price_minor": price.map(|price| price.0),
        });
        let change = price.map(|price| PriceOverride {
            price,
            by: user.clone(),
        });
        sales::set_price_override(tx, line_id, change, clock.now())?;
        audit_log::record_approved(
            tx,
            clock,
            PRICE_OVERRIDE,
            details,
            Some(user.clone()),
            approved_by.cloned(),
        )?;
        cart_view::cart_view(tx, clock, sales::get_cart(tx, &cart.id)?)
    })
}

/// Attaches a customer to the cart, or takes them off (CHK-11). Their tier's price list
/// and discount apply at once. Archived customers can't be attached.
pub fn set_customer(
    state: &AppState,
    user: &Id,
    customer_id: Option<&Id>,
) -> Result<CartView, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let cart = own_cart(tx, clock, user)?;
        if let Some(customer_id) = customer_id {
            let customer = customers::get(tx, customer_id)?;
            if customer.customer.archived_at.is_some() {
                return Err(AppError::validation("customer_archived"));
            }
        }
        sales::set_customer(tx, &cart.id, customer_id, clock.now())?;
        cart_view::cart_view(tx, clock, sales::get_cart(tx, &cart.id)?)
    })
}

/// Switches the cart to another price list, or back to normal prices (CHK-17). The
/// command checked `PriceListSwitch` or took an approval. A switched-off list is refused.
pub fn set_price_list(
    state: &AppState,
    user: &Id,
    price_list_id: Option<&Id>,
) -> Result<CartView, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let cart = own_cart(tx, clock, user)?;
        if let Some(price_list_id) = price_list_id {
            let list = pricing::price_lists(tx)?
                .into_iter()
                .find(|list| list.id == *price_list_id)
                .ok_or_else(|| AppError::NotFound {
                    entity: "price_list".into(),
                    key: price_list_id.to_string(),
                })?;
            if !list.active {
                return Err(AppError::validation("price_list_inactive"));
            }
        }
        sales::set_price_list(tx, &cart.id, price_list_id, clock.now())?;
        cart_view::cart_view(tx, clock, sales::get_cart(tx, &cart.id)?)
    })
}

/// The biggest manual discount on the cart as a percentage (PRC-07).
fn biggest_discount(
    tx: &Transaction,
    state: &AppState,
    cart: sales::Cart,
) -> Result<Bps, AppError> {
    let priced = cart_view::price_cart(tx, state.clock.as_ref(), cart)?;
    let lines: Vec<pricing::PriceLineOut> =
        priced.lines.into_iter().map(|line| line.price).collect();
    Ok(pricing::manual_discount_bps(&lines))
}

/// The biggest discount the user may give alone: unlimited for an admin, the role's
/// limit with `DiscountApply`, otherwise none.
pub(crate) fn own_limit(user: &SessionUser) -> Bps {
    if user.is_admin {
        Bps(u32::MAX)
    } else if user.permissions.contains(Permission::DiscountApply) {
        user.max_discount
    } else {
        Bps(0)
    }
}

/// The admin who approved a discount of `wanted`, from the approval token. Without a
/// fitting token, or when the approving user's own limit is lower, the screen must ask
/// for an approval (`OverrideRequired`).
fn approver(
    tx: &Transaction,
    state: &AppState,
    user: &SessionUser,
    wanted: Bps,
    approval: Option<&str>,
) -> Result<Id, AppError> {
    let needed = AppError::OverrideRequired {
        permission: Permission::DiscountApply,
    };
    let Some(token) = approval else {
        return Err(needed);
    };
    let Some(approved_by) = session::take_approval(state, user, Permission::DiscountApply, token)?
    else {
        return Err(needed);
    };
    let admin = auth::session_user(tx, &approved_by)?;
    if own_limit(&admin) < wanted {
        return Err(needed);
    }
    Ok(approved_by)
}

fn sales_discount(discount: ManualDiscount) -> Discount {
    match discount {
        ManualDiscount::Percent(rate) => Discount::Percent(rate),
        ManualDiscount::Amount(amount) => Discount::Amount(amount),
    }
}

fn not_found_line(line_id: &Id) -> AppError {
    AppError::NotFound {
        entity: "cart_line".into(),
        key: line_id.to_string(),
    }
}
