//! The pricing screens (Req - Pricing and Promotions): price lists, tiers and
//! promotions. Everything needs `PricingManage`.

use pos_core::{AppError, Id, Permission};
use pricing::{PriceList, PriceListItem, Promotion, Tier};

use crate::session;
use crate::state::AppState;
use crate::workflows::pricing_admin::{
    self, PriceListItemRow, PriceListSave, PromotionSave, TierSave,
};

pub fn price_list_list(state: &AppState) -> Result<Vec<PriceList>, AppError> {
    session::require(state, Permission::PricingManage)?;
    pricing_admin::price_lists(state)
}

pub fn price_list_save(state: &AppState, input: &PriceListSave) -> Result<Id, AppError> {
    let user = session::require(state, Permission::PricingManage)?;
    pricing_admin::save_price_list(state, &user.id, input)
}

pub fn price_list_items(state: &AppState, id: &Id) -> Result<Vec<PriceListItemRow>, AppError> {
    session::require(state, Permission::PricingManage)?;
    pricing_admin::price_list_items(state, id)
}

pub fn price_list_set_items(
    state: &AppState,
    id: &Id,
    items: &[PriceListItem],
) -> Result<(), AppError> {
    let user = session::require(state, Permission::PricingManage)?;
    pricing_admin::set_price_list_items(state, &user.id, id, items)
}

pub fn price_list_remove_items(
    state: &AppState,
    id: &Id,
    product_ids: &[Id],
) -> Result<(), AppError> {
    let user = session::require(state, Permission::PricingManage)?;
    pricing_admin::remove_price_list_items(state, &user.id, id, product_ids)
}

pub fn tier_list(state: &AppState) -> Result<Vec<Tier>, AppError> {
    session::require(state, Permission::PricingManage)?;
    pricing_admin::tiers(state)
}

pub fn tier_save(state: &AppState, input: &TierSave) -> Result<Id, AppError> {
    let user = session::require(state, Permission::PricingManage)?;
    pricing_admin::save_tier(state, &user.id, input)
}

pub fn tier_archive(state: &AppState, id: &Id) -> Result<(), AppError> {
    let user = session::require(state, Permission::PricingManage)?;
    pricing_admin::archive_tier(state, &user.id, id)
}

pub fn promotion_list(state: &AppState) -> Result<Vec<Promotion>, AppError> {
    session::require(state, Permission::PricingManage)?;
    pricing_admin::promotions(state)
}

pub fn promotion_save(state: &AppState, input: &PromotionSave) -> Result<Id, AppError> {
    let user = session::require(state, Permission::PricingManage)?;
    pricing_admin::save_promotion(state, &user.id, input)
}

pub fn promotion_set_active(state: &AppState, id: &Id, active: bool) -> Result<(), AppError> {
    let user = session::require(state, Permission::PricingManage)?;
    pricing_admin::set_promotion_active(state, &user.id, id, active)
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use pos_core::{AppError, Id};
    use pricing::{PriceList, PriceListItem, Promotion, Tier};
    use tauri::State;

    use crate::commands::on_worker;
    use crate::state::AppState;
    use crate::workflows::pricing_admin::{
        PriceListItemRow, PriceListSave, PromotionSave, TierSave,
    };

    #[tauri::command]
    pub async fn price_list_list(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<PriceList>, AppError> {
        let state = Arc::clone(&state);
        on_worker("price_list_list", move || super::price_list_list(&state)).await
    }

    #[tauri::command]
    pub async fn price_list_save(
        state: State<'_, Arc<AppState>>,
        input: PriceListSave,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("price_list_save", move || {
            super::price_list_save(&state, &input)
        })
        .await
    }

    #[tauri::command]
    pub async fn price_list_items(
        state: State<'_, Arc<AppState>>,
        id: Id,
    ) -> Result<Vec<PriceListItemRow>, AppError> {
        let state = Arc::clone(&state);
        on_worker("price_list_items", move || {
            super::price_list_items(&state, &id)
        })
        .await
    }

    #[tauri::command]
    pub async fn price_list_set_items(
        state: State<'_, Arc<AppState>>,
        id: Id,
        items: Vec<PriceListItem>,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("price_list_set_items", move || {
            super::price_list_set_items(&state, &id, &items)
        })
        .await
    }

    #[tauri::command]
    pub async fn price_list_remove_items(
        state: State<'_, Arc<AppState>>,
        id: Id,
        product_ids: Vec<Id>,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("price_list_remove_items", move || {
            super::price_list_remove_items(&state, &id, &product_ids)
        })
        .await
    }

    #[tauri::command]
    pub async fn tier_list(state: State<'_, Arc<AppState>>) -> Result<Vec<Tier>, AppError> {
        let state = Arc::clone(&state);
        on_worker("tier_list", move || super::tier_list(&state)).await
    }

    #[tauri::command]
    pub async fn tier_save(
        state: State<'_, Arc<AppState>>,
        input: TierSave,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("tier_save", move || super::tier_save(&state, &input)).await
    }

    #[tauri::command]
    pub async fn tier_archive(state: State<'_, Arc<AppState>>, id: Id) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("tier_archive", move || super::tier_archive(&state, &id)).await
    }

    #[tauri::command]
    pub async fn promotion_list(
        state: State<'_, Arc<AppState>>,
    ) -> Result<Vec<Promotion>, AppError> {
        let state = Arc::clone(&state);
        on_worker("promotion_list", move || super::promotion_list(&state)).await
    }

    #[tauri::command]
    pub async fn promotion_save(
        state: State<'_, Arc<AppState>>,
        input: PromotionSave,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("promotion_save", move || {
            super::promotion_save(&state, &input)
        })
        .await
    }

    #[tauri::command]
    pub async fn promotion_set_active(
        state: State<'_, Arc<AppState>>,
        id: Id,
        active: bool,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("promotion_set_active", move || {
            super::promotion_set_active(&state, &id, active)
        })
        .await
    }
}
