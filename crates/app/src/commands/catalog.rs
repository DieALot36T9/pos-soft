//! The product screens (Req - Catalog). Reading needs `StockView`; every change needs
//! `CatalogManage`, and a change to a sale or cost price also needs `CatalogPriceEdit`.

use std::path::Path;

use catalog::{Brand, Category, ProductFilter, ProductList, ProductPage, SearchHit, Unit};
use pos_core::{AppError, Id, Permission};

use crate::session;
use crate::state::AppState;
use crate::workflows::catalog_lists::{self, BrandInput, CategoryInput, UnitInput};
use crate::workflows::products::{
    self, KitPartInput, ProductChoices, ProductDetails, ProductInput, VariantAttribute,
};

pub fn product_list(
    state: &AppState,
    filter: &ProductFilter,
    page: ProductPage,
) -> Result<ProductList, AppError> {
    session::require(state, Permission::StockView)?;
    products::list(state, filter, page)
}

pub fn product_search(
    state: &AppState,
    text: &str,
    limit: u32,
) -> Result<Vec<SearchHit>, AppError> {
    session::require(state, Permission::StockView)?;
    products::search(state, text, limit)
}

pub fn product_get(state: &AppState, id: &Id) -> Result<ProductDetails, AppError> {
    session::require(state, Permission::StockView)?;
    products::details(state, id)
}

pub fn product_choices(state: &AppState) -> Result<ProductChoices, AppError> {
    session::require(state, Permission::StockView)?;
    products::choices(state)
}

/// Saves a product. A price change also needs `CatalogPriceEdit` (Roles and Permissions).
pub fn product_save(state: &AppState, input: ProductInput) -> Result<Id, AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    if products::changes_prices(state, &input)? {
        session::require(state, Permission::CatalogPriceEdit)?;
    }
    products::save(state, &by.id, input)
}

pub fn product_archive(state: &AppState, id: &Id) -> Result<(), AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    products::archive(state, &by.id, id)
}

pub fn product_unarchive(state: &AppState, id: &Id) -> Result<(), AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    products::unarchive(state, &by.id, id)
}

pub fn product_delete(state: &AppState, id: &Id) -> Result<(), AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    products::delete(state, &by.id, id)
}

pub fn barcode_generate(state: &AppState, id: &Id) -> Result<String, AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    products::generate_barcode(state, &by.id, id)
}

pub fn variants_generate(
    state: &AppState,
    parent: &Id,
    attributes: &[VariantAttribute],
) -> Result<Vec<Id>, AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    products::generate_variants(state, &by.id, parent, attributes)
}

pub fn kit_save(state: &AppState, kit: &Id, parts: &[KitPartInput]) -> Result<(), AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    products::save_kit(state, &by.id, kit, parts)
}

pub fn product_image_set(state: &AppState, id: &Id, file: &str) -> Result<(), AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    products::set_image(state, &by.id, id, Path::new(file))
}

pub fn unit_list(state: &AppState, include_archived: bool) -> Result<Vec<Unit>, AppError> {
    session::require(state, Permission::StockView)?;
    catalog_lists::list_units(state, include_archived)
}

pub fn unit_save(state: &AppState, input: UnitInput) -> Result<Id, AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    catalog_lists::save_unit(state, &by.id, input)
}

pub fn unit_archive(state: &AppState, id: &Id) -> Result<(), AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    catalog_lists::archive_unit(state, &by.id, id)
}

pub fn category_list(state: &AppState, include_archived: bool) -> Result<Vec<Category>, AppError> {
    session::require(state, Permission::StockView)?;
    catalog_lists::list_categories(state, include_archived)
}

pub fn category_save(state: &AppState, input: CategoryInput) -> Result<Id, AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    catalog_lists::save_category(state, &by.id, input)
}

pub fn category_archive(state: &AppState, id: &Id) -> Result<(), AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    catalog_lists::archive_category(state, &by.id, id)
}

pub fn brand_list(state: &AppState, include_archived: bool) -> Result<Vec<Brand>, AppError> {
    session::require(state, Permission::StockView)?;
    catalog_lists::list_brands(state, include_archived)
}

pub fn brand_save(state: &AppState, input: BrandInput) -> Result<Id, AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    catalog_lists::save_brand(state, &by.id, input)
}

pub fn brand_archive(state: &AppState, id: &Id) -> Result<(), AppError> {
    let by = session::require(state, Permission::CatalogManage)?;
    catalog_lists::archive_brand(state, &by.id, id)
}

/// The Tauri commands. Each runs its handler above on a worker thread.
pub mod cmd {
    use std::sync::Arc;

    use catalog::{Brand, Category, ProductFilter, ProductList, ProductPage, SearchHit, Unit};
    use pos_core::{AppError, Id};
    use tauri::State;

    use crate::commands::on_worker;
    use crate::state::AppState;
    use crate::workflows::catalog_lists::{BrandInput, CategoryInput, UnitInput};
    use crate::workflows::products::{
        KitPartInput, ProductChoices, ProductDetails, ProductInput, VariantAttribute,
    };

    #[tauri::command]
    pub async fn product_list(
        state: State<'_, Arc<AppState>>,
        filter: ProductFilter,
        page: ProductPage,
    ) -> Result<ProductList, AppError> {
        let state = Arc::clone(&state);
        on_worker("product_list", move || {
            super::product_list(&state, &filter, page)
        })
        .await
    }

    #[tauri::command]
    pub async fn product_search(
        state: State<'_, Arc<AppState>>,
        text: String,
        limit: u32,
    ) -> Result<Vec<SearchHit>, AppError> {
        let state = Arc::clone(&state);
        on_worker("product_search", move || {
            super::product_search(&state, &text, limit)
        })
        .await
    }

    #[tauri::command]
    pub async fn product_get(
        state: State<'_, Arc<AppState>>,
        id: Id,
    ) -> Result<ProductDetails, AppError> {
        let state = Arc::clone(&state);
        on_worker("product_get", move || super::product_get(&state, &id)).await
    }

    #[tauri::command]
    pub async fn product_choices(
        state: State<'_, Arc<AppState>>,
    ) -> Result<ProductChoices, AppError> {
        let state = Arc::clone(&state);
        on_worker("product_choices", move || super::product_choices(&state)).await
    }

    #[tauri::command]
    pub async fn product_save(
        state: State<'_, Arc<AppState>>,
        input: ProductInput,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("product_save", move || super::product_save(&state, input)).await
    }

    #[tauri::command]
    pub async fn product_archive(state: State<'_, Arc<AppState>>, id: Id) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("product_archive", move || {
            super::product_archive(&state, &id)
        })
        .await
    }

    #[tauri::command]
    pub async fn product_unarchive(
        state: State<'_, Arc<AppState>>,
        id: Id,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("product_unarchive", move || {
            super::product_unarchive(&state, &id)
        })
        .await
    }

    #[tauri::command]
    pub async fn product_delete(state: State<'_, Arc<AppState>>, id: Id) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("product_delete", move || super::product_delete(&state, &id)).await
    }

    #[tauri::command]
    pub async fn barcode_generate(
        state: State<'_, Arc<AppState>>,
        id: Id,
    ) -> Result<String, AppError> {
        let state = Arc::clone(&state);
        on_worker("barcode_generate", move || {
            super::barcode_generate(&state, &id)
        })
        .await
    }

    #[tauri::command]
    pub async fn variants_generate(
        state: State<'_, Arc<AppState>>,
        parent: Id,
        attributes: Vec<VariantAttribute>,
    ) -> Result<Vec<Id>, AppError> {
        let state = Arc::clone(&state);
        on_worker("variants_generate", move || {
            super::variants_generate(&state, &parent, &attributes)
        })
        .await
    }

    #[tauri::command]
    pub async fn kit_save(
        state: State<'_, Arc<AppState>>,
        kit: Id,
        parts: Vec<KitPartInput>,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("kit_save", move || super::kit_save(&state, &kit, &parts)).await
    }

    #[tauri::command]
    pub async fn product_image_set(
        state: State<'_, Arc<AppState>>,
        id: Id,
        file: String,
    ) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("product_image_set", move || {
            super::product_image_set(&state, &id, &file)
        })
        .await
    }

    #[tauri::command]
    pub async fn unit_list(
        state: State<'_, Arc<AppState>>,
        include_archived: bool,
    ) -> Result<Vec<Unit>, AppError> {
        let state = Arc::clone(&state);
        on_worker("unit_list", move || {
            super::unit_list(&state, include_archived)
        })
        .await
    }

    #[tauri::command]
    pub async fn unit_save(
        state: State<'_, Arc<AppState>>,
        input: UnitInput,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("unit_save", move || super::unit_save(&state, input)).await
    }

    #[tauri::command]
    pub async fn unit_archive(state: State<'_, Arc<AppState>>, id: Id) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("unit_archive", move || super::unit_archive(&state, &id)).await
    }

    #[tauri::command]
    pub async fn category_list(
        state: State<'_, Arc<AppState>>,
        include_archived: bool,
    ) -> Result<Vec<Category>, AppError> {
        let state = Arc::clone(&state);
        on_worker("category_list", move || {
            super::category_list(&state, include_archived)
        })
        .await
    }

    #[tauri::command]
    pub async fn category_save(
        state: State<'_, Arc<AppState>>,
        input: CategoryInput,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("category_save", move || super::category_save(&state, input)).await
    }

    #[tauri::command]
    pub async fn category_archive(state: State<'_, Arc<AppState>>, id: Id) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("category_archive", move || {
            super::category_archive(&state, &id)
        })
        .await
    }

    #[tauri::command]
    pub async fn brand_list(
        state: State<'_, Arc<AppState>>,
        include_archived: bool,
    ) -> Result<Vec<Brand>, AppError> {
        let state = Arc::clone(&state);
        on_worker("brand_list", move || {
            super::brand_list(&state, include_archived)
        })
        .await
    }

    #[tauri::command]
    pub async fn brand_save(
        state: State<'_, Arc<AppState>>,
        input: BrandInput,
    ) -> Result<Id, AppError> {
        let state = Arc::clone(&state);
        on_worker("brand_save", move || super::brand_save(&state, input)).await
    }

    #[tauri::command]
    pub async fn brand_archive(state: State<'_, Arc<AppState>>, id: Id) -> Result<(), AppError> {
        let state = Arc::clone(&state);
        on_worker("brand_archive", move || super::brand_archive(&state, &id)).await
    }
}
