//! The purchasing screens (Req - Purchasing). Everything needs `PurchasingManage`, except
//! paying a supplier, which needs `PurchasingPay`. Each change is audited in its workflow.

use std::path::Path;

use chrono::NaiveDate;
use pos_core::{AppError, Id, Permission};
use purchasing::{Grn, GrnRow, NewOrderLine, OrderStatus, ReturnRef, ReturnRow, Statement};

use crate::devices::Devices;
use crate::session;
use crate::state::AppState;
use crate::workflows::purchasing_admin::{
    self, AgingView, OrderList, OrderSave, OrderView, SupplierList, SupplierPage,
    SupplierPaymentDone, SupplierPaymentInput, SupplierSave,
};
use crate::workflows::receiving::{
    self, GrnDone, GrnInput, GrnPreview, GrnStart, SupplierReturnInput,
};

pub fn supplier_list(state: &AppState, include_archived: bool) -> Result<SupplierList, AppError> {
    session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::suppliers(state, include_archived)
}

pub fn supplier_get(state: &AppState, id: &Id) -> Result<SupplierPage, AppError> {
    let user = session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::supplier_page(state, &user.id, id)
}

pub fn supplier_save(state: &AppState, input: &SupplierSave) -> Result<Id, AppError> {
    let user = session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::save_supplier(state, &user.id, input)
}

pub fn supplier_archive(state: &AppState, id: &Id) -> Result<(), AppError> {
    let user = session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::archive_supplier(state, &user.id, id)
}

pub fn po_list(state: &AppState, status: Option<OrderStatus>) -> Result<OrderList, AppError> {
    session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::orders(state, status)
}

pub fn po_get(state: &AppState, id: &Id) -> Result<OrderView, AppError> {
    session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::order(state, id)
}

pub fn po_save(state: &AppState, input: &OrderSave) -> Result<Id, AppError> {
    let user = session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::save_order(state, &user.id, input)
}

pub fn po_suggest(state: &AppState, supplier_id: &Id) -> Result<Vec<NewOrderLine>, AppError> {
    session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::suggest(state, supplier_id)
}

pub fn po_mark_sent(state: &AppState, id: &Id) -> Result<(), AppError> {
    let user = session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::mark_sent(state, &user.id, id)
}

pub fn po_export(state: &AppState, id: &Id, file: &str) -> Result<(), AppError> {
    let user = session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::export_order(state, &user.id, id, Path::new(file))
}

pub fn po_cancel(state: &AppState, id: &Id) -> Result<(), AppError> {
    let user = session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::cancel_order(state, &user.id, id)
}

pub fn grn_start(state: &AppState, order_id: &Id) -> Result<GrnStart, AppError> {
    session::require(state, Permission::PurchasingManage)?;
    receiving::start(state, order_id)
}

pub fn grn_preview(state: &AppState, input: &GrnInput) -> Result<GrnPreview, AppError> {
    session::require(state, Permission::PurchasingManage)?;
    receiving::preview(state, input)
}

pub fn grn_save(state: &AppState, input: &GrnInput) -> Result<GrnDone, AppError> {
    let user = session::require(state, Permission::PurchasingManage)?;
    receiving::save(state, &user.id, input)
}

pub fn grn_list(state: &AppState) -> Result<Vec<GrnRow>, AppError> {
    session::require(state, Permission::PurchasingManage)?;
    receiving::grns(state)
}

pub fn grn_get(state: &AppState, id: &Id) -> Result<Grn, AppError> {
    session::require(state, Permission::PurchasingManage)?;
    receiving::grn(state, id)
}

pub fn supplier_return_save(
    state: &AppState,
    input: &SupplierReturnInput,
) -> Result<ReturnRef, AppError> {
    let user = session::require(state, Permission::PurchasingManage)?;
    receiving::send_back(state, &user.id, input)
}

pub fn supplier_return_list(state: &AppState) -> Result<Vec<ReturnRow>, AppError> {
    session::require(state, Permission::PurchasingManage)?;
    receiving::returns(state)
}

pub fn payables_aging(state: &AppState) -> Result<AgingView, AppError> {
    session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::aging(state)
}

pub fn supplier_statement(
    state: &AppState,
    id: &Id,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Statement, AppError> {
    session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::statement(state, id, from, to)
}

pub fn supplier_statement_export(
    state: &AppState,
    id: &Id,
    from: NaiveDate,
    to: NaiveDate,
    file: &str,
) -> Result<u32, AppError> {
    session::require(state, Permission::PurchasingManage)?;
    purchasing_admin::export_statement(state, id, from, to, Path::new(file))
}

pub fn supplier_payment(
    state: &AppState,
    devices: &Devices,
    input: &SupplierPaymentInput,
) -> Result<SupplierPaymentDone, AppError> {
    let user = session::require(state, Permission::PurchasingPay)?;
    purchasing_admin::pay_supplier(state, devices, &user.id, input)
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use chrono::NaiveDate;
    use pos_core::{AppError, Id};
    use purchasing::{Grn, GrnRow, NewOrderLine, OrderStatus, ReturnRef, ReturnRow, Statement};
    use tauri::State;

    use crate::commands::on_worker;
    use crate::devices::Devices;
    use crate::state::AppState;
    use crate::workflows::purchasing_admin::{
        AgingView, OrderList, OrderSave, OrderView, SupplierList, SupplierPage,
        SupplierPaymentDone, SupplierPaymentInput, SupplierSave,
    };
    use crate::workflows::receiving::{
        GrnDone, GrnInput, GrnPreview, GrnStart, SupplierReturnInput,
    };

    #[tauri::command]
    pub async fn supplier_list(
        state: State<'_, Arc<AppState>>,
        include_archived: bool,
    ) -> Result<SupplierList, AppError> {
        let state = Arc::clone(&state);
        on_worker("supplier_list", move || {
            super::supplier_list(&state, include_archived)
        })
        .await
    }

    #[tauri::command]
    pub async fn supplier_get(
        state: State<'_, Arc<AppState>>,
        id: Id,
    ) -> Result<SupplierPage, AppError> {
        let state = Arc::clone(&state);
        on_worker("supplier_get", move || super::supplier_get(&state, &id)).await
    }

    #[tauri::command]
    pub async fn supplier_save(
        state: State<'_, Arc<AppState>>,
        input: SupplierSave,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("supplier_save", move || {
            super::supplier_save(&state, &input)
        })
        .await
    }

    #[tauri::command]
    pub async fn supplier_archive(state: State<'_, Arc<AppState>>, id: Id) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("supplier_archive", move || {
            super::supplier_archive(&state, &id)
        })
        .await
    }

    #[tauri::command]
    pub async fn po_list(
        state: State<'_, Arc<AppState>>,
        status: Option<OrderStatus>,
    ) -> Result<OrderList, AppError> {
        let state = Arc::clone(&state);
        on_worker("po_list", move || super::po_list(&state, status)).await
    }

    #[tauri::command]
    pub async fn po_get(state: State<'_, Arc<AppState>>, id: Id) -> Result<OrderView, AppError> {
        let state = Arc::clone(&state);
        on_worker("po_get", move || super::po_get(&state, &id)).await
    }

    #[tauri::command]
    pub async fn po_save(
        state: State<'_, Arc<AppState>>,
        input: OrderSave,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("po_save", move || super::po_save(&state, &input)).await
    }

    #[tauri::command]
    pub async fn po_suggest(
        state: State<'_, Arc<AppState>>,
        supplier_id: Id,
    ) -> Result<Vec<NewOrderLine>, AppError> {
        let state = Arc::clone(&state);
        on_worker("po_suggest", move || {
            super::po_suggest(&state, &supplier_id)
        })
        .await
    }

    #[tauri::command]
    pub async fn po_mark_sent(state: State<'_, Arc<AppState>>, id: Id) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("po_mark_sent", move || super::po_mark_sent(&state, &id)).await
    }

    #[tauri::command]
    pub async fn po_export(
        state: State<'_, Arc<AppState>>,
        id: Id,
        file: String,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("po_export", move || super::po_export(&state, &id, &file)).await
    }

    #[tauri::command]
    pub async fn po_cancel(state: State<'_, Arc<AppState>>, id: Id) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("po_cancel", move || super::po_cancel(&state, &id)).await
    }

    #[tauri::command]
    pub async fn grn_start(
        state: State<'_, Arc<AppState>>,
        order_id: Id,
    ) -> Result<GrnStart, AppError> {
        let state = Arc::clone(&state);
        on_worker("grn_start", move || super::grn_start(&state, &order_id)).await
    }

    #[tauri::command]
    pub async fn grn_preview(
        state: State<'_, Arc<AppState>>,
        input: GrnInput,
    ) -> Result<GrnPreview, AppError> {
        let state = Arc::clone(&state);
        on_worker("grn_preview", move || super::grn_preview(&state, &input)).await
    }

    #[tauri::command]
    pub async fn grn_save(
        state: State<'_, Arc<AppState>>,
        input: GrnInput,
    ) -> Result<GrnDone, AppError> {
        let state = Arc::clone(&state);
        on_worker("grn_save", move || super::grn_save(&state, &input)).await
    }

    #[tauri::command]
    pub async fn grn_list(state: State<'_, Arc<AppState>>) -> Result<Vec<GrnRow>, AppError> {
        let state = Arc::clone(&state);
        on_worker("grn_list", move || super::grn_list(&state)).await
    }

    #[tauri::command]
    pub async fn grn_get(state: State<'_, Arc<AppState>>, id: Id) -> Result<Grn, AppError> {
        let state = Arc::clone(&state);
        on_worker("grn_get", move || super::grn_get(&state, &id)).await
    }

    #[tauri::command]
    pub async fn supplier_return_save(
        state: State<'_, Arc<AppState>>,
        input: SupplierReturnInput,
    ) -> Result<ReturnRef, AppError> {
        let state = Arc::clone(&state);
        on_worker("supplier_return_save", move || {
            super::supplier_return_save(&state, &input)
        })
        .await
    }

    #[tauri::command]
    pub async fn supplier_return_list(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<ReturnRow>, AppError> {
        let state = Arc::clone(&state);
        on_worker("supplier_return_list", move || {
            super::supplier_return_list(&state)
        })
        .await
    }

    #[tauri::command]
    pub async fn payables_aging(state: State<'_, Arc<AppState>>) -> Result<AgingView, AppError> {
        let state = Arc::clone(&state);
        on_worker("payables_aging", move || super::payables_aging(&state)).await
    }

    #[tauri::command]
    pub async fn supplier_statement(
        state: State<'_, Arc<AppState>>,
        id: Id,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Statement, AppError> {
        let state = Arc::clone(&state);
        on_worker("supplier_statement", move || {
            super::supplier_statement(&state, &id, from, to)
        })
        .await
    }

    #[tauri::command]
    pub async fn supplier_statement_export(
        state: State<'_, Arc<AppState>>,
        id: Id,
        from: NaiveDate,
        to: NaiveDate,
        file: String,
    ) -> Result<u32, AppError> {
        let state = Arc::clone(&state);
        on_worker("supplier_statement_export", move || {
            super::supplier_statement_export(&state, &id, from, to, &file)
        })
        .await
    }

    #[tauri::command]
    pub async fn supplier_payment(
        state: State<'_, Arc<AppState>>,
        devices: State<'_, Arc<Devices>>,
        input: SupplierPaymentInput,
    ) -> Result<SupplierPaymentDone, AppError> {
        let state = Arc::clone(&state);
        let devices = Arc::clone(&devices);
        on_worker("supplier_payment", move || {
            super::supplier_payment(&state, &devices, &input)
        })
        .await
    }
}
