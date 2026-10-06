//! The checkout screen's starting point (Req - Checkout): what it loads, the shift the
//! cashier sells on, their cart, and the payment methods. Changing the cart is in
//! `cart_lines`; paying is in `complete_sale`.

use ::shifts::Shift;
use pos_core::{AppError, Clock, CurrencyFormat, Id, Money, Qty};
use rusqlite::{Connection, Transaction};
use sales::{Cart, PaymentKind};
use serde::{Deserialize, Serialize};
use settings::CheckoutSettings;
use ts_rs::TS;

use crate::state::AppState;
use crate::workflows::cart_view::{self, CartView};
use crate::workflows::till::{self as shifts, ShiftView, conflict};

/// Everything the checkout screen loads when it opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CheckoutView {
    /// The till's open shift. None: the screen asks to open one (CHK-20).
    pub shift: Option<ShiftView>,
    /// The cashier's cart. None while the till has no shift of theirs.
    pub cart: Option<CartView>,
    /// Carts on hold at this till: the amber "Held (2)" count (CHK-10).
    pub held_count: u32,
    /// The ways to pay, in list order (PAY-01, PAY-02).
    pub payment_methods: Vec<PaymentMethodView>,
    /// Beeps on a good scan and on errors (CHK-22).
    pub sounds: bool,
    pub currency: CurrencyFormat,
}

/// The kind of a payment method, as the screen colours it (Design System).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum MethodKind {
    Cash,
    Card,
    Wallet,
    Other,
    Credit,
    Points,
    ExchangeCredit,
}

impl From<PaymentKind> for MethodKind {
    fn from(kind: PaymentKind) -> MethodKind {
        match kind {
            PaymentKind::Cash => MethodKind::Cash,
            PaymentKind::Card => MethodKind::Card,
            PaymentKind::Wallet => MethodKind::Wallet,
            PaymentKind::Other => MethodKind::Other,
            PaymentKind::Credit => MethodKind::Credit,
            PaymentKind::Points => MethodKind::Points,
            PaymentKind::ExchangeCredit => MethodKind::ExchangeCredit,
        }
    }
}

/// One row of the payment dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PaymentMethodView {
    pub id: Id,
    /// Example: "bKash".
    pub name: String,
    pub kind: MethodKind,
    /// A reference such as a transaction id must be typed (PAY-02).
    pub requires_reference: bool,
}

/// A product that must be weighed before it goes in the cart (CHK-06, CHK-07).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct WeighProduct {
    pub product_id: Id,
    pub name: String,
    /// Example: "kg".
    pub unit_symbol: String,
    /// The price for one unit, for "price × weight = amount" in the dialog.
    pub unit_price: Money,
}

/// One product in the till's search list (CHK-03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct SaleHit {
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub sale_price: Money,
    pub unit_symbol: String,
    pub on_hand: Qty,
    /// The small picture as a data address, if the product has one.
    pub thumbnail: Option<String>,
}

/// What a scan or a pick from the search list did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum ScanResult {
    /// The product is in the cart. `line_id` is the line it went into (select it).
    Added { cart: Box<CartView>, line_id: Id },
    /// A weighed product: the screen opens the weight dialog.
    NeedsWeight { product: WeighProduct },
    /// The text wasn't a code: pick one of these.
    Search { hits: Vec<SaleHit> },
}

/// The seller's place: the till and the shift the signed-in user sells on.
pub(crate) struct Selling {
    pub till_id: Id,
    pub till_code: String,
    pub cutoff_hour: u8,
    pub shift: Shift,
}

/// The till and the user's open shift on it. Refused without one (CHK-20), and on a
/// shift that belongs to someone else (SHF-02).
pub(crate) fn selling(conn: &Connection, user: &Id) -> Result<Selling, AppError> {
    let till = shifts::till_settings(conn)?;
    let shift = shifts::till_shift(conn)?.ok_or_else(|| conflict("shift_not_open"))?;
    if shift.user_id != *user {
        return Err(conflict("shift_not_yours"));
    }
    Ok(Selling {
        till_id: till.id,
        till_code: till.till.till_code,
        cutoff_hour: till.till.business_day_cutoff_hour,
        shift,
    })
}

/// The cashier's open cart on this shift, made if there is none. A cart left on an
/// older shift is held (any cashier can resume it), or thrown away when empty.
pub(crate) fn current_cart(
    tx: &Transaction,
    clock: &dyn Clock,
    selling: &Selling,
    user: &Id,
) -> Result<Cart, AppError> {
    if let Some(cart) = sales::current_cart(tx, &selling.till_id, user)? {
        if cart.shift_id == selling.shift.id {
            return Ok(cart);
        }
        if cart.lines.is_empty() {
            sales::cancel(tx, &cart.id)?;
        } else {
            sales::hold(tx, &cart.id, None, clock.now())?;
        }
    }
    let id = sales::open_cart(tx, &selling.till_id, &selling.shift.id, user, clock.now())?;
    Ok(sales::get_cart(tx, &id)?)
}

/// What the checkout screen shows when it opens. The cart comes back exactly as it
/// was saved, also after a crash or restart (CHK-19).
pub fn load(state: &AppState, user: &Id) -> Result<CheckoutView, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let shift = shifts::till_shift(tx)?;
        let cart = match &shift {
            Some(open) if open.user_id == *user => {
                let selling = selling(tx, user)?;
                let cart = current_cart(tx, clock, &selling, user)?;
                Some(cart_view::cart_view(tx, clock, cart)?)
            }
            _ => None,
        };
        let till = shifts::till_settings(tx)?;
        let checkout: CheckoutSettings = settings::get(tx)?;
        Ok(CheckoutView {
            shift: shift
                .map(|open| shifts::shift_view(tx, &open, user))
                .transpose()?,
            cart,
            held_count: u32::try_from(sales::held_carts(tx, &till.id)?.len()).unwrap_or(u32::MAX),
            payment_methods: payment_methods(tx)?,
            sounds: checkout.sounds,
            currency: settings::get(tx)?,
        })
    })
}

/// The active payment methods the till can take. Customer credit shows when selling on
/// credit is switched on, loyalty points when loyalty is on; both need an attached
/// customer when paying (PAY-04, PAY-05). Exchange credit needs a return (C-M9).
pub(crate) fn payment_methods(conn: &Connection) -> Result<Vec<PaymentMethodView>, AppError> {
    let credit: settings::CreditSettings = settings::get(conn)?;
    let loyalty: settings::LoyaltySettings = settings::get(conn)?;
    Ok(sales::payment_methods(conn)?
        .into_iter()
        .filter(|method| {
            method.active
                && (takes_payment_now(method.kind)
                    || (method.kind == PaymentKind::Credit && credit.enabled)
                    || (method.kind == PaymentKind::Points && loyalty.enabled))
        })
        .map(|method| PaymentMethodView {
            id: method.id,
            name: method.name,
            kind: method.kind.into(),
            requires_reference: method.requires_reference,
        })
        .collect())
}

/// Cash, card, wallet and other methods. The rest need a customer or a return.
pub(crate) fn takes_payment_now(kind: PaymentKind) -> bool {
    matches!(
        kind,
        PaymentKind::Cash | PaymentKind::Card | PaymentKind::Wallet | PaymentKind::Other
    )
}

/// The user's cart on their open shift, in a transaction.
pub(crate) fn own_cart(tx: &Transaction, clock: &dyn Clock, user: &Id) -> Result<Cart, AppError> {
    let selling = selling(tx, user)?;
    current_cart(tx, clock, &selling, user)
}
