//! Holding, resuming and cancelling carts (CHK-05, CHK-09, CHK-10; Workflows
//! "Hold / resume / cancel cart"). Each is one transaction.

use pos_core::{AppError, Id, Money, Timestamp};
use serde::Serialize;
use ts_rs::TS;

use crate::audit_actions::CART_CANCEL;
use crate::audit_log;
use crate::state::AppState;
use crate::workflows::cart_view::{self, CartView};
use crate::workflows::checkout::{current_cart, own_cart, selling};
use crate::workflows::till::{conflict, till_settings};

/// One row of the held sales list (CHK-10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct HeldCartView {
    pub cart_id: Id,
    pub held_at: Timestamp,
    /// The cashier who held it. Example: "Rina".
    pub cashier_name: String,
    pub item_count: u32,
    /// The total at today's prices.
    pub total: Money,
    /// Example: "blue shirt, forgot rice".
    pub note: Option<String>,
}

/// A resumed cart, with the lines whose price changed while it was on hold (CHK-10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ResumedCart {
    pub cart: CartView,
    /// These lines show a "price changed" badge.
    pub price_changed: Vec<Id>,
}

/// Puts the cart aside with an optional note and starts an empty one (CHK-09).
/// Held carts are saved, so they survive restarts and power cuts.
pub fn hold(state: &AppState, user: &Id, note: Option<&str>) -> Result<CartView, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let selling = selling(tx, user)?;
        let cart = current_cart(tx, clock, &selling, user)?;
        // The exchange's credit belongs to this customer now: finish it or give it back.
        if cart.exchange_return_id.is_some() {
            return Err(conflict("exchange_in_progress"));
        }
        sales::hold(tx, &cart.id, note.map(str::to_string), clock.now())?;
        let fresh = current_cart(tx, clock, &selling, user)?;
        cart_view::cart_view(tx, clock, fresh)
    })
}

/// The carts on hold at this till, oldest first, with today's totals (CHK-10).
pub fn held_list(state: &AppState) -> Result<Vec<HeldCartView>, AppError> {
    let clock = state.clock.as_ref();
    state.db.read(|conn| {
        let till = till_settings(conn)?;
        let users = auth::list_users(conn)?;
        let mut rows = Vec::new();
        for held in sales::held_carts(conn, &till.id)? {
            let cart = sales::get_cart(conn, &held.cart_id)?;
            let view = cart_view::cart_view(conn, clock, cart)?;
            rows.push(HeldCartView {
                cart_id: held.cart_id,
                held_at: held.held_at,
                cashier_name: users
                    .iter()
                    .find(|each| each.id == held.user_id)
                    .map(|each| each.display_name.clone())
                    .unwrap_or_default(),
                item_count: view.totals.item_count,
                total: view.totals.total,
                note: held.note,
            });
        }
        Ok(rows)
    })
}

/// Brings a held cart back for this cashier and shift, repriced (CHK-10). Any cashier
/// with an open shift on this till may resume it. Refused while the current cart has items.
pub fn resume(state: &AppState, user: &Id, cart_id: &Id) -> Result<ResumedCart, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let selling = selling(tx, user)?;
        let held = sales::get_cart(tx, cart_id)?;
        if held.till_id != selling.till_id {
            return Err(AppError::NotFound {
                entity: "cart".into(),
                key: cart_id.to_string(),
            });
        }
        let held_at = held.updated_at;
        sales::resume(tx, cart_id, user, &selling.shift.id, clock.now())?;
        let cart = sales::get_cart(tx, cart_id)?;
        let mut price_changed = Vec::new();
        for line in &cart.lines {
            let changed = catalog::price_history(tx, &line.product_id)?
                .iter()
                .any(|row| row.field == catalog::PriceField::SalePrice && row.changed_at > held_at);
            if changed {
                price_changed.push(line.id.clone());
            }
        }
        Ok(ResumedCart {
            cart: cart_view::cart_view(tx, clock, cart)?,
            price_changed,
        })
    })
}

/// Throws the whole cart away after the cashier confirmed (CHK-05). The cart and its
/// lines go into the audit log; no invoice number is used. An empty cart stays as it is.
pub fn cancel(state: &AppState, user: &Id) -> Result<CartView, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let selling = selling(tx, user)?;
        let cart = own_cart(tx, clock, user)?;
        if cart.exchange_return_id.is_some() {
            return Err(conflict("exchange_in_progress"));
        }
        if cart.lines.is_empty() {
            return cart_view::cart_view(tx, clock, cart);
        }
        let priced = cart_view::price_cart(tx, clock, cart)?;
        let lines: Vec<serde_json::Value> = priced
            .lines
            .iter()
            .map(|line| {
                serde_json::json!({
                    "product_id": line.product.id,
                    "name": line.product.name,
                    "qty_milli": line.line.qty.0,
                    "amount_minor": line.tax.gross.0,
                })
            })
            .collect();
        let details = serde_json::json!({
            "cart_id": priced.cart.id,
            "shift_id": selling.shift.id,
            "total_minor": priced.gross()?.0,
            "lines": lines,
        });
        sales::cancel(tx, &priced.cart.id)?;
        audit_log::record(tx, clock, CART_CANCEL, details, Some(user.clone()))?;
        let fresh = current_cart(tx, clock, &selling, user)?;
        cart_view::cart_view(tx, clock, fresh)
    })
}
