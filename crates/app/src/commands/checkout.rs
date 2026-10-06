//! The checkout screen (Req - Checkout, Req - Payments). Selling needs `CheckoutSell`;
//! typing a weight also needs `CheckoutManualWeight`; holding and resuming need
//! `CheckoutHoldResume`; cancelling a cart needs `CheckoutCancelCart`. Discounts above
//! the role's limit, a typed price (`PriceOverride`), another price list
//! (`PriceListSwitch`) and credit over the limit (`CreditLimitExceed`) take an admin's
//! approval token (USR-05). Attaching a customer needs `CustomerViewCreate`; paying with
//! customer credit needs `SellOnCredit`.

use std::time::Duration;

use pos_core::{AppError, Id, Money, Permission, Qty};
use pricing::ManualDiscount;
use sales::LineSource;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::devices::Devices;
use crate::session;
use crate::state::AppState;
use crate::workflows::cart_lines;
use crate::workflows::cart_pricing::{self, DiscountOn};
use crate::workflows::cart_view::CartView;
use crate::workflows::checkout::{self, CheckoutView, SaleHit, ScanResult};
use crate::workflows::complete_sale::{self, PaymentEntry, PaymentPreview, SaleDone};
use crate::workflows::held_carts::{self, HeldCartView, ResumedCart};
use crate::workflows::weights;

/// How long adding a weighed item waits for a stable weight (CHK-07: 2 seconds).
const SCALE_WAIT: Duration = Duration::from_secs(2);
/// How long one live reading in the weight dialog waits, so the dialog stays quick.
const LIVE_WAIT: Duration = Duration::from_millis(300);

/// Where a weight comes from (CHK-06, CHK-07).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum WeightInput {
    /// Read the scale now. Only a stable weight above zero is taken.
    Scale,
    /// Typed by hand. Example: "1.25" or "1250 g". Needs `CheckoutManualWeight`.
    Manual { text: String },
}

/// What the weight dialog shows while the item is on the scale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ScaleView {
    /// False when no scale is set up: the weight must be typed.
    pub has_scale: bool,
    /// The weight in the product's unit, when the scale answered.
    pub qty: Option<Qty>,
    /// The scale says the weight has settled: the green "Stable" badge.
    pub stable: bool,
    /// Why there is no weight, as an error code. Example: "timeout".
    pub problem: Option<String>,
}

pub fn checkout_state(state: &AppState) -> Result<CheckoutView, AppError> {
    let user = session::require(state, Permission::CheckoutSell)?;
    checkout::load(state, &user.id, cart_pricing::own_limit(&user))
}

pub fn checkout_scan(state: &AppState, input: &str) -> Result<ScanResult, AppError> {
    let user = session::require(state, Permission::CheckoutSell)?;
    cart_lines::scan(state, &user.id, input)
}

pub fn checkout_search(state: &AppState, text: &str) -> Result<Vec<SaleHit>, AppError> {
    session::require(state, Permission::CheckoutSell)?;
    cart_lines::search(state, text)
}

pub fn checkout_add_product(
    state: &AppState,
    product_id: &Id,
    qty: Option<Qty>,
) -> Result<ScanResult, AppError> {
    let user = session::require(state, Permission::CheckoutSell)?;
    cart_lines::add_product(state, &user.id, product_id, qty)
}

/// One live reading for the weight dialog. A missing scale or a timeout is not an
/// error here: the dialog shows it and offers typing the weight.
pub fn checkout_read_scale(
    state: &AppState,
    devices: &Devices,
    product_id: &Id,
) -> Result<ScaleView, AppError> {
    session::require(state, Permission::CheckoutSell)?;
    let Some(scale) = devices.scale(&hardware_settings(state)?) else {
        return Ok(ScaleView {
            has_scale: false,
            qty: None,
            stable: false,
            problem: Some("scale_not_set_up".into()),
        });
    };
    let unit = cart_lines::weighed_unit(state, product_id)?;
    let view = match scale.read(LIVE_WAIT) {
        Ok(reading) => {
            let qty = weights::weight_from_reading(
                &hardware::Reading {
                    stable: true,
                    ..reading.clone()
                },
                &unit,
            );
            match qty {
                Ok(qty) => ScaleView {
                    has_scale: true,
                    qty: Some(qty),
                    stable: reading.stable,
                    problem: None,
                },
                Err(error) => ScaleView {
                    has_scale: true,
                    qty: None,
                    stable: false,
                    problem: Some(code_of(&error)),
                },
            }
        }
        Err(error) => ScaleView {
            has_scale: true,
            qty: None,
            stable: false,
            problem: Some(error.code().to_string()),
        },
    };
    Ok(view)
}

pub fn checkout_add_weighed(
    state: &AppState,
    devices: &Devices,
    product_id: &Id,
    weight: &WeightInput,
) -> Result<CartView, AppError> {
    let user = session::require(state, Permission::CheckoutSell)?;
    let unit = cart_lines::weighed_unit(state, product_id)?;
    let (qty, source) = match weight {
        WeightInput::Scale => {
            let scale = devices
                .scale(&hardware_settings(state)?)
                .ok_or_else(|| AppError::validation("scale_not_set_up"))?;
            let reading = scale.read(SCALE_WAIT)?;
            (
                weights::weight_from_reading(&reading, &unit)?,
                LineSource::Scan,
            )
        }
        WeightInput::Manual { text } => {
            session::require(state, Permission::CheckoutManualWeight)?;
            (weights::parse_weight(text, &unit)?, LineSource::Manual)
        }
    };
    cart_lines::add_weighed(state, &user.id, product_id, qty, source)
}

pub fn checkout_set_qty(state: &AppState, line_id: &Id, qty: Qty) -> Result<CartView, AppError> {
    let user = session::require(state, Permission::CheckoutSell)?;
    cart_lines::set_qty(state, &user.id, line_id, qty)
}

pub fn checkout_remove_line(state: &AppState, line_id: &Id) -> Result<CartView, AppError> {
    let user = session::require(state, Permission::CheckoutSell)?;
    cart_lines::remove_line(state, &user.id, line_id)
}

pub fn checkout_hold(state: &AppState, note: Option<&str>) -> Result<CartView, AppError> {
    let user = session::require(state, Permission::CheckoutHoldResume)?;
    held_carts::hold(state, &user.id, note)
}

pub fn checkout_held_list(state: &AppState) -> Result<Vec<HeldCartView>, AppError> {
    session::require(state, Permission::CheckoutHoldResume)?;
    held_carts::held_list(state)
}

pub fn checkout_resume(state: &AppState, cart_id: &Id) -> Result<ResumedCart, AppError> {
    let user = session::require(state, Permission::CheckoutHoldResume)?;
    held_carts::resume(state, &user.id, cart_id)
}

pub fn checkout_cancel(state: &AppState) -> Result<CartView, AppError> {
    let user = session::require(state, Permission::CheckoutCancelCart)?;
    held_carts::cancel(state, &user.id)
}

pub fn checkout_payment_preview(
    state: &AppState,
    payments: &[PaymentEntry],
) -> Result<PaymentPreview, AppError> {
    let user = session::require(state, Permission::CheckoutSell)?;
    complete_sale::preview(state, &user.id, payments)
}

/// Completes the sale, then prints the receipt and opens the drawer for cash (PAY-09).
pub fn checkout_complete(
    state: &AppState,
    devices: &Devices,
    payments: &[PaymentEntry],
) -> Result<SaleDone, AppError> {
    checkout_complete_approved(state, devices, payments, None)
}

/// `checkout_complete` with an admin's approval token, for credit over the customer's
/// limit (PAY-04). Paying with customer credit needs `SellOnCredit`.
pub fn checkout_complete_approved(
    state: &AppState,
    devices: &Devices,
    payments: &[PaymentEntry],
    approval: Option<&str>,
) -> Result<SaleDone, AppError> {
    let user = session::require(state, Permission::CheckoutSell)?;
    if complete_sale::uses_credit(state, payments)? {
        session::require(state, Permission::SellOnCredit)?;
    }
    complete_sale::complete_and_print(state, devices, &user, payments, approval)
}

/// A line's discount (`F7`, CHK-12). Above the role's limit it needs an approval token.
pub fn checkout_set_discount(
    state: &AppState,
    line_id: &Id,
    discount: Option<ManualDiscount>,
    approval: Option<&str>,
) -> Result<CartView, AppError> {
    let user = session::require(state, Permission::CheckoutSell)?;
    let on = DiscountOn::Line(line_id.clone());
    cart_pricing::set_discount(state, &user, &on, discount, approval)
}

/// The discount on the whole cart (`Shift+F7`, CHK-12). Same limit as line discounts.
pub fn checkout_set_invoice_discount(
    state: &AppState,
    discount: Option<ManualDiscount>,
    approval: Option<&str>,
) -> Result<CartView, AppError> {
    let user = session::require(state, Permission::CheckoutSell)?;
    cart_pricing::set_discount(state, &user, &DiscountOn::Invoice, discount, approval)
}

/// A typed unit price for a line (`F8`, CHK-13). None goes back to the normal price.
pub fn checkout_price_override(
    state: &AppState,
    line_id: &Id,
    price: Option<Money>,
    approval: Option<&str>,
) -> Result<CartView, AppError> {
    let allowed = session::require_or_approval(state, Permission::PriceOverride, approval)?;
    session::require(state, Permission::CheckoutSell)?;
    cart_pricing::override_price(
        state,
        &allowed.user.id,
        allowed.approved_by.as_ref(),
        line_id,
        price,
    )
}

/// Attaches a customer to the cart (`F3`, CHK-11), or takes them off with None.
pub fn checkout_set_customer(
    state: &AppState,
    customer_id: Option<&Id>,
) -> Result<CartView, AppError> {
    session::require(state, Permission::CustomerViewCreate)?;
    let user = session::require(state, Permission::CheckoutSell)?;
    cart_pricing::set_customer(state, &user.id, customer_id)
}

/// Switches the cart to another price list (CHK-17), or back to normal prices with None.
pub fn checkout_set_price_list(
    state: &AppState,
    price_list_id: Option<&Id>,
    approval: Option<&str>,
) -> Result<CartView, AppError> {
    let allowed = session::require_or_approval(state, Permission::PriceListSwitch, approval)?;
    session::require(state, Permission::CheckoutSell)?;
    cart_pricing::set_price_list(state, &allowed.user.id, price_list_id)
}

/// The saved device settings: which printer and scale to use.
fn hardware_settings(state: &AppState) -> Result<settings::HardwareSettings, AppError> {
    state.db.read(|conn| Ok(settings::get(conn)?))
}

fn code_of(error: &AppError) -> String {
    match error {
        AppError::Validation { code, .. } | AppError::Conflict { code, .. } => code.clone(),
        _ => "weight_invalid".into(),
    }
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::{AppError, Id, Money, Qty};
    use pricing::ManualDiscount;
    use tauri::State;

    use super::{ScaleView, WeightInput};
    use crate::commands::on_worker;
    use crate::devices::Devices;
    use crate::state::AppState;
    use crate::workflows::cart_view::CartView;
    use crate::workflows::checkout::{CheckoutView, SaleHit, ScanResult};
    use crate::workflows::complete_sale::{PaymentEntry, PaymentPreview, SaleDone};
    use crate::workflows::held_carts::{HeldCartView, ResumedCart};

    #[tauri::command]
    pub async fn checkout_state(state: State<'_, Arc<AppState>>) -> Result<CheckoutView, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_state", move || super::checkout_state(&state)).await
    }

    #[tauri::command]
    pub async fn checkout_scan(
        state: State<'_, Arc<AppState>>,
        input: String,
    ) -> Result<ScanResult, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_scan", move || {
            super::checkout_scan(&state, &input)
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_search(
        state: State<'_, Arc<AppState>>,
        text: String,
    ) -> Result<Vec<SaleHit>, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_search", move || {
            super::checkout_search(&state, &text)
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_add_product(
        state: State<'_, Arc<AppState>>,
        product_id: Id,
        qty: Option<Qty>,
    ) -> Result<ScanResult, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_add_product", move || {
            super::checkout_add_product(&state, &product_id, qty)
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_read_scale(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        product_id: Id,
    ) -> Result<ScaleView, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("checkout_read_scale", move || {
            super::checkout_read_scale(&state, &devices, &product_id)
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_add_weighed(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        product_id: Id,
        weight: WeightInput,
    ) -> Result<CartView, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("checkout_add_weighed", move || {
            super::checkout_add_weighed(&state, &devices, &product_id, &weight)
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_set_qty(
        state: State<'_, Arc<AppState>>,
        line_id: Id,
        qty: Qty,
    ) -> Result<CartView, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_set_qty", move || {
            super::checkout_set_qty(&state, &line_id, qty)
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_remove_line(
        state: State<'_, Arc<AppState>>,
        line_id: Id,
    ) -> Result<CartView, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_remove_line", move || {
            super::checkout_remove_line(&state, &line_id)
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_hold(
        state: State<'_, Arc<AppState>>,
        note: Option<String>,
    ) -> Result<CartView, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_hold", move || {
            super::checkout_hold(&state, note.as_deref())
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_held_list(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<HeldCartView>, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_held_list", move || {
            super::checkout_held_list(&state)
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_resume(
        state: State<'_, Arc<AppState>>,
        cart_id: Id,
    ) -> Result<ResumedCart, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_resume", move || {
            super::checkout_resume(&state, &cart_id)
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_cancel(state: State<'_, Arc<AppState>>) -> Result<CartView, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_cancel", move || super::checkout_cancel(&state)).await
    }

    #[tauri::command]
    pub async fn checkout_payment_preview(
        state: State<'_, Arc<AppState>>,
        payments: Vec<PaymentEntry>,
    ) -> Result<PaymentPreview, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_payment_preview", move || {
            super::checkout_payment_preview(&state, &payments)
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_complete(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        payments: Vec<PaymentEntry>,
        approval: Option<String>,
    ) -> Result<SaleDone, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("checkout_complete", move || {
            super::checkout_complete_approved(&state, &devices, &payments, approval.as_deref())
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_set_discount(
        state: State<'_, Arc<AppState>>,
        line_id: Id,
        discount: Option<ManualDiscount>,
        approval: Option<String>,
    ) -> Result<CartView, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_set_discount", move || {
            super::checkout_set_discount(&state, &line_id, discount, approval.as_deref())
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_set_invoice_discount(
        state: State<'_, Arc<AppState>>,
        discount: Option<ManualDiscount>,
        approval: Option<String>,
    ) -> Result<CartView, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_set_invoice_discount", move || {
            super::checkout_set_invoice_discount(&state, discount, approval.as_deref())
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_price_override(
        state: State<'_, Arc<AppState>>,
        line_id: Id,
        price: Option<Money>,
        approval: Option<String>,
    ) -> Result<CartView, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_price_override", move || {
            super::checkout_price_override(&state, &line_id, price, approval.as_deref())
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_set_customer(
        state: State<'_, Arc<AppState>>,
        customer_id: Option<Id>,
    ) -> Result<CartView, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_set_customer", move || {
            super::checkout_set_customer(&state, customer_id.as_ref())
        })
        .await
    }

    #[tauri::command]
    pub async fn checkout_set_price_list(
        state: State<'_, Arc<AppState>>,
        price_list_id: Option<Id>,
        approval: Option<String>,
    ) -> Result<CartView, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_set_price_list", move || {
            super::checkout_set_price_list(&state, price_list_id.as_ref(), approval.as_deref())
        })
        .await
    }
}
