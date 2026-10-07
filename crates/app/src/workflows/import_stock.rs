//! Import rows that belong to a product that exists already (IO-02): opening stock,
//! prices for one price list, and extra barcodes. Each row finds its product by SKU
//! or by barcode, as the wizard chose.

use std::collections::HashMap;

use catalog::Product;
use inventory::{BatchChoice, MoveCtx, MoveKind, Movement};
use pos_core::{AppError, CurrencyFormat, Id, Money};
use pricing::PriceListItem;
use rusqlite::Transaction;

use crate::state::AppState;
use crate::workflows::import_file::{
    Field, ImportEntity, ImportRequest, MatchBy, RowAction, RowCheck, RowCtx,
};
use crate::workflows::import_products::product_with_sku;
use crate::workflows::products::clean_barcodes;
use crate::workflows::stock::{move_ctx, require_stocked};

const PRODUCT: Field = field(
    "product",
    "SKU",
    &["Barcode", "Item code", "Code", "Product code"],
    true,
);

/// Opening stock. The batches export has these columns, so it imports back here.
pub(crate) const OPENING_FIELDS: &[Field] = &[
    PRODUCT,
    field("qty", "Quantity", &["Qty", "Stock", "On hand"], true),
    field("unit_cost", "Unit cost", &["Cost", "Cost price"], false),
    field("batch_no", "Batch", &["Batch number", "Lot"], false),
    field("expiry_date", "Expiry date", &["Expiry", "Expires"], false),
];

/// Prices for one price list.
pub(crate) const PRICE_FIELDS: &[Field] = &[
    PRODUCT,
    field("price", "Price", &["List price", "Sale price"], true),
];

/// Extra barcodes. The product is found by SKU here, so "Barcode" means the new code.
pub(crate) const BARCODE_FIELDS: &[Field] = &[
    field(
        "product",
        "SKU",
        &["Item code", "Code", "Product code"],
        true,
    ),
    field("barcode", "Barcode", &["New barcode", "EAN", "UPC"], true),
];

const fn field(
    key: &'static str,
    header: &'static str,
    other_names: &'static [&'static str],
    required: bool,
) -> Field {
    Field {
        key,
        header,
        other_names,
        required,
    }
}

/// What these rows need, read once per import.
pub(crate) struct Lookups {
    money_digits: u8,
    unit_decimals: HashMap<Id, u8>,
    /// Who, when and where, for opening stock.
    move_ctx: MoveCtx,
    /// The price list and its current prices, for price list prices.
    price_list: Option<(Id, HashMap<Id, Money>)>,
}

impl Lookups {
    pub(crate) fn load(
        tx: &Transaction,
        state: &AppState,
        request: &ImportRequest,
        by: &Id,
    ) -> Result<Lookups, AppError> {
        let currency: CurrencyFormat = settings::get(tx)?;
        let price_list = match (request.entity, &request.price_list_id) {
            (ImportEntity::PriceListPrices, None) => {
                return Err(AppError::Validation {
                    field: Some("price_list_id".into()),
                    code: "price_list_needed".into(),
                    params: Default::default(),
                });
            }
            (ImportEntity::PriceListPrices, Some(id)) => {
                let prices = pricing::price_list_items(tx, id)?
                    .into_iter()
                    .map(|item| (item.product_id, item.price))
                    .collect();
                Some((id.clone(), prices))
            }
            _ => None,
        };
        Ok(Lookups {
            money_digits: currency.minor_digits,
            unit_decimals: catalog::list_units(tx, true)?
                .into_iter()
                .map(|unit| (unit.id, unit.decimals))
                .collect(),
            move_ctx: move_ctx(tx, state.clock.as_ref(), by)?,
            price_list,
        })
    }
}

/// The product a row names, by SKU or by barcode. An error on the cell when unknown.
fn find_product(
    tx: &Transaction,
    ctx: &RowCtx,
    check: &mut RowCheck,
) -> Result<Option<Product>, AppError> {
    let text = check.required("product");
    if text.is_empty() {
        return Ok(None);
    }
    let barcode_column =
        ctx.request.entity != ImportEntity::Barcodes && ctx.request.match_by == MatchBy::Barcode;
    let id = if barcode_column {
        catalog::find_by_code(tx, &text)?.map(|product| product.id)
    } else {
        product_with_sku(tx, &text)?
    };
    match id {
        Some(id) => Ok(Some(catalog::get(tx, &id)?)),
        None => {
            check.fail("product", "product_unknown");
            Ok(None)
        }
    }
}

/// Saves one opening stock row as an `opening` movement, in its own batch when it has
/// a batch number or an expiry date.
pub(crate) fn save_opening(
    tx: &Transaction,
    ctx: &RowCtx,
    lookups: &mut Lookups,
    check: &mut RowCheck,
) -> Result<RowAction, AppError> {
    let product = find_product(tx, ctx, check)?;
    let decimals = product
        .as_ref()
        .and_then(|product| lookups.unit_decimals.get(&product.unit_id).copied())
        .unwrap_or(3);
    let qty = check.qty("qty", decimals);
    if check.cell("qty").is_empty() {
        check.fail("qty", "cell_empty");
    }
    if qty.is_some_and(|qty| qty.0 <= 0) {
        check.fail("qty", "qty_not_positive");
    }
    let cost = check.money("unit_cost", lookups.money_digits);
    let batch_no = check.text("batch_no");
    let expiry = check.date("expiry_date");
    if let Some(product) = &product {
        if require_stocked(product).is_err() {
            check.fail("product", "product_keeps_no_stock");
        } else if product.track_expiry && expiry.is_none() && check.cell("expiry_date").is_empty() {
            check.fail("expiry_date", "expiry_needed");
        }
    }
    check.ready()?;
    let (Some(product), Some(qty)) = (product, qty) else {
        return Err(AppError::validation("row_has_errors"));
    };
    let cost = cost.unwrap_or(product.cost_price);
    let batch = if batch_no.is_some() || expiry.is_some() {
        BatchChoice::Specific(inventory::create_batch(
            tx,
            &product.id,
            batch_no.as_deref(),
            expiry,
            cost,
            ctx.now,
        )?)
    } else {
        BatchChoice::None
    };
    let movement = Movement {
        batch,
        unit_cost: cost,
        tracks_expiry: product.track_expiry,
        ..Movement::new(
            &product.id,
            qty,
            MoveKind::Opening,
            lookups.move_ctx.clone(),
        )
    };
    inventory::record(tx, movement, true)?;
    Ok(RowAction::Create)
}

/// Saves one price for the chosen price list.
pub(crate) fn save_price(
    tx: &Transaction,
    ctx: &RowCtx,
    lookups: &mut Lookups,
    check: &mut RowCheck,
) -> Result<RowAction, AppError> {
    let product = find_product(tx, ctx, check)?;
    let price = check.money("price", lookups.money_digits);
    if check.cell("price").is_empty() {
        check.fail("price", "cell_empty");
    }
    check.ready()?;
    let (Some(product), Some(price), Some((list, prices))) =
        (product, price, &mut lookups.price_list)
    else {
        return Err(AppError::validation("row_has_errors"));
    };
    let action = match prices.get(&product.id) {
        Some(saved) if *saved == price => return Ok(RowAction::Skip),
        Some(_) => RowAction::Update,
        None => RowAction::Create,
    };
    let item = PriceListItem {
        product_id: product.id.clone(),
        price,
    };
    pricing::set_price_list_items(tx, list, &[item])?;
    prices.insert(product.id, price);
    Ok(action)
}

/// Adds one more barcode to a product.
pub(crate) fn save_barcode(
    tx: &Transaction,
    ctx: &RowCtx,
    _lookups: &mut Lookups,
    check: &mut RowCheck,
) -> Result<RowAction, AppError> {
    let product = find_product(tx, ctx, check)?;
    let code = check.required("barcode");
    if !code.is_empty() && clean_barcodes(std::slice::from_ref(&code)).is_err() {
        check.fail("barcode", "barcode_check_digit_wrong");
    }
    check.ready()?;
    let Some(product) = product else {
        return Err(AppError::validation("row_has_errors"));
    };
    if product.barcodes.contains(&code) {
        return Ok(RowAction::Skip);
    }
    catalog::add_barcode(tx, &product.id, &code, ctx.now)?;
    Ok(RowAction::Create)
}
