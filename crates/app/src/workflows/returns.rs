//! Returns and exchanges (Req - Returns and Voids RET-01 to RET-07; Workflows "Return
//! items" and "Exchange"). A return is one transaction: the return with its number,
//! stock back (or written off), the customer's debt, store credit and points, the
//! refunds on the shift, and the audit event. The return receipt prints after the commit.
//!
//! Refunds are worked out by `sales` from the amounts stored on the sale, never from
//! today's prices (RET-02). A return without a receipt is priced here at today's price.

use std::collections::HashMap;

use auth::SessionUser;
use catalog::{Product, ProductKind};
use customers::{CreditKind, LedgerRef, PointsKind};
use pos_core::{AppError, Clock, CurrencyFormat, Id, Money, Permission, Qty, RoundingRule};
use pos_core::{Timestamp, cash_round, mul_qty, round_half_away};
use rusqlite::{Connection, Transaction, named_params};
use sales::{
    Cart, NewPayment, NewReturn, NewReturnLine, PaymentKind, PaymentMethod, ReturnQty,
    ReturnedProduct, ReturnedSaleLine, Sale, SaleStatus,
};
use serde::{Deserialize, Serialize};
use settings::TaxSettings;
use shifts::{MovementKind, ShiftMovement};
use ts_rs::TS;

use crate::audit_actions::{EXCHANGE_CASH_OUT, RETURN_COMPLETE};
use crate::audit_log;
use crate::devices::Devices;
use crate::state::AppState;
use crate::workflows::cart_view::{self, CartView, keeps_stock};
use crate::workflows::checkout::{MethodKind, Selling, current_cart, selling, takes_payment_now};
use crate::workflows::customer_records::{ledger_ctx, loyalty_view};
use crate::workflows::receipts::{self, PrintResult};
use crate::workflows::refund_plan::{CustomerMoneyLeft, points_to_take_back, split_refund};
use crate::workflows::stock::move_ctx;
use crate::workflows::till::{conflict, till_shift};

/// The reason written on points rows made by a return (the void's rows say "void").
const RETURN_REASON: &str = "return";

/// A sale found by its invoice number, for a return or a void (RET-01, RET-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ReturnSale {
    pub sale_id: Id,
    /// Example: "T1-000123".
    pub invoice_no: String,
    pub completed_at: Timestamp,
    pub customer: Option<ReturnCustomer>,
    pub voided: bool,
    pub lines: Vec<ReturnableLine>,
    /// How the sale was paid.
    pub payments: Vec<PaidWith>,
    pub total: Money,
    /// The method a refund goes back by unless the cashier picks another (RET-01):
    /// the sale's biggest cash, card, wallet or other payment, else cash.
    pub refund_method_id: Id,
    /// Earlier returns of this sale. Example: ["T1-R000003"].
    pub earlier_returns: Vec<String>,
    /// None: the sale can be voided. Otherwise why not: "sale_already_voided",
    /// "sale_has_returns" or "sale_shift_closed" (RET-04).
    pub void_blocked: Option<String>,
    pub currency: CurrencyFormat,
}

/// The customer of the sale being returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ReturnCustomer {
    pub id: Id,
    pub name: String,
}

/// One line of the sale with what can still be returned (RET-01).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ReturnableLine {
    pub sale_line_id: Id,
    pub product_id: Id,
    pub name: String,
    /// Example: "kg", "pc".
    pub unit_symbol: String,
    /// How many decimals a quantity may have. Piece is 0.
    pub unit_decimals: u8,
    pub sold: Qty,
    pub returned: Qty,
    /// sold − returned: the most that can come back now.
    pub returnable: Qty,
    /// What the customer paid for the whole line, after its discounts.
    pub amount: Money,
}

/// One payment of a sale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PaidWith {
    pub method_id: Id,
    /// Example: "bKash".
    pub name: String,
    pub kind: MethodKind,
    pub amount: Money,
}

/// A return as the return screen sends it. With a receipt, `sale_id` is set and every
/// line names its sale line; without one (RET-03), every line names a product.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct ReturnInput {
    pub sale_id: Option<Id>,
    pub lines: Vec<ReturnLineInput>,
    /// Why the goods came back. Needed to complete the return.
    pub reason: Option<String>,
    /// How the rest of the refund goes back: cash, card, wallet, other, or customer
    /// credit (store credit). An exchange ignores it.
    pub refund_method_id: Id,
    /// A transaction id or similar, when the method needs one (PAY-02).
    pub reference: Option<String>,
}

/// One line of a return.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct ReturnLineInput {
    /// With a receipt: the sale line.
    pub sale_line_id: Option<Id>,
    /// Without a receipt: the product.
    pub product_id: Option<Id>,
    pub qty: Qty,
    /// True: back on the shelf. False: written off as damaged.
    pub restock: bool,
    /// Without a receipt: a typed price for one unit (needs `PriceOverride`).
    /// None: today's price.
    pub unit_price: Option<Money>,
}

/// What a return would refund, line by line and by method. Saves nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ReturnPreview {
    pub lines: Vec<RefundLineView>,
    pub net: Money,
    pub tax: Money,
    pub gross: Money,
    /// Cash rounding of the refund (SET-06). total = gross + rounding.
    pub rounding: Money,
    pub total: Money,
    /// How the refund goes back: debt and points first (RET-07), then the chosen method.
    pub refunds: Vec<RefundView>,
    /// Loyalty points earned on the returned goods, taken back (RET-07).
    pub points_taken_back: i64,
}

/// One returned line with its refund.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RefundLineView {
    pub name: String,
    pub qty: Qty,
    pub net: Money,
    pub tax: Money,
    pub gross: Money,
    pub restock: bool,
}

/// One part of a refund.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct RefundView {
    pub method_id: Id,
    pub name: String,
    pub kind: MethodKind,
    pub amount: Money,
}

/// A saved return (RET-06).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ReturnDone {
    pub return_id: Id,
    /// Example: "T1-R000045".
    pub return_no: String,
    pub total: Money,
    pub refunds: Vec<RefundView>,
    /// Cash to hand back. The drawer opens for it.
    pub cash_back: Money,
    pub points_taken_back: i64,
    /// For an exchange: the new cart, with the refund as exchange credit (RET-05).
    pub cart: Option<CartView>,
    /// How the return receipt printed. None only inside `complete`, before printing.
    pub receipt: Option<PrintResult>,
}

/// The exchange credit given back in cash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ExchangeCashOut {
    /// Cash to hand back, after cash rounding.
    pub cash_back: Money,
    pub cart: CartView,
    /// How the drawer opened. None only inside the transaction.
    pub drawer: Option<PrintResult>,
}

/// The exchange credit on a cart, as the till shows it (RET-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ExchangeView {
    pub return_id: Id,
    /// Example: "T1-R000045".
    pub return_no: String,
    /// The refund that pays for the new sale.
    pub credit: Money,
}

/// Who returns the goods: the signed-in user and the admin who approved it (USR-05).
pub struct Returner<'a> {
    pub user: &'a SessionUser,
    pub approved_by: Option<&'a Id>,
}

/// Finds a sale by its invoice number, with what can still be returned (RET-01).
pub fn lookup(state: &AppState, invoice_no: &str) -> Result<ReturnSale, AppError> {
    state.db.read(|conn| {
        let sale = find_sale(conn, invoice_no)?;
        let returnable = sales::returnable_qty(conn, &sale.id)?;
        let decimals = unit_decimals(conn)?;
        let mut lines = Vec::with_capacity(sale.lines.len());
        for line in &sale.lines {
            let left = returnable.get(&line.id).copied().unwrap_or(Qty(0));
            lines.push(ReturnableLine {
                sale_line_id: line.id.clone(),
                product_id: line.snapshot.product_id.clone(),
                name: line.snapshot.name.clone(),
                unit_symbol: line.snapshot.unit_symbol.clone(),
                unit_decimals: decimals
                    .get(&line.snapshot.unit_symbol)
                    .copied()
                    .unwrap_or(0),
                sold: line.snapshot.qty,
                returned: Qty(line.snapshot.qty.0 - left.0),
                returnable: left,
                amount: line.snapshot.gross,
            });
        }
        let mut payments = Vec::with_capacity(sale.payments.len());
        for payment in &sale.payments {
            payments.push(PaidWith {
                method_id: payment.method_id.clone(),
                name: sales::payment_method(conn, &payment.method_id)?.name,
                kind: payment.kind.into(),
                amount: payment.amount,
            });
        }
        let earlier = earlier_returns(conn, &sale.id)?;
        Ok(ReturnSale {
            void_blocked: void_blocked(conn, &sale, !earlier.is_empty())?,
            sale_id: sale.id.clone(),
            invoice_no: sale.invoice_no.clone(),
            completed_at: sale.completed_at,
            customer: sale.customer.as_ref().map(|customer| ReturnCustomer {
                id: customer.id.clone(),
                name: customer.name.clone(),
            }),
            voided: sale.status == SaleStatus::Voided,
            lines,
            payments,
            total: sale.totals.total,
            refund_method_id: default_refund_method(conn, &sale)?,
            earlier_returns: earlier.into_iter().map(|(_, number)| number).collect(),
            currency: settings::get(conn)?,
        })
    })
}

/// What the return would refund, without saving anything.
pub fn preview(state: &AppState, input: &ReturnInput) -> Result<ReturnPreview, AppError> {
    state.db.read(|conn| {
        let planned = plan(conn, input, RefundTo::Chosen)?;
        Ok(planned.preview)
    })
}

/// Saves the return in one transaction (RET-01, RET-03, RET-07). Needs the user's own
/// open shift: the refunds go on it.
pub fn complete(
    state: &AppState,
    by: &Returner,
    input: &ReturnInput,
) -> Result<ReturnDone, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let selling = selling(tx, &by.user.id)?;
        let (done, _) = save_return(tx, clock, &selling, by, input, RefundTo::Chosen)?;
        Ok(done)
    })
}

/// Saves the return, then prints its receipt and opens the drawer for a cash refund.
/// A printer problem never undoes the saved return (HW-08).
pub fn complete_and_print(
    state: &AppState,
    devices: &Devices,
    by: &Returner,
    input: &ReturnInput,
) -> Result<ReturnDone, AppError> {
    let mut done = complete(state, by, input)?;
    done.receipt = Some(receipts::print_return(
        state,
        devices,
        &done.return_id,
        done.cash_back.0 > 0,
    ));
    Ok(done)
}

/// Exchange (RET-05): the return is saved with its refund as exchange credit, and the
/// user's empty cart becomes the new sale, which the credit pays for. The sale's
/// customer comes along to the new cart.
pub fn exchange_start(
    state: &AppState,
    devices: &Devices,
    by: &Returner,
    input: &ReturnInput,
) -> Result<ReturnDone, AppError> {
    let clock = state.clock.as_ref();
    let mut done = state.db.write(|tx| {
        let selling = selling(tx, &by.user.id)?;
        let cart = current_cart(tx, clock, &selling, &by.user.id)?;
        if !cart.lines.is_empty() {
            return Err(conflict("cart_in_progress"));
        }
        if cart.exchange_return_id.is_some() {
            return Err(conflict("exchange_in_progress"));
        }
        let (mut done, customer) = save_return(tx, clock, &selling, by, input, RefundTo::Exchange)?;
        let now = clock.now();
        sales::set_exchange_return(tx, &cart.id, Some(&done.return_id), now)?;
        if let Some(customer) = customer
            && customers::get(tx, &customer)?
                .customer
                .archived_at
                .is_none()
        {
            sales::set_customer(tx, &cart.id, Some(&customer), now)?;
        }
        let cart = sales::get_cart(tx, &cart.id)?;
        done.cart = Some(cart_view::cart_view(tx, clock, cart)?);
        Ok(done)
    })?;
    done.receipt = Some(receipts::print_return(
        state,
        devices,
        &done.return_id,
        false,
    ));
    Ok(done)
}

/// The customer took nothing new after all: the exchange credit goes back in cash and
/// the cart is an ordinary cart again. Audited `exchange.cash_out`; the drawer opens.
/// A cart with no exchange gives back nothing.
pub fn exchange_cash_out(
    state: &AppState,
    devices: &Devices,
    user: &Id,
) -> Result<ExchangeCashOut, AppError> {
    let clock = state.clock.as_ref();
    let mut out = state.db.write(|tx| {
        let selling = selling(tx, user)?;
        let cart = current_cart(tx, clock, &selling, user)?;
        let cash_back = pay_out_exchange(tx, clock, &cart, &selling.shift.id, user)?;
        let cart = sales::get_cart(tx, &cart.id)?;
        Ok(ExchangeCashOut {
            cash_back,
            cart: cart_view::cart_view(tx, clock, cart)?,
            drawer: None,
        })
    })?;
    if out.cash_back.0 > 0 {
        out.drawer = Some(receipts::open_drawer(state, devices));
    }
    Ok(out)
}

/// The exchange credit on a cart, if it is the new half of an exchange.
pub(crate) fn exchange_view(
    conn: &Connection,
    cart: &Cart,
) -> Result<Option<ExchangeView>, AppError> {
    let Some(return_id) = &cart.exchange_return_id else {
        return Ok(None);
    };
    let found = sales::get_return(conn, return_id)?;
    Ok(Some(ExchangeView {
        return_id: found.id.clone(),
        return_no: found.return_no.clone(),
        credit: paid_with_kind(&found.payments, PaymentKind::ExchangeCredit)?,
    }))
}

/// Gives a cart's exchange credit back in cash on the shift and takes it off the cart.
/// Used when the customer takes nothing, and when a shift is force closed with an
/// exchange still open. Gives the cash handed back (cash rounding applies).
pub(crate) fn pay_out_exchange(
    tx: &Transaction,
    clock: &dyn Clock,
    cart: &Cart,
    shift_id: &Id,
    user: &Id,
) -> Result<Money, AppError> {
    let Some(exchange) = exchange_view(tx, cart)? else {
        return Ok(Money::zero());
    };
    let rule: RoundingRule = settings::get(tx)?;
    let cash_back = cash_round(exchange.credit, &rule)?;
    record_cash_refund(tx, clock, shift_id, user, cash_back, &exchange.return_id)?;
    sales::set_exchange_return(tx, &cart.id, None, clock.now())?;
    audit_log::record(
        tx,
        clock,
        EXCHANGE_CASH_OUT,
        serde_json::json!({
            "return_id": exchange.return_id,
            "return_no": exchange.return_no,
            "credit_minor": exchange.credit.0,
            "cash_minor": cash_back.0,
        }),
        Some(user.clone()),
    )?;
    Ok(cash_back)
}

/// A cash refund on the shift for an exchange's credit, pointing at its return.
pub(crate) fn record_cash_refund(
    tx: &Transaction,
    clock: &dyn Clock,
    shift_id: &Id,
    user: &Id,
    amount: Money,
    return_id: &Id,
) -> Result<(), AppError> {
    if amount.0 <= 0 {
        return Ok(());
    }
    let cash = sales::system_payment_method(tx, PaymentKind::Cash)?;
    shifts::record(
        tx,
        shift_id,
        ShiftMovement {
            kind: MovementKind::Refund,
            method_id: Some(cash.id),
            method_kind: PaymentKind::Cash.as_str().to_string(),
            amount: amount.checked_neg()?,
            ref_type: Some("return".to_string()),
            ref_id: Some(return_id.clone()),
            reason: None,
            at: clock.now(),
            user_id: user.clone(),
        },
    )?;
    Ok(())
}

/// True when the input is a return without a receipt (RET-03): it needs
/// `ReturnWithoutReceipt` instead of `ReturnWithReceipt`.
pub fn permission_for(input: &ReturnInput) -> Permission {
    if input.sale_id.is_some() {
        Permission::ReturnWithReceipt
    } else {
        Permission::ReturnWithoutReceipt
    }
}

/// Where the rest of a refund goes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RefundTo {
    Chosen,
    Exchange,
}

/// Everything a return needs, worked out before anything is saved.
struct Planned {
    sale: Option<Sale>,
    lines: Vec<NewReturnLine>,
    /// Without a receipt: the lines whose price was typed (they need `PriceOverride`).
    typed_price: bool,
    preview: ReturnPreview,
    payments: Vec<(PaymentMethod, Money)>,
    reference: Option<String>,
    points_returned: i64,
}

fn plan(conn: &Connection, input: &ReturnInput, to: RefundTo) -> Result<Planned, AppError> {
    if input.lines.is_empty() {
        return Err(AppError::validation("return_empty"));
    }
    let mut planned = match &input.sale_id {
        Some(sale_id) => lines_from_sale(conn, sale_id, input)?,
        None => lines_without_receipt(conn, input)?,
    };
    let left = match &planned.sale {
        Some(sale) => customer_money_left(conn, sale)?,
        None => CustomerMoneyLeft::default(),
    };
    let rest_method = match to {
        RefundTo::Exchange => sales::system_payment_method(conn, PaymentKind::ExchangeCredit)?,
        RefundTo::Chosen => refund_method(conn, &input.refund_method_id, planned.sale.as_ref())?,
    };
    let reference = input
        .reference
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string);
    if rest_method.requires_reference && reference.is_none() {
        return Err(AppError::validation("payment_reference_missing"));
    }
    let loyalty = loyalty_view(conn)?;
    let rule: RoundingRule = settings::get(conn)?;
    let preview = &mut planned.preview;
    let split = split_refund(
        preview.gross,
        left,
        loyalty.value_per_point,
        rest_method.kind,
        &rule,
    )?;
    preview.rounding = split.rounding;
    preview.total = split.total;
    let mut payments = Vec::new();
    if split.credit.0 > 0 {
        payments.push((
            sales::system_payment_method(conn, PaymentKind::Credit)?,
            split.credit,
        ));
    }
    if split.points.0 > 0 {
        payments.push((
            sales::system_payment_method(conn, PaymentKind::Points)?,
            split.points,
        ));
        planned.points_returned = split.points.0 / loyalty.value_per_point.0;
    }
    if split.rest.0 > 0 {
        payments.push((rest_method, split.rest));
    }
    preview.refunds = payments
        .iter()
        .map(|(method, amount)| RefundView {
            method_id: method.id.clone(),
            name: method.name.clone(),
            kind: method.kind.into(),
            amount: *amount,
        })
        .collect();
    if let Some(sale) = &planned.sale {
        preview.points_taken_back = earned_points_back(conn, sale, preview.gross)?;
    }
    planned.payments = payments;
    planned.reference = reference;
    Ok(planned)
}

/// The refund for each line of a return with a receipt, from the stored sale (RET-02).
fn lines_from_sale(
    conn: &Connection,
    sale_id: &Id,
    input: &ReturnInput,
) -> Result<Planned, AppError> {
    let sale = sales::get_sale(conn, sale_id)?;
    let mut wanted = Vec::with_capacity(input.lines.len());
    let mut lines = Vec::with_capacity(input.lines.len());
    for line in &input.lines {
        let (Some(sale_line_id), None) = (&line.sale_line_id, &line.product_id) else {
            return Err(AppError::validation("return_lines_mixed"));
        };
        wanted.push(ReturnQty {
            sale_line_id: sale_line_id.clone(),
            qty: line.qty,
        });
        lines.push(NewReturnLine::FromSale(ReturnedSaleLine {
            sale_line_id: sale_line_id.clone(),
            qty: line.qty,
            restock: line.restock,
        }));
    }
    let refunds = sales::refund_amounts(conn, sale_id, &wanted)?;
    let mut preview = empty_preview();
    for (refund, line) in refunds.iter().zip(&input.lines) {
        add_line_view(
            &mut preview,
            RefundLineView {
                name: refund.name.clone(),
                qty: refund.qty,
                net: refund.amounts.net,
                tax: refund.amounts.tax,
                gross: refund.amounts.gross,
                restock: line.restock,
            },
        )?;
    }
    Ok(Planned {
        sale: Some(sale),
        lines,
        typed_price: false,
        preview,
        payments: Vec::new(),
        reference: None,
        points_returned: 0,
    })
}

/// Rule RET-03: without a receipt, each product is refunded at today's price (or a
/// typed one), with tax worked out as on a sale.
fn lines_without_receipt(conn: &Connection, input: &ReturnInput) -> Result<Planned, AppError> {
    let tax: TaxSettings = settings::get(conn)?;
    let mut preview = empty_preview();
    let mut lines = Vec::with_capacity(input.lines.len());
    let mut typed_price = false;
    for line in &input.lines {
        let (None, Some(product_id)) = (&line.sale_line_id, &line.product_id) else {
            return Err(AppError::validation("return_lines_mixed"));
        };
        if line.qty.0 <= 0 {
            return Err(AppError::validation("qty_not_positive"));
        }
        let product = catalog::get(conn, product_id)?;
        let price = match line.unit_price {
            Some(price) if price.0 < 0 => return Err(AppError::validation("price_negative")),
            Some(price) => {
                typed_price |= price != product.sale_price;
                price
            }
            None => product.sale_price,
        };
        let rates = tax::get_many(conn, std::slice::from_ref(&product.tax_rate_id))?;
        let rate = rates
            .get(&product.tax_rate_id)
            .ok_or_else(|| AppError::internal(format!("tax rate of {} is gone", product.id)))?;
        let amounts = tax::calc_line(mul_qty(price, line.qty)?, rate.rate_bps, tax.mode)?;
        add_line_view(
            &mut preview,
            RefundLineView {
                name: product.name.clone(),
                qty: line.qty,
                net: amounts.net,
                tax: amounts.tax,
                gross: amounts.gross,
                restock: line.restock,
            },
        )?;
        lines.push(NewReturnLine::WithoutReceipt(ReturnedProduct {
            product_id: product.id.clone(),
            name: product.name.clone(),
            qty: line.qty,
            net: amounts.net,
            tax: amounts.tax,
            gross: amounts.gross,
            tax_rate_id: product.tax_rate_id.clone(),
            tax_bps: rate.rate_bps,
            unit_cost: product.cost_price,
            restock: line.restock,
        }));
    }
    Ok(Planned {
        sale: None,
        lines,
        typed_price,
        preview,
        payments: Vec::new(),
        reference: None,
        points_returned: 0,
    })
}

fn empty_preview() -> ReturnPreview {
    ReturnPreview {
        lines: Vec::new(),
        net: Money::zero(),
        tax: Money::zero(),
        gross: Money::zero(),
        rounding: Money::zero(),
        total: Money::zero(),
        refunds: Vec::new(),
        points_taken_back: 0,
    }
}

fn add_line_view(preview: &mut ReturnPreview, line: RefundLineView) -> Result<(), AppError> {
    preview.net = preview.net.checked_add(line.net)?;
    preview.tax = preview.tax.checked_add(line.tax)?;
    preview.gross = preview.gross.checked_add(line.gross)?;
    preview.lines.push(line);
    Ok(())
}

/// The method chosen for the rest of the refund. Cash, card, wallet and other always;
/// customer credit (store credit) only for a sale with a customer.
fn refund_method(
    conn: &Connection,
    id: &Id,
    sale: Option<&Sale>,
) -> Result<PaymentMethod, AppError> {
    let method = sales::payment_method(conn, id)?;
    if !method.active {
        return Err(AppError::validation("payment_method_inactive"));
    }
    let has_customer = sale.is_some_and(|sale| sale.customer.is_some());
    match method.kind {
        kind if takes_payment_now(kind) => Ok(method),
        PaymentKind::Credit if has_customer => Ok(method),
        PaymentKind::Credit => Err(AppError::validation("payment_needs_customer")),
        _ => Err(AppError::validation("refund_method_not_allowed")),
    }
}

/// Saves the return and everything that goes with it, in the caller's transaction.
/// Gives the saved return and the sale's customer.
fn save_return(
    tx: &Transaction,
    clock: &dyn Clock,
    selling: &Selling,
    by: &Returner,
    input: &ReturnInput,
    to: RefundTo,
) -> Result<(ReturnDone, Option<Id>), AppError> {
    let reason = input
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| AppError::validation("return_reason_missing"))?;
    let planned = plan(tx, input, to)?;
    if planned.typed_price {
        require_price_override(tx, by)?;
    }
    let customer_id = planned
        .sale
        .as_ref()
        .and_then(|sale| sale.customer.as_ref())
        .map(|customer| customer.id.clone());
    let now = clock.now();
    let payments: Vec<NewPayment> = planned
        .payments
        .iter()
        .map(|(method, amount)| NewPayment {
            method_id: method.id.clone(),
            amount: *amount,
            tendered: None,
            reference: if method.kind == PaymentKind::Credit || method.kind == PaymentKind::Points {
                None
            } else {
                planned.reference.clone()
            },
        })
        .collect();
    let saved = sales::record_return(
        tx,
        NewReturn {
            sale_id: planned.sale.as_ref().map(|sale| sale.id.clone()),
            till_id: selling.till_id.clone(),
            till_code: selling.till_code.clone(),
            shift_id: selling.shift.id.clone(),
            user_id: by.user.id.clone(),
            approved_by: by.approved_by.cloned(),
            business_date: pos_core::business_date(
                now,
                clock.local_offset_minutes(),
                selling.cutoff_hour,
            )?,
            at: now,
            customer_id: customer_id.clone(),
            reason: Some(reason.to_string()),
            lines: planned.lines.clone(),
            rounding: planned.preview.rounding,
            payments,
        },
    )?;
    stock_back(tx, clock, &by.user.id, &planned, &saved.id)?;
    let cash_back = refunds_on_shift(tx, clock, selling, &by.user.id, &planned, &saved.id)?;
    if let Some(customer) = &customer_id {
        customer_back(tx, clock, &by.user.id, customer, &planned, &saved.id)?;
    }
    audit_return(tx, clock, by, &planned, &saved, to, reason)?;
    let done = ReturnDone {
        return_id: saved.id,
        return_no: saved.return_no,
        total: planned.preview.total,
        refunds: planned.preview.refunds.clone(),
        cash_back,
        points_taken_back: planned.preview.points_taken_back,
        cart: None,
        receipt: None,
    };
    Ok((done, customer_id))
}

/// Each refund goes on the user's shift as a `refund` row, except exchange credit,
/// which is no money leaving the drawer: the new sale spends it. Gives the cash refunded.
fn refunds_on_shift(
    tx: &Transaction,
    clock: &dyn Clock,
    selling: &Selling,
    user: &Id,
    planned: &Planned,
    return_id: &Id,
) -> Result<Money, AppError> {
    let mut cash_back = Money::zero();
    for (method, amount) in &planned.payments {
        if method.kind == PaymentKind::Cash {
            cash_back = cash_back.checked_add(*amount)?;
        }
        if method.kind == PaymentKind::ExchangeCredit {
            continue;
        }
        shifts::record(
            tx,
            &selling.shift.id,
            ShiftMovement {
                kind: MovementKind::Refund,
                method_id: Some(method.id.clone()),
                method_kind: method.kind.as_str().to_string(),
                amount: amount.checked_neg()?,
                ref_type: Some("return".to_string()),
                ref_id: Some(return_id.clone()),
                reason: None,
                at: clock.now(),
                user_id: user.clone(),
            },
        )?;
    }
    Ok(cash_back)
}

/// The audit event `return.complete`, with the admin who approved it (USR-05).
fn audit_return(
    tx: &Transaction,
    clock: &dyn Clock,
    by: &Returner,
    planned: &Planned,
    saved: &sales::ReturnRef,
    to: RefundTo,
    reason: &str,
) -> Result<(), AppError> {
    audit_log::record_approved(
        tx,
        clock,
        RETURN_COMPLETE,
        serde_json::json!({
            "return_id": saved.id,
            "return_no": saved.return_no,
            "sale_id": planned.sale.as_ref().map(|sale| sale.id.clone()),
            "invoice_no": planned.sale.as_ref().map(|sale| sale.invoice_no.clone()),
            "total_minor": planned.preview.total.0,
            "reason": reason,
            "exchange": to == RefundTo::Exchange,
            "lines": planned.preview.lines.iter().map(|line| serde_json::json!({
                "name": line.name, "qty_milli": line.qty.0, "gross_minor": line.gross.0,
                "restock": line.restock,
            })).collect::<Vec<_>>(),
        }),
        Some(by.user.id.clone()),
        by.approved_by.cloned(),
    )?;
    Ok(())
}

/// Rule RET-03: a typed price needs `PriceOverride`, from the user or from the admin
/// who approved the return.
fn require_price_override(conn: &Connection, by: &Returner) -> Result<(), AppError> {
    if by.user.permissions.contains(Permission::PriceOverride) {
        return Ok(());
    }
    if let Some(admin) = by.approved_by
        && auth::session_user(conn, admin)?
            .permissions
            .contains(Permission::PriceOverride)
    {
        return Ok(());
    }
    Err(AppError::PermissionDenied {
        permission: Permission::PriceOverride,
    })
}

/// Rule RET-07 on the customer's ledgers: debt off first then store credit for the
/// credit part, points paid back as points, and the points earned on the returned
/// goods taken back (the balance may go below zero).
fn customer_back(
    tx: &Transaction,
    clock: &dyn Clock,
    user: &Id,
    customer: &Id,
    planned: &Planned,
    return_id: &Id,
) -> Result<(), AppError> {
    let ctx = ledger_ctx(tx, clock, user)?;
    let ref_ = LedgerRef {
        ref_type: "return".into(),
        ref_id: return_id.clone(),
    };
    for (method, amount) in &planned.payments {
        if method.kind != PaymentKind::Credit {
            continue;
        }
        let debt = customers::get(tx, customer)?.credit.max(Money::zero());
        let off_debt = debt.min(*amount);
        let store_credit = amount.checked_sub(off_debt)?;
        if off_debt.0 > 0 {
            let kind = CreditKind::Return;
            customers::add_credit(
                tx,
                customer,
                off_debt.checked_neg()?,
                kind,
                None,
                Some(&ref_),
                &ctx,
            )?;
        }
        if store_credit.0 > 0 {
            let kind = CreditKind::StoreCredit;
            customers::add_credit(
                tx,
                customer,
                store_credit.checked_neg()?,
                kind,
                None,
                Some(&ref_),
                &ctx,
            )?;
        }
    }
    let reason = Some(RETURN_REASON);
    if planned.points_returned > 0 {
        let points = planned.points_returned;
        customers::add_points(
            tx,
            customer,
            points,
            PointsKind::Reverse,
            Some(&ref_),
            reason,
            &ctx,
        )?;
    }
    let taken = planned.preview.points_taken_back;
    if taken > 0 {
        customers::add_points(
            tx,
            customer,
            -taken,
            PointsKind::Reverse,
            Some(&ref_),
            reason,
            &ctx,
        )?;
    }
    Ok(())
}

/// The goods come back into stock (`return` rows), or come back and are written off at
/// once (`return` then `adjust` with reason damage), so the history shows both. With a
/// receipt, they go back into the batches the sale took them from, and only as much
/// as the sale took out: a product that kept no stock then stays without stock.
fn stock_back(
    tx: &Transaction,
    clock: &dyn Clock,
    user: &Id,
    planned: &Planned,
    return_id: &Id,
) -> Result<(), AppError> {
    let ctx = move_ctx(tx, clock, user)?;
    let mut taken_out: HashMap<Id, Vec<BatchOut>> = HashMap::new();
    for (restock, product_id, qty) in returned_goods(planned)? {
        for (product, qty) in parts_of(tx, &product_id, qty)? {
            let pieces = match &planned.sale {
                Some(sale) => {
                    let batches = match taken_out.get_mut(&product.id) {
                        Some(batches) => batches,
                        None => {
                            let found = batches_out(tx, &sale.id, &product.id)?;
                            taken_out.entry(product.id.clone()).or_insert(found)
                        }
                    };
                    share_out(batches, qty)
                }
                None if keeps_stock(&product) => vec![(None, qty, product.cost_price)],
                None => Vec::new(),
            };
            for (batch, piece, unit_cost) in pieces {
                stock_row(
                    tx, &ctx, &product, batch, piece, unit_cost, restock, return_id,
                )?;
            }
        }
    }
    Ok(())
}

/// Each returned line of the plan: whether it goes back on the shelf, its product and quantity.
fn returned_goods(planned: &Planned) -> Result<Vec<(bool, Id, Qty)>, AppError> {
    let mut goods = Vec::with_capacity(planned.lines.len());
    for line in &planned.lines {
        match line {
            NewReturnLine::FromSale(line) => {
                let sale = planned
                    .sale
                    .as_ref()
                    .ok_or_else(|| AppError::internal("a sale line without its sale"))?;
                let sold = sale
                    .lines
                    .iter()
                    .find(|each| each.id == line.sale_line_id)
                    .ok_or_else(|| AppError::internal("a returned line is not on the sale"))?;
                goods.push((line.restock, sold.snapshot.product_id.clone(), line.qty));
            }
            NewReturnLine::WithoutReceipt(product) => {
                goods.push((product.restock, product.product_id.clone(), product.qty));
            }
        }
    }
    Ok(goods)
}

/// A kit comes back as its parts (CHK-14); anything else as itself.
fn parts_of(conn: &Connection, product_id: &Id, qty: Qty) -> Result<Vec<(Product, Qty)>, AppError> {
    let product = catalog::get(conn, product_id)?;
    if product.kind != ProductKind::Kit {
        return Ok(vec![(product, qty)]);
    }
    let mut parts = Vec::new();
    for (part_id, part_qty) in catalog::kit_components(conn, &product.id)? {
        // Parts per kit × kits returned, both in thousandths. Example: 2 × 3 = 6.
        let total = round_half_away(i128::from(part_qty.0) * i128::from(qty.0), 1000)?;
        parts.push((catalog::get(conn, &part_id)?, Qty(total)));
    }
    Ok(parts)
}

/// How much of a product the sale took out of each batch and hasn't had back yet.
struct BatchOut {
    batch_id: Option<Id>,
    out: i64,
    unit_cost: Money,
}

/// Reads the published stock ledger (`inventory_v_ledger`): the sale's rows for the
/// product, less the rows of its earlier returns, batch by batch in the order taken.
fn batches_out(
    conn: &Connection,
    sale_id: &Id,
    product_id: &Id,
) -> Result<Vec<BatchOut>, AppError> {
    let mut statement = conn
        .prepare(
            "SELECT batch_id, qty_delta_milli, unit_cost_minor FROM inventory_v_ledger
         WHERE product_id = :product AND kind IN ('sale', 'return')
           AND ((ref_type = 'sale' AND ref_id = :sale)
             OR (ref_type = 'return' AND ref_id IN
                   (SELECT return_id FROM sales_v_return_lines WHERE sale_id = :sale)))
         ORDER BY at, id",
        )
        .map_err(sql_error)?;
    let rows = statement
        .query_map(
            named_params! { ":product": product_id.as_str(), ":sale": sale_id.as_str() },
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .map_err(sql_error)?;
    let mut batches: Vec<BatchOut> = Vec::new();
    for row in rows {
        let (batch, delta, cost) = row.map_err(sql_error)?;
        let batch_id = batch
            .map(|text| Id::parse(&text).map_err(|_| AppError::internal("a bad batch id")))
            .transpose()?;
        // A sale row takes out (negative); an earlier return's row brings back (positive).
        let out = -delta;
        match batches.iter_mut().find(|each| each.batch_id == batch_id) {
            Some(each) => each.out += out,
            None => batches.push(BatchOut {
                batch_id,
                out,
                unit_cost: Money(cost),
            }),
        }
    }
    Ok(batches)
}

/// Splits a returned quantity over the batches the sale took it from, first taken first.
/// Anything beyond what the sale took (a kit whose parts changed since) goes with the
/// last batch. Nothing comes back for a product the sale took nothing of.
fn share_out(batches: &mut [BatchOut], qty: Qty) -> Vec<(Option<Id>, Qty, Money)> {
    let mut pieces = Vec::new();
    let mut left = qty.0;
    for batch in batches.iter_mut() {
        if left == 0 {
            break;
        }
        let take = batch.out.min(left);
        if take > 0 {
            pieces.push((batch.batch_id.clone(), Qty(take), batch.unit_cost));
            batch.out -= take;
            left -= take;
        }
    }
    if left > 0
        && let Some(last) = batches.last()
    {
        pieces.push((last.batch_id.clone(), Qty(left), last.unit_cost));
    }
    pieces
}

#[allow(clippy::too_many_arguments)]
fn stock_row(
    tx: &Transaction,
    ctx: &inventory::MoveCtx,
    product: &Product,
    batch: Option<Id>,
    qty: Qty,
    unit_cost: Money,
    restock: bool,
    return_id: &Id,
) -> Result<(), AppError> {
    let choice = match &batch {
        Some(id) => inventory::BatchChoice::Specific(id.clone()),
        None => inventory::BatchChoice::None,
    };
    let back = inventory::Movement {
        batch: choice.clone(),
        ref_: Some(("return".to_string(), return_id.clone())),
        unit_cost,
        // Goods that left without a batch come back without one.
        tracks_expiry: product.track_expiry && batch.is_some(),
        ..inventory::Movement::new(&product.id, qty, inventory::MoveKind::Return, ctx.clone())
    };
    inventory::record(tx, back, true)?;
    if !restock {
        let written_off = inventory::Movement {
            batch: choice,
            reason_code: Some("damage".to_string()),
            ref_: Some(("return".to_string(), return_id.clone())),
            unit_cost,
            ..inventory::Movement::new(
                &product.id,
                Qty(-qty.0),
                inventory::MoveKind::Adjust,
                ctx.clone(),
            )
        };
        inventory::record(tx, written_off, true)?;
    }
    Ok(())
}

/// What is left to give back of the credit and points paid on the sale. Points paid
/// back by earlier returns are their points refunds; the credit part of each earlier
/// return is worked out again the same way (RET-07: the debt comes off first).
fn customer_money_left(conn: &Connection, sale: &Sale) -> Result<CustomerMoneyLeft, AppError> {
    let mut left = CustomerMoneyLeft {
        credit: paid_with_kind(&sale.payments, PaymentKind::Credit)?,
        points: paid_with_kind(&sale.payments, PaymentKind::Points)?,
    };
    for (return_id, _) in earlier_returns(conn, &sale.id)? {
        let earlier = sales::get_return(conn, &return_id)?;
        let credit = left.credit.min(earlier.totals.gross).max(Money::zero());
        left.credit = left.credit.checked_sub(credit)?;
        let points = paid_with_kind(&earlier.payments, PaymentKind::Points)?;
        left.points = left.points.checked_sub(points)?.max(Money::zero());
    }
    Ok(left)
}

/// Rule RET-07: the points earned on the sale, in the share now returned.
fn earned_points_back(conn: &Connection, sale: &Sale, gross: Money) -> Result<i64, AppError> {
    let earned: i64 = conn
        .query_row(
            "SELECT coalesce(sum(points_delta), 0) FROM customers_v_points_ledger
         WHERE ref_type = 'sale' AND ref_id = :sale AND kind = 'earn'",
            named_params! { ":sale": sale.id.as_str() },
            |row| row.get(0),
        )
        .map_err(sql_error)?;
    if earned == 0 {
        return Ok(0);
    }
    let mut refunded = gross;
    let mut taken_before = 0;
    for (return_id, _) in earlier_returns(conn, &sale.id)? {
        refunded = refunded.checked_add(sales::get_return(conn, &return_id)?.totals.gross)?;
        let taken: i64 = conn
            .query_row(
                "SELECT coalesce(-sum(points_delta), 0) FROM customers_v_points_ledger
             WHERE ref_type = 'return' AND ref_id = :return AND kind = 'reverse'
               AND points_delta < 0",
                named_params! { ":return": return_id.as_str() },
                |row| row.get(0),
            )
            .map_err(sql_error)?;
        taken_before += taken;
    }
    Ok(points_to_take_back(
        earned,
        sale.totals.gross,
        refunded,
        taken_before,
    ))
}

/// The sale's earlier returns, oldest first: id and number.
fn earlier_returns(conn: &Connection, sale_id: &Id) -> Result<Vec<(Id, String)>, AppError> {
    let mut statement = conn
        .prepare(
            "SELECT return_id, return_no FROM sales_v_return_lines WHERE sale_id = :sale
         GROUP BY return_id, return_no ORDER BY min(at), return_no",
        )
        .map_err(sql_error)?;
    let rows = statement
        .query_map(named_params! { ":sale": sale_id.as_str() }, |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(sql_error)?;
    let mut found = Vec::new();
    for row in rows {
        let (id, number) = row.map_err(sql_error)?;
        let id = Id::parse(&id).map_err(|_| AppError::internal("a bad return id"))?;
        found.push((id, number));
    }
    Ok(found)
}

/// Σ the payments of one kind.
pub(crate) fn paid_with_kind(
    payments: &[sales::Payment],
    kind: PaymentKind,
) -> Result<Money, AppError> {
    let mut total = Money::zero();
    for payment in payments.iter().filter(|payment| payment.kind == kind) {
        total = total.checked_add(payment.amount)?;
    }
    Ok(total)
}

/// Why a sale can't be voided, or None (RET-04).
fn void_blocked(
    conn: &Connection,
    sale: &Sale,
    has_returns: bool,
) -> Result<Option<String>, AppError> {
    if sale.status == SaleStatus::Voided {
        return Ok(Some("sale_already_voided".into()));
    }
    if has_returns {
        return Ok(Some("sale_has_returns".into()));
    }
    let open = till_shift(conn)?.is_some_and(|shift| shift.id == sale.shift_id);
    Ok((!open).then(|| "sale_shift_closed".into()))
}

/// The sale's biggest cash, card, wallet or other payment, else cash (RET-01).
fn default_refund_method(conn: &Connection, sale: &Sale) -> Result<Id, AppError> {
    let biggest = sale
        .payments
        .iter()
        .filter(|payment| takes_payment_now(payment.kind))
        .max_by_key(|payment| payment.amount);
    match biggest {
        Some(payment) if sales::payment_method(conn, &payment.method_id)?.active => {
            Ok(payment.method_id.clone())
        }
        _ => Ok(sales::system_payment_method(conn, PaymentKind::Cash)?.id),
    }
}

/// A sale by its invoice number, typed or scanned from the receipt's barcode.
pub(crate) fn find_sale(conn: &Connection, invoice_no: &str) -> Result<Sale, AppError> {
    sales::find_by_invoice(conn, invoice_no.trim())?.ok_or_else(|| AppError::NotFound {
        entity: "sale".into(),
        key: invoice_no.trim().into(),
    })
}

/// A database error while reading the published views. The details go to the log.
fn sql_error(error: rusqlite::Error) -> AppError {
    AppError::internal(format!("returns: {error}"))
}

/// Every unit symbol with its number of decimals. Example: "pc" → 0, "kg" → 3.
fn unit_decimals(conn: &Connection) -> Result<HashMap<String, u8>, AppError> {
    Ok(catalog::list_units(conn, true)?
        .into_iter()
        .map(|unit| (unit.symbol, unit.decimals))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(out: i64) -> BatchOut {
        BatchOut {
            batch_id: Some(Id::new()),
            out,
            unit_cost: Money(100),
        }
    }

    #[test]
    fn ret_01_returned_goods_go_back_into_the_batches_they_came_from() {
        let mut batches = vec![batch(2_000), batch(1_000)];
        let first = share_out(&mut batches, Qty(1_000));
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].0, batches[0].batch_id);
        let rest = share_out(&mut batches, Qty(2_000));
        let quantities: Vec<i64> = rest.iter().map(|piece| piece.1.0).collect();
        assert_eq!(quantities, vec![1_000, 1_000]);
        assert_eq!(rest[1].0, batches[1].batch_id);
    }

    #[test]
    fn inv_09_nothing_comes_back_for_a_product_the_sale_took_nothing_of() {
        assert!(share_out(&mut [], Qty(1_000)).is_empty());
    }
}
