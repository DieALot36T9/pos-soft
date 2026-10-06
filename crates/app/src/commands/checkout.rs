//! The checkout screen (Req - Checkout, Req - Payments). Selling needs `CheckoutSell`;
//! typing a weight also needs `CheckoutManualWeight`; holding and resuming need
//! `CheckoutHoldResume`; cancelling a cart needs `CheckoutCancelCart`.

use std::time::Duration;

use pos_core::{AppError, Id, Permission, Qty};
use sales::LineSource;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::devices::Devices;
use crate::session;
use crate::state::AppState;
use crate::workflows::cart_lines;
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
    checkout::load(state, &user.id)
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
    let Some(scale) = devices.scale() else {
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
                .scale()
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

pub fn checkout_complete(
    state: &AppState,
    payments: &[PaymentEntry],
) -> Result<SaleDone, AppError> {
    let user = session::require(state, Permission::CheckoutSell)?;
    complete_sale::complete(state, &user.id, payments)
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

    use pos_core::{AppError, Id, Qty};
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
        payments: Vec<PaymentEntry>,
    ) -> Result<SaleDone, AppError> {
        let state = Arc::clone(&state);
        on_worker("checkout_complete", move || {
            super::checkout_complete(&state, &payments)
        })
        .await
    }
}
