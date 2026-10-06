//! The stock screens (Req - Inventory). Reading needs `StockView`; adjusting and
//! "Verify stock data" need `StockAdjust`; counting needs `StockCountEnter`; starting,
//! reviewing, posting and cancelling a count need `StockCountManage`.

use catalog::ProductPage;
use inventory::Reason;
use pos_core::{AppError, Id, Permission, Qty};

use crate::session;
use crate::state::AppState;
use crate::workflows::stock::{
    self, AdjustInput, HistoryQuery, HistoryRow, LowStockView, MismatchView, NearExpiryView,
    StockAlerts, StockFilter, StockList, StockProduct,
};
use crate::workflows::stock_counts::{
    self, CountInput, CountLineInput, CountPosted, CountSummary, CountView, VarianceReport,
};

pub fn stock_levels(
    state: &AppState,
    filter: &StockFilter,
    page: ProductPage,
) -> Result<StockList, AppError> {
    session::require(state, Permission::StockView)?;
    stock::levels(state, filter, page)
}

pub fn stock_product(state: &AppState, id: &Id) -> Result<StockProduct, AppError> {
    session::require(state, Permission::StockView)?;
    stock::product(state, id)
}

pub fn stock_lookup(state: &AppState, code: &str) -> Result<StockProduct, AppError> {
    session::require(state, Permission::StockView)?;
    stock::lookup(state, code)
}

pub fn stock_history(state: &AppState, query: &HistoryQuery) -> Result<Vec<HistoryRow>, AppError> {
    session::require(state, Permission::StockView)?;
    stock::history(state, query)
}

pub fn stock_reasons(state: &AppState) -> Result<Vec<Reason>, AppError> {
    session::require(state, Permission::StockView)?;
    stock::reasons(state)
}

pub fn stock_adjust(state: &AppState, input: &AdjustInput) -> Result<Qty, AppError> {
    let by = session::require(state, Permission::StockAdjust)?;
    stock::adjust(state, &by.id, input)
}

pub fn low_stock(state: &AppState, page: ProductPage) -> Result<Vec<LowStockView>, AppError> {
    session::require(state, Permission::StockView)?;
    stock::low_stock(state, page)
}

pub fn near_expiry(state: &AppState) -> Result<Vec<NearExpiryView>, AppError> {
    session::require(state, Permission::StockView)?;
    stock::near_expiry(state)
}

pub fn stock_alerts(state: &AppState) -> Result<StockAlerts, AppError> {
    session::require(state, Permission::StockView)?;
    stock::alerts(state)
}

pub fn stock_verify(state: &AppState) -> Result<Vec<MismatchView>, AppError> {
    session::require(state, Permission::StockAdjust)?;
    stock::verify(state)
}

pub fn count_list(state: &AppState, page: ProductPage) -> Result<Vec<CountSummary>, AppError> {
    session::require(state, Permission::StockCountEnter)?;
    stock_counts::list(state, page)
}

pub fn count_get(state: &AppState, id: &Id) -> Result<CountView, AppError> {
    session::require(state, Permission::StockCountEnter)?;
    stock_counts::view(state, id)
}

pub fn count_create(state: &AppState, input: &CountInput) -> Result<Id, AppError> {
    let by = session::require(state, Permission::StockCountManage)?;
    stock_counts::create(state, &by.id, input)
}

pub fn count_line(
    state: &AppState,
    count_id: &Id,
    line: &CountLineInput,
) -> Result<CountView, AppError> {
    let by = session::require(state, Permission::StockCountEnter)?;
    stock_counts::count_line(state, &by.id, count_id, line)
}

pub fn count_variances(state: &AppState, id: &Id) -> Result<VarianceReport, AppError> {
    session::require(state, Permission::StockCountManage)?;
    stock_counts::variances(state, id)
}

pub fn count_post(
    state: &AppState,
    id: &Id,
    confirm_uncounted: bool,
) -> Result<CountPosted, AppError> {
    let by = session::require(state, Permission::StockCountManage)?;
    stock_counts::post(state, &by.id, id, confirm_uncounted)
}

pub fn count_cancel(state: &AppState, id: &Id) -> Result<(), AppError> {
    let by = session::require(state, Permission::StockCountManage)?;
    stock_counts::cancel(state, &by.id, id)
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use catalog::ProductPage;
    use inventory::Reason;
    use pos_core::{AppError, Id, Qty};
    use tauri::State;

    use crate::commands::on_worker;
    use crate::state::AppState;
    use crate::workflows::stock::{
        AdjustInput, HistoryQuery, HistoryRow, LowStockView, MismatchView, NearExpiryView,
        StockAlerts, StockFilter, StockList, StockProduct,
    };
    use crate::workflows::stock_counts::{
        CountInput, CountLineInput, CountPosted, CountSummary, CountView, VarianceReport,
    };

    #[tauri::command]
    pub async fn stock_levels(
        state: State<'_, Arc<AppState>>,
        filter: StockFilter,
        page: ProductPage,
    ) -> Result<StockList, AppError> {
        let state = Arc::clone(&state);
        on_worker("stock_levels", move || {
            super::stock_levels(&state, &filter, page)
        })
        .await
    }

    #[tauri::command]
    pub async fn stock_product(
        state: State<'_, Arc<AppState>>,
        id: Id,
    ) -> Result<StockProduct, AppError> {
        let state = Arc::clone(&state);
        on_worker("stock_product", move || super::stock_product(&state, &id)).await
    }

    #[tauri::command]
    pub async fn stock_lookup(
        state: State<'_, Arc<AppState>>,
        code: String,
    ) -> Result<StockProduct, AppError> {
        let state = Arc::clone(&state);
        on_worker("stock_lookup", move || super::stock_lookup(&state, &code)).await
    }

    #[tauri::command]
    pub async fn stock_history(
        state: State<'_, Arc<AppState>>,
        query: HistoryQuery,
    ) -> Result<Vec<HistoryRow>, AppError> {
        let state = Arc::clone(&state);
        on_worker("stock_history", move || {
            super::stock_history(&state, &query)
        })
        .await
    }

    #[tauri::command]
    pub async fn stock_reasons(state: State<'_, Arc<AppState>>) -> Result<Vec<Reason>, AppError> {
        let state = Arc::clone(&state);
        on_worker("stock_reasons", move || super::stock_reasons(&state)).await
    }

    #[tauri::command]
    pub async fn stock_adjust(
        state: State<'_, Arc<AppState>>,
        input: AdjustInput,
    ) -> Result<Qty, AppError> {
        let state = Arc::clone(&state);
        on_worker("stock_adjust", move || super::stock_adjust(&state, &input)).await
    }

    #[tauri::command]
    pub async fn low_stock(
        state: State<'_, Arc<AppState>>,
        page: ProductPage,
    ) -> Result<Vec<LowStockView>, AppError> {
        let state = Arc::clone(&state);
        on_worker("low_stock", move || super::low_stock(&state, page)).await
    }

    #[tauri::command]
    pub async fn near_expiry(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<NearExpiryView>, AppError> {
        let state = Arc::clone(&state);
        on_worker("near_expiry", move || super::near_expiry(&state)).await
    }

    #[tauri::command]
    pub async fn stock_alerts(state: State<'_, Arc<AppState>>) -> Result<StockAlerts, AppError> {
        let state = Arc::clone(&state);
        on_worker("stock_alerts", move || super::stock_alerts(&state)).await
    }

    #[tauri::command]
    pub async fn stock_verify(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<MismatchView>, AppError> {
        let state = Arc::clone(&state);
        on_worker("stock_verify", move || super::stock_verify(&state)).await
    }

    #[tauri::command]
    pub async fn count_list(
        state: State<'_, Arc<AppState>>,
        page: ProductPage,
    ) -> Result<Vec<CountSummary>, AppError> {
        let state = Arc::clone(&state);
        on_worker("count_list", move || super::count_list(&state, page)).await
    }

    #[tauri::command]
    pub async fn count_get(state: State<'_, Arc<AppState>>, id: Id) -> Result<CountView, AppError> {
        let state = Arc::clone(&state);
        on_worker("count_get", move || super::count_get(&state, &id)).await
    }

    #[tauri::command]
    pub async fn count_create(
        state: State<'_, Arc<AppState>>,
        input: CountInput,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("count_create", move || super::count_create(&state, &input)).await
    }

    #[tauri::command]
    pub async fn count_line(
        state: State<'_, Arc<AppState>>,
        count_id: Id,
        line: CountLineInput,
    ) -> Result<CountView, AppError> {
        let state = Arc::clone(&state);
        on_worker("count_line", move || {
            super::count_line(&state, &count_id, &line)
        })
        .await
    }

    #[tauri::command]
    pub async fn count_variances(
        state: State<'_, Arc<AppState>>,
        id: Id,
    ) -> Result<VarianceReport, AppError> {
        let state = Arc::clone(&state);
        on_worker("count_variances", move || {
            super::count_variances(&state, &id)
        })
        .await
    }

    #[tauri::command]
    pub async fn count_post(
        state: State<'_, Arc<AppState>>,
        id: Id,
        confirm_uncounted: bool,
    ) -> Result<CountPosted, AppError> {
        let state = Arc::clone(&state);
        on_worker("count_post", move || {
            super::count_post(&state, &id, confirm_uncounted)
        })
        .await
    }

    #[tauri::command]
    pub async fn count_cancel(state: State<'_, Arc<AppState>>, id: Id) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("count_cancel", move || super::count_cancel(&state, &id)).await
    }
}
