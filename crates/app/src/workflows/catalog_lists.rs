//! Units, categories and brands (CAT-02, CAT-03): add, change and archive, each in
//! one transaction with its audit event. They are never deleted.

use catalog::{Brand, Category, CategoryFields, Unit, UnitFields};
use pos_core::{AppError, Id};
use serde::Deserialize;
use ts_rs::TS;

use crate::audit_actions::{BRAND_CHANGE, CATEGORY_CHANGE, UNIT_CHANGE};
use crate::audit_log;
use crate::state::AppState;

/// A unit as typed on the Units screen. Without `id` it is a new unit.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct UnitInput {
    pub id: Option<Id>,
    pub fields: UnitFields,
}

/// A category as typed on the Categories screen. Without `id` it is a new category.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct CategoryInput {
    pub id: Option<Id>,
    pub fields: CategoryFields,
}

/// A brand as typed on the Brands screen. Without `id` it is a new brand.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct BrandInput {
    pub id: Option<Id>,
    pub name: String,
}

pub fn list_units(state: &AppState, include_archived: bool) -> Result<Vec<Unit>, AppError> {
    state
        .db
        .read(|conn| Ok(catalog::list_units(conn, include_archived)?))
}

pub fn list_categories(
    state: &AppState,
    include_archived: bool,
) -> Result<Vec<Category>, AppError> {
    state
        .db
        .read(|conn| Ok(catalog::list_categories(conn, include_archived)?))
}

pub fn list_brands(state: &AppState, include_archived: bool) -> Result<Vec<Brand>, AppError> {
    state
        .db
        .read(|conn| Ok(catalog::list_brands(conn, include_archived)?))
}

/// Adds a unit or changes one (CAT-02). Returns its id.
pub fn save_unit(state: &AppState, by: &Id, input: UnitInput) -> Result<Id, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let id = match &input.id {
            Some(id) => {
                catalog::update_unit(tx, id, &input.fields)?;
                id.clone()
            }
            None => catalog::create_unit(tx, &input.fields)?,
        };
        let details = serde_json::json!({
            "unit_id": id,
            "name": input.fields.name,
            "symbol": input.fields.symbol,
            "decimals": input.fields.decimals,
            "change": if input.id.is_some() { "update" } else { "create" },
        });
        audit_log::record(tx, clock, UNIT_CHANGE, details, Some(by.clone()))?;
        Ok(id)
    })
}

/// Archives a unit. Refused for built-in units and units an active product uses.
pub fn archive_unit(state: &AppState, by: &Id, id: &Id) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        catalog::archive_unit(tx, id, clock.now())?;
        let details = serde_json::json!({ "unit_id": id, "change": "archive" });
        audit_log::record(tx, clock, UNIT_CHANGE, details, Some(by.clone()))
    })
}

/// Adds a category or changes one (CAT-03). Returns its id.
pub fn save_category(state: &AppState, by: &Id, input: CategoryInput) -> Result<Id, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let id = match &input.id {
            Some(id) => {
                catalog::update_category(tx, id, &input.fields)?;
                id.clone()
            }
            None => catalog::create_category(tx, &input.fields)?,
        };
        let details = serde_json::json!({
            "category_id": id,
            "name": input.fields.name,
            "parent_id": input.fields.parent_id,
            "change": if input.id.is_some() { "update" } else { "create" },
        });
        audit_log::record(tx, clock, CATEGORY_CHANGE, details, Some(by.clone()))?;
        Ok(id)
    })
}

/// Archives a category. Refused while it has active products or sub-categories.
pub fn archive_category(state: &AppState, by: &Id, id: &Id) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        catalog::archive_category(tx, id, clock.now())?;
        let details = serde_json::json!({ "category_id": id, "change": "archive" });
        audit_log::record(tx, clock, CATEGORY_CHANGE, details, Some(by.clone()))
    })
}

/// Adds a brand or renames one (CAT-03). Returns its id.
pub fn save_brand(state: &AppState, by: &Id, input: BrandInput) -> Result<Id, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let id = match &input.id {
            Some(id) => {
                catalog::rename_brand(tx, id, &input.name)?;
                id.clone()
            }
            None => catalog::create_brand(tx, &input.name)?,
        };
        let details = serde_json::json!({
            "brand_id": id,
            "name": input.name,
            "change": if input.id.is_some() { "update" } else { "create" },
        });
        audit_log::record(tx, clock, BRAND_CHANGE, details, Some(by.clone()))?;
        Ok(id)
    })
}

/// Archives a brand. Its products keep it, but it can't be picked for new ones.
pub fn archive_brand(state: &AppState, by: &Id, id: &Id) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        catalog::archive_brand(tx, id, clock.now())?;
        let details = serde_json::json!({ "brand_id": id, "change": "archive" });
        audit_log::record(tx, clock, BRAND_CHANGE, details, Some(by.clone()))
    })
}
