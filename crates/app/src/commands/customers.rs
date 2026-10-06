//! The customer screens (Req - Customers and Loyalty). Looking up, adding and reading
//! customers needs `CustomerViewCreate`; changing or archiving one needs
//! `CustomerManage`; a debt payment needs `CustomerReceivePayment`; a points correction
//! needs `LoyaltyAdjust`.

use customers::Statement;
use pos_core::{AppError, BusinessDate, Id, Permission};

use crate::session;
use crate::state::AppState;
use crate::workflows::customer_records::{
    self, CustomerPage, CustomerPaymentDone, CustomerPaymentInput, CustomerRow, CustomerSave,
};

pub fn customer_lookup(state: &AppState, text: &str) -> Result<Vec<CustomerRow>, AppError> {
    session::require(state, Permission::CustomerViewCreate)?;
    customer_records::lookup(state, text)
}

pub fn customer_list(
    state: &AppState,
    text: &str,
    include_archived: bool,
) -> Result<Vec<CustomerRow>, AppError> {
    session::require(state, Permission::CustomerViewCreate)?;
    customer_records::list(state, text, include_archived)
}

pub fn customer_get(state: &AppState, id: &Id) -> Result<CustomerPage, AppError> {
    session::require(state, Permission::CustomerViewCreate)?;
    customer_records::page(state, id)
}

/// A new customer needs `CustomerViewCreate` (quick create, CUS-02: no tier, the shop's
/// default credit limit); changing one, or setting the tier and limit, needs `CustomerManage`.
pub fn customer_save(state: &AppState, input: &CustomerSave) -> Result<Id, AppError> {
    let user = session::require(state, Permission::CustomerViewCreate)?;
    if input.id.is_some() {
        session::require(state, Permission::CustomerManage)?;
    }
    let can_manage = user.permissions.contains(Permission::CustomerManage);
    customer_records::save(state, &user.id, can_manage, input)
}

pub fn customer_archive(state: &AppState, id: &Id, archived: bool) -> Result<(), AppError> {
    let user = session::require(state, Permission::CustomerManage)?;
    customer_records::set_archived(state, &user.id, id, archived)
}

pub fn customer_payment(
    state: &AppState,
    input: &CustomerPaymentInput,
) -> Result<CustomerPaymentDone, AppError> {
    let user = session::require(state, Permission::CustomerReceivePayment)?;
    customer_records::take_payment(state, &user.id, input)
}

pub fn points_adjust(
    state: &AppState,
    customer_id: &Id,
    points: i64,
    reason: &str,
) -> Result<i64, AppError> {
    let user = session::require(state, Permission::LoyaltyAdjust)?;
    customer_records::adjust_points(state, &user.id, customer_id, points, reason)
}

pub fn customer_statement(
    state: &AppState,
    id: &Id,
    from: BusinessDate,
    to: BusinessDate,
) -> Result<Statement, AppError> {
    session::require(state, Permission::CustomerViewCreate)?;
    customer_records::statement(state, id, from, to)
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use customers::Statement;
    use pos_core::{AppError, BusinessDate, Id};
    use tauri::State;

    use crate::commands::on_worker;
    use crate::state::AppState;
    use crate::workflows::customer_records::{
        CustomerPage, CustomerPaymentDone, CustomerPaymentInput, CustomerRow, CustomerSave,
    };

    #[tauri::command]
    pub async fn customer_lookup(
        state: State<'_, Arc<AppState>>,
        text: String,
    ) -> Result<Vec<CustomerRow>, AppError> {
        let state = Arc::clone(&state);
        on_worker("customer_lookup", move || {
            super::customer_lookup(&state, &text)
        })
        .await
    }

    #[tauri::command]
    pub async fn customer_list(
        state: State<'_, Arc<AppState>>,
        text: String,
        include_archived: bool,
    ) -> Result<Vec<CustomerRow>, AppError> {
        let state = Arc::clone(&state);
        on_worker("customer_list", move || {
            super::customer_list(&state, &text, include_archived)
        })
        .await
    }

    #[tauri::command]
    pub async fn customer_get(
        state: State<'_, Arc<AppState>>,
        id: Id,
    ) -> Result<CustomerPage, AppError> {
        let state = Arc::clone(&state);
        on_worker("customer_get", move || super::customer_get(&state, &id)).await
    }

    #[tauri::command]
    pub async fn customer_save(
        state: State<'_, Arc<AppState>>,
        input: CustomerSave,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("customer_save", move || {
            super::customer_save(&state, &input)
        })
        .await
    }

    #[tauri::command]
    pub async fn customer_archive(
        state: State<'_, Arc<AppState>>,
        id: Id,
        archived: bool,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("customer_archive", move || {
            super::customer_archive(&state, &id, archived)
        })
        .await
    }

    #[tauri::command]
    pub async fn customer_payment(
        state: State<'_, Arc<AppState>>,
        input: CustomerPaymentInput,
    ) -> Result<CustomerPaymentDone, AppError> {
        let state = Arc::clone(&state);
        on_worker("customer_payment", move || {
            super::customer_payment(&state, &input)
        })
        .await
    }

    #[tauri::command]
    pub async fn points_adjust(
        state: State<'_, Arc<AppState>>,
        customer_id: Id,
        points: i64,
        reason: String,
    ) -> Result<i64, AppError> {
        let state = Arc::clone(&state);
        on_worker("points_adjust", move || {
            super::points_adjust(&state, &customer_id, points, &reason)
        })
        .await
    }

    #[tauri::command]
    pub async fn customer_statement(
        state: State<'_, Arc<AppState>>,
        id: Id,
        from: BusinessDate,
        to: BusinessDate,
    ) -> Result<Statement, AppError> {
        let state = Arc::clone(&state);
        on_worker("customer_statement", move || {
            super::customer_statement(&state, &id, from, to)
        })
        .await
    }
}
