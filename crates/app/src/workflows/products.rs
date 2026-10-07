//! Products on the back office (Req - Catalog, CAT-01 to CAT-11): the product list,
//! the product page, and every change to a product. Each change is one transaction
//! with its audit event ("Save product" in Workflows).

use std::collections::HashMap;
use std::path::Path;

use base64::Engine;
use catalog::{
    Brand, CatalogError, Category, NewProduct, PriceField, Product, ProductFilter, ProductKind,
    ProductList, ProductPage, ProductRow, ProductUpdate, SearchHit, UNCATEGORISED, Unit,
};
use pos_core::{AppError, CurrencyFormat, Id, Money, Qty};
use rusqlite::{Connection, Transaction};
use serde::{Deserialize, Serialize};
use settings::{InternalBarcode, TaxSettings};
use ts_rs::TS;

use crate::audit_actions::{
    PRODUCT_ARCHIVE, PRODUCT_CHANGE, PRODUCT_CREATE, PRODUCT_DELETE, PRODUCT_UNARCHIVE,
};
use crate::audit_log;
use crate::state::AppState;

/// The biggest page the product list returns at once.
const MAX_PAGE: u32 = 500;
/// The most search results the screen asks for.
const MAX_HITS: u32 = 100;

/// A product as typed on the product form. Without `id` it is a new product.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct ProductInput {
    pub id: Option<Id>,
    /// Used only for a new product. Variants are made with "Generate variants".
    pub kind: ProductKind,
    pub name: String,
    /// Left empty on a new product, the catalog makes one: "P000001".
    pub sku: String,
    /// Left empty, the product goes in "Uncategorised".
    pub category_id: Option<Id>,
    pub brand_id: Option<Id>,
    pub unit_id: Id,
    pub sale_price: Money,
    pub cost_price: Money,
    /// Left empty, the shop's default tax rate.
    pub tax_rate_id: Option<Id>,
    pub is_weighed: bool,
    pub scale_item_code: Option<String>,
    pub track_stock: bool,
    pub track_expiry: bool,
    pub preferred_supplier_id: Option<Id>,
    pub description: Option<String>,
    /// Every barcode the product should have, primary first. Codes not listed are removed.
    pub barcodes: Vec<String>,
    /// When to order more (INV-03). Empty means no reorder rule.
    pub reorder: Option<ReorderInput>,
}

/// "Order `qty` when stock falls below `level`" (INV-03), kept by inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReorderInput {
    pub level: Qty,
    pub qty: Qty,
}

/// One part of a kit, as shown on the Kit tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct KitPart {
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub unit_symbol: String,
    pub qty: Qty,
}

/// One part of a kit, as typed on the Kit tab.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct KitPartInput {
    pub product_id: Id,
    pub qty: Qty,
}

/// One attribute for "Generate variants". Example: Size with 200 ml and 400 ml.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct VariantAttribute {
    pub name: String,
    pub values: Vec<String>,
}

/// One row of the price history tab (CAT-09), with the name of who changed it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PriceHistoryView {
    pub field: PriceField,
    pub old: Money,
    pub new: Money,
    pub changed_at: pos_core::Timestamp,
    pub changed_by_name: Option<String>,
    /// "manual", "grn" or "import".
    pub reason: Option<String>,
}

/// Everything the product page shows about one product.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ProductDetails {
    pub product: Product,
    pub reorder: Option<ReorderInput>,
    pub on_hand: Qty,
    /// For a kit: its parts. Empty for other products.
    pub kit_parts: Vec<KitPart>,
    /// For a variant parent: its variants, archived ones included.
    pub variants: Vec<ProductRow>,
    /// Oldest first.
    pub price_history: Vec<PriceHistoryView>,
    /// The 512 px picture as a `data:image/jpeg;base64,…` address, if the product has one.
    pub image: Option<String>,
}

/// A name to pick from a list. Example: a tax rate or a supplier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Choice {
    pub id: Id,
    pub name: String,
}

/// The lists the product form picks from. Only active entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct ProductChoices {
    pub units: Vec<Unit>,
    pub categories: Vec<Category>,
    pub brands: Vec<Brand>,
    pub tax_rates: Vec<Choice>,
    pub suppliers: Vec<Choice>,
    pub default_tax_rate_id: Option<Id>,
    /// How the shop writes money, for the price fields.
    pub currency: CurrencyFormat,
}

/// One page of the product list (CAT-08: archived products only when asked).
pub fn list(
    state: &AppState,
    filter: &ProductFilter,
    page: ProductPage,
) -> Result<ProductList, AppError> {
    let page = ProductPage {
        offset: page.offset,
        limit: page.limit.clamp(1, MAX_PAGE),
    };
    state.db.read(|conn| Ok(catalog::list(conn, filter, page)?))
}

/// Search by name, SKU, barcode, category or brand; exact barcode or SKU first (CAT-05).
pub fn search(state: &AppState, text: &str, limit: u32) -> Result<Vec<SearchHit>, AppError> {
    state
        .db
        .read(|conn| Ok(catalog::search(conn, text, limit.clamp(1, MAX_HITS))?))
}

/// The lists the product form needs: units, categories, brands, tax rates, suppliers.
pub fn choices(state: &AppState) -> Result<ProductChoices, AppError> {
    state.db.read(|conn| {
        let tax: TaxSettings = settings::get(conn)?;
        let tax_rates = tax::list(conn, false)?
            .into_iter()
            .map(|rate| Choice {
                id: rate.id,
                name: rate.name,
            })
            .collect();
        let suppliers = purchasing::list_suppliers(conn, false)?
            .into_iter()
            .map(|supplier| Choice {
                id: supplier.id,
                name: supplier.name,
            })
            .collect();
        Ok(ProductChoices {
            units: catalog::list_units(conn, false)?,
            categories: catalog::list_categories(conn, false)?,
            brands: catalog::list_brands(conn, false)?,
            tax_rates,
            suppliers,
            default_tax_rate_id: tax.default_rate_id,
            currency: settings::get(conn)?,
        })
    })
}

/// Everything about one product for the product page.
pub fn details(state: &AppState, id: &Id) -> Result<ProductDetails, AppError> {
    let images = state.paths.images();
    state.db.read(|conn| {
        let product = catalog::get(conn, id)?;
        let reorder = inventory::reorder_rule(conn, id)?.map(|rule| ReorderInput {
            level: rule.level,
            qty: rule.qty,
        });
        let kit_parts = if product.kind == ProductKind::Kit {
            kit_parts(conn, id)?
        } else {
            Vec::new()
        };
        let variants = if product.kind == ProductKind::VariantParent {
            let filter = ProductFilter {
                parent_id: Some(id.clone()),
                include_archived: true,
                ..ProductFilter::default()
            };
            let all = ProductPage {
                offset: 0,
                limit: u32::MAX,
            };
            catalog::list(conn, &filter, all)?.rows
        } else {
            Vec::new()
        };
        let image = match &product.image_path {
            Some(file) => image_address(&images.join(file)),
            None => None,
        };
        Ok(ProductDetails {
            on_hand: inventory::on_hand(conn, id)?,
            price_history: price_history(conn, id)?,
            product,
            reorder,
            kit_parts,
            variants,
            image,
        })
    })
}

/// True when saving this input would change a price, so `CatalogPriceEdit` is needed too.
/// A new product with any price above zero counts as a change.
pub fn changes_prices(state: &AppState, input: &ProductInput) -> Result<bool, AppError> {
    match &input.id {
        None => Ok(input.sale_price.0 != 0 || input.cost_price.0 != 0),
        Some(id) => state.db.read(|conn| {
            let product = catalog::get(conn, id)?;
            Ok(product.sale_price != input.sale_price || product.cost_price != input.cost_price)
        }),
    }
}

/// Saves a product: catalog fields and barcodes, the reorder rule, price history (by
/// the catalog) and the audit event, in one transaction. Returns the product's id.
pub fn save(state: &AppState, by: &Id, input: ProductInput) -> Result<Id, AppError> {
    let clock = state.clock.as_ref();
    let now = clock.now();
    state.db.write(|tx| {
        let tax_rate_id = tax_rate_for(tx, input.tax_rate_id.as_ref())?;
        if let Some(supplier) = &input.preferred_supplier_id {
            purchasing::get_supplier(tx, supplier)?;
        }
        let barcodes = clean_barcodes(&input.barcodes)?;
        let (id, action, details) = match &input.id {
            None => {
                let new = NewProduct {
                    kind: input.kind,
                    name: input.name.clone(),
                    sku: blank_to_none(Some(&input.sku)),
                    category_id: input.category_id.clone(),
                    brand_id: input.brand_id.clone(),
                    unit_id: input.unit_id.clone(),
                    sale_price: input.sale_price,
                    cost_price: input.cost_price,
                    tax_rate_id,
                    is_weighed: input.is_weighed,
                    scale_item_code: blank_to_none(input.scale_item_code.as_deref()),
                    track_stock: input.track_stock,
                    track_expiry: input.track_expiry,
                    preferred_supplier_id: input.preferred_supplier_id.clone(),
                    description: blank_to_none(input.description.as_deref()),
                    barcodes,
                };
                let id = catalog::create_product(tx, new, now)?;
                let product = catalog::get(tx, &id)?;
                let details = serde_json::json!({
                    "product_id": id,
                    "name": product.name,
                    "sku": product.sku,
                    "sale_price": product.sale_price,
                    "cost_price": product.cost_price,
                });
                (id, PRODUCT_CREATE, details)
            }
            Some(id) => {
                let update = ProductUpdate {
                    name: input.name.clone(),
                    sku: input.sku.trim().to_string(),
                    category_id: match &input.category_id {
                        Some(category) => category.clone(),
                        None => Id::parse(UNCATEGORISED)?,
                    },
                    brand_id: input.brand_id.clone(),
                    unit_id: input.unit_id.clone(),
                    sale_price: input.sale_price,
                    cost_price: input.cost_price,
                    tax_rate_id,
                    is_weighed: input.is_weighed,
                    scale_item_code: blank_to_none(input.scale_item_code.as_deref()),
                    track_stock: input.track_stock,
                    track_expiry: input.track_expiry,
                    preferred_supplier_id: input.preferred_supplier_id.clone(),
                    description: blank_to_none(input.description.as_deref()),
                };
                let changes = catalog::update_product(tx, id, update, by, now)?;
                replace_barcodes(tx, id, &barcodes, now)?;
                let product = catalog::get(tx, id)?;
                let details = serde_json::json!({
                    "product_id": id,
                    "name": product.name,
                    "sku": product.sku,
                    "sale_price": changes.sale_price,
                    "cost_price": changes.cost_price,
                });
                (id.clone(), PRODUCT_CHANGE, details)
            }
        };
        let product = catalog::get(tx, &id)?;
        match input.reorder {
            Some(rule) if product.track_stock => {
                inventory::set_reorder_rule(tx, &id, rule.level, rule.qty)?;
            }
            _ => inventory::remove_reorder_rule(tx, &id)?,
        }
        audit_log::record(tx, clock, action, details, Some(by.clone()))?;
        Ok(id)
    })
}

/// Hides a product from sale and search; history and reports keep it (CAT-08).
pub fn archive(state: &AppState, by: &Id, id: &Id) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        catalog::archive_product(tx, id, clock.now())?;
        record_product_event(tx, state, by, id, PRODUCT_ARCHIVE)
    })
}

/// Brings an archived product back into sale and search.
pub fn unarchive(state: &AppState, by: &Id, id: &Id) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        catalog::unarchive_product(tx, id, clock.now())?;
        record_product_event(tx, state, by, id, PRODUCT_UNARCHIVE)
    })
}

/// Deletes a product for good. Refused once it was sold, moved in stock or put on a
/// purchasing document (CAT-08): those products are archived instead.
pub fn delete(state: &AppState, by: &Id, id: &Id) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let product = catalog::get(tx, id)?;
        let used = sales::is_product_used(tx, id)?
            || inventory::is_product_used(tx, id)?
            || purchasing::is_product_used(tx, id)?;
        catalog::delete_unused_product(tx, id, used)?;
        inventory::remove_reorder_rule(tx, id)?;
        let details = serde_json::json!({
            "product_id": id,
            "name": product.name,
            "sku": product.sku,
        });
        audit_log::record(tx, clock, PRODUCT_DELETE, details, Some(by.clone()))
    })
}

/// Gives the product a new internal EAN-13 from the shop's prefix (CAT-11) and returns it.
pub fn generate_barcode(state: &AppState, by: &Id, id: &Id) -> Result<String, AppError> {
    let clock = state.clock.as_ref();
    let now = clock.now();
    state.db.write(|tx| {
        let product = catalog::get(tx, id)?;
        let internal: InternalBarcode = settings::get(tx)?;
        // A number already typed by hand on another product is skipped.
        let code = loop {
            let number = catalog::next_internal_number(tx)?;
            let code = barcode::internal_ean13(&internal.prefix, number)?;
            match catalog::add_barcode(tx, id, &code, now) {
                Ok(()) => break code,
                Err(CatalogError::BarcodeInUse { .. }) => continue,
                Err(other) => return Err(other.into()),
            }
        };
        let details = serde_json::json!({
            "product_id": id,
            "name": product.name,
            "barcode_added": code,
        });
        audit_log::record(tx, clock, PRODUCT_CHANGE, details, Some(by.clone()))?;
        Ok(code)
    })
}

/// Makes a variant for every combination of the attributes (CAT-06). Combinations that
/// already exist are skipped. Returns the new variants' ids.
pub fn generate_variants(
    state: &AppState,
    by: &Id,
    parent: &Id,
    attributes: &[VariantAttribute],
) -> Result<Vec<Id>, AppError> {
    let clock = state.clock.as_ref();
    let now = clock.now();
    let attributes: Vec<(String, Vec<String>)> = attributes
        .iter()
        .map(|attribute| {
            let values = attribute
                .values
                .iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect();
            (attribute.name.trim().to_string(), values)
        })
        .collect();
    state.db.write(|tx| {
        let product = catalog::get(tx, parent)?;
        let made = catalog::generate_variants(tx, parent, &attributes, now)?;
        let details = serde_json::json!({
            "product_id": parent,
            "name": product.name,
            "variants_added": made.len(),
        });
        audit_log::record(tx, clock, PRODUCT_CHANGE, details, Some(by.clone()))?;
        Ok(made)
    })
}

/// Replaces a kit's parts and their quantities (CAT-07).
pub fn save_kit(
    state: &AppState,
    by: &Id,
    kit: &Id,
    parts: &[KitPartInput],
) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    let parts: Vec<(Id, Qty)> = parts
        .iter()
        .map(|part| (part.product_id.clone(), part.qty))
        .collect();
    state.db.write(|tx| {
        let product = catalog::get(tx, kit)?;
        catalog::set_kit_components(tx, kit, &parts)?;
        let details = serde_json::json!({
            "product_id": kit,
            "name": product.name,
            "kit_parts": parts.len(),
        });
        audit_log::record(tx, clock, PRODUCT_CHANGE, details, Some(by.clone()))
    })
}

/// Sets the product's picture from a PNG, JPEG or WebP file (CAT-10): a 512 px copy and
/// a 96 px thumbnail go in the app's images folder.
pub fn set_image(state: &AppState, by: &Id, id: &Id, source: &Path) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    let images = state.paths.images();
    std::fs::create_dir_all(&images).map_err(|error| {
        AppError::internal(format!("could not make the images folder: {error}"))
    })?;
    state.db.write(|tx| {
        let product = catalog::get(tx, id)?;
        catalog::set_image(tx, id, source, &images)?;
        let details = serde_json::json!({
            "product_id": id,
            "name": product.name,
            "image": "changed",
        });
        audit_log::record(tx, clock, PRODUCT_CHANGE, details, Some(by.clone()))
    })
}

/// The tax rate to save: the one picked, or the shop's default. It must exist and be active.
fn tax_rate_for(tx: &Transaction, picked: Option<&Id>) -> Result<Id, AppError> {
    let id = match picked {
        Some(id) => id.clone(),
        None => {
            let tax: TaxSettings = settings::get(tx)?;
            tax.default_rate_id
                .ok_or_else(|| field_error("tax_rate_id", "tax_rate_missing"))?
        }
    };
    let rate = tax::get(tx, &id)?;
    if !rate.active {
        return Err(field_error("tax_rate_id", "tax_rate_inactive"));
    }
    Ok(id)
}

/// Trims the barcodes, drops empty ones, and checks the check digit of codes that look
/// like an EAN-8, UPC-A or EAN-13 (all digits, 8, 12 or 13 long). Other codes, such as
/// Code 128, are kept as typed (CAT-04).
pub(crate) fn clean_barcodes(codes: &[String]) -> Result<Vec<String>, AppError> {
    let mut clean: Vec<String> = Vec::new();
    for code in codes {
        let code = code.trim();
        if code.is_empty() || clean.iter().any(|seen| seen == code) {
            continue;
        }
        let looks_like_gtin =
            matches!(code.len(), 8 | 12 | 13) && code.bytes().all(|byte| byte.is_ascii_digit());
        if looks_like_gtin && !barcode::is_valid_gtin(code) {
            return Err(AppError::Validation {
                field: Some("barcodes".to_string()),
                code: "barcode_check_digit_wrong".to_string(),
                params: [("code".to_string(), code.to_string())].into(),
            });
        }
        clean.push(code.to_string());
    }
    Ok(clean)
}

/// Makes the product's barcodes exactly `wanted`: removes the others, adds the new ones.
pub(crate) fn replace_barcodes(
    tx: &Transaction,
    id: &Id,
    wanted: &[String],
    now: pos_core::Timestamp,
) -> Result<(), AppError> {
    let current = catalog::get(tx, id)?.barcodes;
    for code in current.iter().filter(|code| !wanted.contains(code)) {
        catalog::remove_barcode(tx, code)?;
    }
    for code in wanted.iter().filter(|code| !current.contains(code)) {
        catalog::add_barcode(tx, id, code, now)?;
    }
    Ok(())
}

fn kit_parts(conn: &Connection, kit: &Id) -> Result<Vec<KitPart>, AppError> {
    let mut parts = Vec::new();
    for (product_id, qty) in catalog::kit_components(conn, kit)? {
        let product = catalog::get(conn, &product_id)?;
        let unit_symbol = catalog::list_units(conn, true)?
            .into_iter()
            .find(|unit| unit.id == product.unit_id)
            .map(|unit| unit.symbol)
            .unwrap_or_default();
        parts.push(KitPart {
            product_id,
            name: product.name,
            sku: product.sku,
            unit_symbol,
            qty,
        });
    }
    Ok(parts)
}

fn price_history(conn: &Connection, id: &Id) -> Result<Vec<PriceHistoryView>, AppError> {
    let names: HashMap<Id, String> = auth::list_users(conn)?
        .into_iter()
        .map(|user| (user.id, user.display_name))
        .collect();
    Ok(catalog::price_history(conn, id)?
        .into_iter()
        .map(|row| PriceHistoryView {
            field: row.field,
            old: row.old,
            new: row.new,
            changed_at: row.changed_at,
            changed_by_name: row
                .changed_by
                .as_ref()
                .and_then(|user| names.get(user).cloned()),
            reason: row.reason,
        })
        .collect())
}

/// The picture as a data address the screen can show, or `None` if the file is gone.
pub(crate) fn image_address(file: &Path) -> Option<String> {
    let bytes = std::fs::read(file).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/jpeg;base64,{encoded}"))
}

fn record_product_event(
    tx: &Transaction,
    state: &AppState,
    by: &Id,
    id: &Id,
    action: &'static str,
) -> Result<(), AppError> {
    let product = catalog::get(tx, id)?;
    let details = serde_json::json!({
        "product_id": id,
        "name": product.name,
        "sku": product.sku,
    });
    audit_log::record(tx, state.clock.as_ref(), action, details, Some(by.clone()))
}

fn blank_to_none(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn field_error(field: &str, code: &str) -> AppError {
    AppError::Validation {
        field: Some(field.to_string()),
        code: code.to_string(),
        params: Default::default(),
    }
}
