//! Taking payment and completing a sale (Req - Payments, Workflows "Complete sale").
//! The payment dialog asks `preview` after every change, so it never does money maths.
//! `complete` saves the sale, its stock movements and its shift payments in one transaction;
//! `complete_and_print` then prints the receipt and opens the drawer.

use std::collections::HashMap;

use pos_core::{AppError, Clock, Id, Money, RoundingRule, business_date, round_half_away};
use rusqlite::{Connection, Transaction};
use sales::{EnteredPayment, PaymentKind, Settlement};
use serde::{Deserialize, Serialize};
use settings::{Denominations, InventorySettings};
use shifts::{MovementKind, ShiftMovement};
use ts_rs::TS;

use crate::devices::Devices;
use crate::state::AppState;
use crate::workflows::cart_view::{self, CartView, PricedCart, keeps_stock};
use crate::workflows::checkout::{Selling, current_cart, selling, takes_payment_now};
use crate::workflows::payment_amounts::{amount_due, quick_cash};
use crate::workflows::receipts::{self, PrintResult};
use crate::workflows::sale_snapshot::completed_sale;

/// One row of the payment dialog as typed. For cash, `amount` is what the customer
/// handed over; for other methods, the amount charged.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct PaymentEntry {
    pub method_id: Id,
    pub amount: Money,
    /// A transaction id or similar (PAY-02). Never a card number.
    pub reference: Option<String>,
}

/// What the payment dialog shows for the amounts typed so far (PAY-01, PAY-03, PAY-06).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PaymentPreview {
    /// The cart's total before cash rounding.
    pub gross: Money,
    /// Cash rounding for these payments. Zero when nothing is paid in cash and
    /// rounding is for cash only.
    pub rounding: Money,
    /// The amount due for these payments: gross + rounding.
    pub due: Money,
    pub paid: Money,
    /// Still to pay. The sale completes only at zero.
    pub remaining: Money,
    /// Cash to give back. Shown large and green.
    pub change: Money,
    /// What the cash row should show when the rest is paid in cash.
    pub cash_to_pay: Money,
    /// The next useful notes above `cash_to_pay`. Example: due 470 → 500, 600, 1000.
    pub quick_cash: Vec<Money>,
}

/// A completed sale, and the empty cart for the next customer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SaleDone {
    pub sale_id: Id,
    /// Example: "T1-000123".
    pub invoice_no: String,
    pub total: Money,
    pub paid: Money,
    pub change: Money,
    /// True when cash was taken: the drawer opens (HW-04).
    pub cash_taken: bool,
    pub cart: CartView,
    /// How the receipt printed, after the sale was saved (PAY-09). None only inside
    /// `complete`, before printing.
    pub receipt: Option<PrintResult>,
}

/// The amounts for the payments typed so far. Saves nothing.
pub fn preview(
    state: &AppState,
    user: &Id,
    entries: &[PaymentEntry],
) -> Result<PaymentPreview, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let selling = selling(tx, user)?;
        let cart = current_cart(tx, clock, &selling, user)?;
        let priced = cart_view::price_cart(tx, clock, cart)?;
        let gross = priced.gross()?;
        let entered = entered_payments(tx, entries)?;
        let rule: RoundingRule = settings::get(tx)?;
        let due = amount_due(gross, &entered, &rule)?;
        let settled = sales::settle_payments(due.due, &entered)?;
        let denominations: Denominations = settings::get(tx)?;
        Ok(PaymentPreview {
            gross,
            rounding: due.due.checked_sub(gross)?,
            due: due.due,
            paid: settled.paid,
            remaining: settled.remaining,
            change: settled.change,
            cash_to_pay: due.cash_to_pay,
            quick_cash: quick_cash(due.cash_to_pay, &denominations.0),
        })
    })
}

/// Completes the sale in one transaction (PAY-07): the sale with its snapshot lines and
/// payments and the next invoice number, the stock movements (FEFO, kit parts), and the
/// shift payments. If any step fails nothing is saved and the cart stays as it was.
pub fn complete(
    state: &AppState,
    user: &Id,
    entries: &[PaymentEntry],
) -> Result<SaleDone, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let selling = selling(tx, user)?;
        let cart = current_cart(tx, clock, &selling, user)?;
        if cart.lines.is_empty() {
            return Err(AppError::validation("cart_empty"));
        }
        let priced = cart_view::price_cart(tx, clock, cart)?;
        let gross = priced.gross()?;
        let entered = entered_payments(tx, entries)?;
        let rule: RoundingRule = settings::get(tx)?;
        let due = amount_due(gross, &entered, &rule)?;
        let settled = sales::settle_payments(due.due, &entered)?;
        if settled.remaining.0 > 0 {
            return Err(AppError::Validation {
                field: None,
                code: "payment_not_enough".into(),
                params: [(
                    "remaining_minor".to_string(),
                    settled.remaining.0.to_string(),
                )]
                .into(),
            });
        }
        let sale = completed_sale(tx, clock, &selling, user, &priced, &settled, due.due)?;
        let total = sale.totals.total;
        let saved = sales::complete(tx, sale)?;
        record_stock(tx, clock, &selling, user, &priced, &saved.id)?;
        let cash_taken = record_payments(tx, clock, &selling, user, &entered, &settled, &saved.id)?;
        let fresh = current_cart(tx, clock, &selling, user)?;
        Ok(SaleDone {
            sale_id: saved.id,
            invoice_no: saved.invoice_no,
            total,
            paid: settled.paid,
            change: settled.change,
            cash_taken,
            cart: cart_view::cart_view(tx, clock, fresh)?,
            receipt: None,
        })
    })
}

/// Completes the sale, then prints its receipt and opens the drawer when cash was
/// taken (HW-04). A printer problem never undoes the saved sale (HW-08, PAY-09).
pub fn complete_and_print(
    state: &AppState,
    devices: &Devices,
    user: &Id,
    entries: &[PaymentEntry],
) -> Result<SaleDone, AppError> {
    let mut done = complete(state, user, entries)?;
    done.receipt = Some(receipts::print_receipt(
        state,
        devices,
        &done.sale_id,
        false,
        done.cash_taken,
    ));
    Ok(done)
}

/// The typed payments with their method's kind. Blank rows are skipped. Only methods
/// the till takes now are allowed (credit and points need a customer, C-M8).
fn entered_payments(
    conn: &Connection,
    entries: &[PaymentEntry],
) -> Result<Vec<EnteredPayment>, AppError> {
    let mut entered = Vec::new();
    for entry in entries.iter().filter(|entry| entry.amount.0 != 0) {
        let method = sales::payment_method(conn, &entry.method_id)?;
        if !method.active {
            return Err(AppError::validation("payment_method_inactive"));
        }
        if !takes_payment_now(method.kind) {
            return Err(AppError::validation("payment_needs_customer"));
        }
        let reference = entry
            .reference
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string);
        if method.requires_reference && reference.is_none() {
            return Err(AppError::validation("payment_reference_missing"));
        }
        entered.push(EnteredPayment {
            method_id: method.id,
            kind: method.kind,
            offered: entry.amount,
            reference,
        });
    }
    Ok(entered)
}

/// Takes the sold goods out of stock: FEFO for batches, the parts for a kit (CHK-14).
/// Products that keep no stock are skipped (INV-09).
fn record_stock(
    tx: &Transaction,
    clock: &dyn Clock,
    selling: &Selling,
    user: &Id,
    priced: &PricedCart,
    sale_id: &Id,
) -> Result<(), AppError> {
    let rules: InventorySettings = settings::get(tx)?;
    let now = clock.now();
    let ctx = inventory::MoveCtx {
        at: now,
        business_date: business_date(now, clock.local_offset_minutes(), selling.cutoff_hour)?,
        till_id: selling.till_id.clone(),
        user_id: Some(user.clone()),
    };
    let mut out: Vec<(catalog::Product, i64)> = Vec::new();
    for line in &priced.lines {
        if line.product.kind == catalog::ProductKind::Kit {
            for (part_id, part_qty) in catalog::kit_components(tx, &line.product.id)? {
                // Parts per kit × kits sold, both in thousandths. Example: 2 × 3 = 6.
                let part_total =
                    round_half_away(i128::from(part_qty.0) * i128::from(line.line.qty.0), 1000)?;
                out.push((catalog::get(tx, &part_id)?, part_total));
            }
        } else {
            out.push((line.product.clone(), line.line.qty.0));
        }
    }
    let mut by_product: HashMap<Id, (catalog::Product, i64)> = HashMap::new();
    let mut order = Vec::new();
    for (product, qty) in out {
        if !keeps_stock(&product) {
            continue;
        }
        let entry = by_product.entry(product.id.clone()).or_insert_with(|| {
            order.push(product.id.clone());
            (product, 0)
        });
        entry.1 += qty;
    }
    for product_id in order {
        let (product, qty) = &by_product[&product_id];
        let movement = inventory::Movement {
            batch: inventory::BatchChoice::Fefo,
            ref_: Some(("sale".to_string(), sale_id.clone())),
            unit_cost: product.cost_price,
            tracks_expiry: product.track_expiry,
            ..inventory::Movement::new(
                &product.id,
                pos_core::Qty(-qty),
                inventory::MoveKind::Sale,
                ctx.clone(),
            )
        };
        inventory::record(tx, movement, rules.allow_negative)?;
    }
    Ok(())
}

/// Records each payment on the shift: for cash, the amount kept after change.
/// Returns true when cash was taken.
fn record_payments(
    tx: &Transaction,
    clock: &dyn Clock,
    selling: &Selling,
    user: &Id,
    entered: &[EnteredPayment],
    settled: &Settlement,
    sale_id: &Id,
) -> Result<bool, AppError> {
    let mut cash_taken = false;
    for (payment, typed) in settled.payments.iter().zip(entered) {
        if typed.kind == PaymentKind::Cash {
            cash_taken = true;
        }
        if payment.amount.0 == 0 {
            continue;
        }
        shifts::record(
            tx,
            &selling.shift.id,
            ShiftMovement {
                kind: MovementKind::SalePayment,
                method_id: Some(payment.method_id.clone()),
                method_kind: typed.kind.as_str().to_string(),
                amount: payment.amount,
                ref_type: Some("sale".to_string()),
                ref_id: Some(sale_id.clone()),
                reason: None,
                at: clock.now(),
                user_id: user.clone(),
            },
        )?;
    }
    Ok(cash_taken)
}
