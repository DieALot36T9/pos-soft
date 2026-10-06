//! Taking payment and completing a sale (Req - Payments, Workflows "Complete sale").
//! The payment dialog asks `preview` after every change, so it never does money maths.
//! `complete` saves the sale, its stock movements and its shift payments in one transaction;
//! `complete_and_print` then prints the receipt and opens the drawer.

use std::collections::HashMap;

use auth::SessionUser;
use customers::{CustomerWithBalances, LedgerRef, PointsKind};
use pos_core::{
    AppError, Clock, Id, Money, Permission, RoundingRule, business_date, cash_round,
    round_half_away,
};
use rusqlite::{Connection, Transaction};
use sales::{Cart, EnteredPayment, PaymentKind, Settlement};
use serde::{Deserialize, Serialize};
use settings::{CreditSettings, Denominations, InventorySettings};
use shifts::{MovementKind, ShiftMovement};
use ts_rs::TS;

use crate::devices::Devices;
use crate::session;
use crate::state::AppState;
use crate::workflows::cart_view::{self, CartView, PricedCart, keeps_stock};
use crate::workflows::checkout::{Selling, current_cart, selling, takes_payment_now};
use crate::workflows::customer_records::{ledger_ctx, loyalty_view};
use crate::workflows::payment_amounts::{amount_due, quick_cash};
use crate::workflows::receipts::{self, PrintResult};
use crate::workflows::returns::{exchange_view, record_cash_refund};
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
    /// With a customer and loyalty on: the most that points may pay now (PAY-05).
    pub points_offer: Option<PointsOffer>,
    /// With a customer: how much more may go on credit before the limit (PAY-04).
    /// Store credit counts too. None without a customer.
    pub credit_available: Option<Money>,
    /// The new half of an exchange: the return's credit that pays for this sale (RET-05).
    pub exchange_credit: Money,
    /// The return's credit beyond this sale, given back in cash (RET-05).
    pub exchange_cash_back: Money,
}

/// What the customer's points can pay (PAY-05). Example: 1,200 points at 0.10 → 120.00.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PointsOffer {
    pub balance: i64,
    /// The most points usable now, in whole points. 0 under the minimum to redeem.
    pub max_points: i64,
    pub max_amount: Money,
    /// One point's value, so the screen can show "= 120 points".
    pub value_per_point: Money,
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
    /// Loyalty points the customer earned on this sale (CUS-04). 0 without a customer.
    pub points_earned: i64,
    /// An exchange's credit beyond this sale, given back in cash (RET-05).
    pub exchange_cash_back: Money,
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
        let mut entered = entered_payments(tx, entries, priced.customer.as_ref())?;
        let rule: RoundingRule = settings::get(tx)?;
        let exchange = use_exchange(tx, &priced.cart, gross, &mut entered, &rule)?;
        let due = amount_due(gross, &entered, &rule)?;
        let settled = sales::settle_payments(due.due, &entered)?;
        let denominations: Denominations = settings::get(tx)?;
        let (points_offer, credit_available) = match &priced.customer {
            Some(customer) => {
                let view = loyalty_view(tx)?;
                let max_points = customers::max_points_to_redeem(customer.points, due.due, &view);
                let offer = view.enabled.then(|| PointsOffer {
                    balance: customer.points,
                    max_points,
                    max_amount: Money(max_points.saturating_mul(view.value_per_point.0)),
                    value_per_point: view.value_per_point,
                });
                let available = customer
                    .customer
                    .credit_limit
                    .checked_sub(customer.credit)?
                    .max(Money::zero());
                (offer, Some(available))
            }
            None => (None, None),
        };
        Ok(PaymentPreview {
            gross,
            rounding: due.due.checked_sub(gross)?,
            due: due.due,
            paid: settled.paid,
            remaining: settled.remaining,
            change: settled.change,
            cash_to_pay: due.cash_to_pay,
            quick_cash: quick_cash(due.cash_to_pay, &denominations.0),
            points_offer,
            credit_available,
            exchange_credit: exchange.applied,
            exchange_cash_back: exchange.cash_back,
        })
    })
}

/// Completes the sale in one transaction (PAY-07): the sale with its snapshot lines and
/// payments and the next invoice number, the stock movements (FEFO, kit parts), and the
/// shift payments. If any step fails nothing is saved and the cart stays as it was.
pub fn complete(
    state: &AppState,
    seller: &SessionUser,
    entries: &[PaymentEntry],
    approval: Option<&str>,
) -> Result<SaleDone, AppError> {
    let clock = state.clock.as_ref();
    let user = &seller.id;
    state.db.write(|tx| {
        let selling = selling(tx, user)?;
        let cart = current_cart(tx, clock, &selling, user)?;
        if cart.lines.is_empty() {
            return Err(AppError::validation("cart_empty"));
        }
        let priced = cart_view::price_cart(tx, clock, cart)?;
        let gross = priced.gross()?;
        let mut entered = entered_payments(tx, entries, priced.customer.as_ref())?;
        let rule: RoundingRule = settings::get(tx)?;
        let exchange = use_exchange(tx, &priced.cart, gross, &mut entered, &rule)?;
        let due = amount_due(gross, &entered, &rule)?;
        let settled = sales::settle_payments(due.due, &entered)?;
        let by_customer = customer_payments(&entered, &settled)?;
        if let Some(customer) = &priced.customer {
            check_points(tx, customer, &by_customer, due.due)?;
            check_credit_limit(state, seller, customer, &by_customer, approval)?;
        }
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
        let mut cash_taken =
            record_payments(tx, clock, &selling, user, &entered, &settled, &saved.id)?;
        if let Some(return_id) = &exchange.return_id {
            let shift = &selling.shift.id;
            record_cash_refund(tx, clock, shift, user, exchange.cash_back, return_id)?;
            cash_taken |= exchange.cash_back.0 > 0;
        }
        let points_earned = match &priced.customer {
            Some(customer) => {
                record_customer(tx, clock, user, customer, &by_customer, total, &saved.id)?
            }
            None => 0,
        };
        let fresh = current_cart(tx, clock, &selling, user)?;
        Ok(SaleDone {
            sale_id: saved.id,
            invoice_no: saved.invoice_no,
            total,
            paid: settled.paid,
            change: settled.change,
            cash_taken,
            points_earned,
            exchange_cash_back: exchange.cash_back,
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
    seller: &SessionUser,
    entries: &[PaymentEntry],
    approval: Option<&str>,
) -> Result<SaleDone, AppError> {
    let mut done = complete(state, seller, entries, approval)?;
    done.receipt = Some(receipts::print_receipt(
        state,
        devices,
        &done.sale_id,
        false,
        done.cash_taken,
    ));
    Ok(done)
}

/// True when any typed payment is "Customer credit": the command then needs `SellOnCredit`.
pub fn uses_credit(state: &AppState, entries: &[PaymentEntry]) -> Result<bool, AppError> {
    state.db.read(|conn| {
        for entry in entries.iter().filter(|entry| entry.amount.0 != 0) {
            if sales::payment_method(conn, &entry.method_id)?.kind == PaymentKind::Credit {
                return Ok(true);
            }
        }
        Ok(false)
    })
}

/// How an exchange's credit pays for the new sale (RET-05).
struct ExchangeUse {
    return_id: Option<Id>,
    /// The part of the credit this sale uses.
    applied: Money,
    /// The rest, given back in cash after cash rounding.
    cash_back: Money,
}

/// Rule RET-05: on the new half of an exchange, the return's credit pays first. Only the
/// difference is paid; credit beyond the sale goes back in cash. The credit row is put
/// first among the payments, so the cashier never types it.
fn use_exchange(
    conn: &Connection,
    cart: &Cart,
    gross: Money,
    entered: &mut Vec<EnteredPayment>,
    rule: &RoundingRule,
) -> Result<ExchangeUse, AppError> {
    let Some(exchange) = exchange_view(conn, cart)? else {
        return Ok(ExchangeUse {
            return_id: None,
            applied: Money::zero(),
            cash_back: Money::zero(),
        });
    };
    let method = sales::system_payment_method(conn, PaymentKind::ExchangeCredit)?;
    let mut applied = exchange.credit.min(gross).max(Money::zero());
    if applied.0 > 0 {
        entered.insert(
            0,
            EnteredPayment {
                method_id: method.id,
                kind: PaymentKind::ExchangeCredit,
                offered: applied,
                reference: None,
            },
        );
        // Cash rounding for all payments may make the amount due less than the gross.
        let due = amount_due(gross, entered, rule)?.due;
        if applied > due {
            applied = due.max(Money::zero());
            if applied.0 > 0 {
                entered[0].offered = applied;
            } else {
                entered.remove(0);
            }
        }
    }
    Ok(ExchangeUse {
        return_id: Some(exchange.return_id),
        applied,
        cash_back: cash_round(exchange.credit.checked_sub(applied)?, rule)?,
    })
}

/// The typed payments with their method's kind. Blank rows are skipped. Customer credit
/// and points need an attached customer and their feature switched on (PAY-04, PAY-05);
/// exchange credit is never typed: it comes from the cart's return (RET-05).
fn entered_payments(
    conn: &Connection,
    entries: &[PaymentEntry],
    customer: Option<&CustomerWithBalances>,
) -> Result<Vec<EnteredPayment>, AppError> {
    let credit: CreditSettings = settings::get(conn)?;
    let loyalty = loyalty_view(conn)?;
    let mut entered = Vec::new();
    for entry in entries.iter().filter(|entry| entry.amount.0 != 0) {
        let method = sales::payment_method(conn, &entry.method_id)?;
        if !method.active {
            return Err(AppError::validation("payment_method_inactive"));
        }
        let customer_method = match method.kind {
            PaymentKind::Credit => credit.enabled,
            PaymentKind::Points => loyalty.enabled,
            _ => false,
        };
        if !takes_payment_now(method.kind) && !(customer_method && customer.is_some()) {
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
        // Exchange credit is no money in the drawer: the return gave it, this sale spends it.
        if payment.amount.0 == 0 || typed.kind == PaymentKind::ExchangeCredit {
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

/// What the customer paid from their own account on this sale.
#[derive(Default)]
struct CustomerMoney {
    points: Money,
    credit: Money,
}

/// Adds up the applied points and credit payments.
fn customer_payments(
    entered: &[EnteredPayment],
    settled: &Settlement,
) -> Result<CustomerMoney, AppError> {
    let mut money = CustomerMoney::default();
    for (payment, typed) in settled.payments.iter().zip(entered) {
        match typed.kind {
            PaymentKind::Points => money.points = money.points.checked_add(payment.amount)?,
            PaymentKind::Credit => money.credit = money.credit.checked_add(payment.amount)?,
            _ => {}
        }
    }
    Ok(money)
}

/// Rule PAY-05: points pay in whole points, up to what the balance and the minimum allow.
fn check_points(
    conn: &Connection,
    customer: &CustomerWithBalances,
    money: &CustomerMoney,
    due: Money,
) -> Result<(), AppError> {
    if money.points.0 == 0 {
        return Ok(());
    }
    let view = loyalty_view(conn)?;
    if view.value_per_point.0 <= 0 || money.points.0 % view.value_per_point.0 != 0 {
        return Err(AppError::validation("points_whole_only"));
    }
    let wanted = money.points.0 / view.value_per_point.0;
    let most = customers::max_points_to_redeem(customer.points, due, &view);
    if wanted > most {
        return Err(AppError::Validation {
            field: None,
            code: "points_over_limit".into(),
            params: [("max_points".to_string(), most.to_string())].into(),
        });
    }
    Ok(())
}

/// Rule PAY-04: new debt may not go over the customer's credit limit unless the user has
/// `CreditLimitExceed` or an admin approved it (USR-05). Store credit is used first.
fn check_credit_limit(
    state: &AppState,
    seller: &SessionUser,
    customer: &CustomerWithBalances,
    money: &CustomerMoney,
    approval: Option<&str>,
) -> Result<(), AppError> {
    if money.credit.0 == 0 {
        return Ok(());
    }
    let split = customers::split_credit_payment(customer.credit, money.credit)?;
    let owed_after = customer.credit.checked_add(money.credit)?;
    if split.new_debt.0 == 0 || owed_after <= customer.customer.credit_limit {
        return Ok(());
    }
    if seller.permissions.contains(Permission::CreditLimitExceed) {
        return Ok(());
    }
    let approved = match approval {
        Some(token) => session::take_approval(state, seller, Permission::CreditLimitExceed, token)?,
        None => None,
    };
    match approved {
        Some(_) => Ok(()),
        None => Err(AppError::OverrideRequired {
            permission: Permission::CreditLimitExceed,
        }),
    }
}

/// The customer's ledgers for the sale (CUS-04, CUS-05, CUS-06): points paid, the credit
/// payment (store credit first, then debt) and the points earned on the rest. Gives the
/// points earned.
fn record_customer(
    tx: &Transaction,
    clock: &dyn Clock,
    user: &Id,
    customer: &CustomerWithBalances,
    money: &CustomerMoney,
    total: Money,
    sale_id: &Id,
) -> Result<i64, AppError> {
    let id = &customer.customer.id;
    let ctx = ledger_ctx(tx, clock, user)?;
    let ref_ = LedgerRef {
        ref_type: "sale".into(),
        ref_id: sale_id.clone(),
    };
    let view = loyalty_view(tx)?;
    if money.points.0 > 0 {
        let points = money.points.0 / view.value_per_point.0;
        customers::add_points(tx, id, -points, PointsKind::Redeem, Some(&ref_), None, &ctx)?;
    }
    if money.credit.0 > 0 {
        customers::pay_with_credit(tx, id, money.credit, Some(&ref_), &ctx)?;
    }
    let eligible = customers::eligible_for_points(total, money.points, money.credit)?;
    let earned = customers::points_earned(eligible, &view);
    if earned > 0 && !customer.customer.loyalty_opt_out {
        customers::add_points(tx, id, earned, PointsKind::Earn, Some(&ref_), None, &ctx)?;
        return Ok(earned);
    }
    Ok(0)
}
