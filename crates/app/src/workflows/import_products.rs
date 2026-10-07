//! Product rows of an import (IO-01): each row updates the product with the same SKU
//! (or barcode) or makes a new one, with its barcodes and reorder rule. Price changes
//! go into the price history with the reason "import" (CAT-09).

use std::collections::HashMap;

use catalog::{CategoryFields, NewProduct, ProductKind, ProductUpdate, UNIT_PIECE, UnitFields};
use pos_core::{AppError, CurrencyFormat, Id, Money, Qty};
use rusqlite::{OptionalExtension, Transaction};
use settings::TaxSettings;

use crate::workflows::import_file::{
    Field, MatchBy, RowAction, RowCheck, RowCtx, name_key, sql_error,
};
use crate::workflows::products::{clean_barcodes, replace_barcodes};

/// The product fields, in the order of the template and the export.
pub(crate) const FIELDS: &[Field] = &[
    field("sku", "SKU", &["Item code", "Code", "Product code"], false),
    field(
        "name",
        "Name",
        &["Product name", "Product", "Item name", "Item"],
        true,
    ),
    field("barcodes", "Barcodes", &["Barcode", "EAN", "UPC"], false),
    field("category", "Category", &["Group"], false),
    field("brand", "Brand", &["Make"], false),
    field("unit", "Unit", &["UOM", "Unit of measure"], false),
    field(
        "sale_price",
        "Sale price",
        &["Price", "MRP", "Retail price", "Selling price"],
        true,
    ),
    field(
        "cost_price",
        "Cost price",
        &["Cost", "Purchase price", "Buying price"],
        false,
    ),
    field("tax_rate", "Tax rate", &["Tax", "VAT", "VAT rate"], false),
    field("weighed", "Weighed", &["Sold by weight"], false),
    field("track_stock", "Track stock", &["Stock item"], false),
    field("track_expiry", "Track expiry", &["Expiry"], false),
    field("reorder_level", "Reorder level", &["Minimum stock"], false),
    field("reorder_qty", "Reorder quantity", &["Reorder qty"], false),
    field("supplier", "Preferred supplier", &["Supplier"], false),
    field("scale_item_code", "Scale item code", &["PLU"], false),
    field("description", "Description", &["Notes"], false),
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

/// Names from the file matched to ids, read once per import.
pub(crate) struct Lookups {
    money_digits: u8,
    default_tax_rate: Option<Id>,
    /// By symbol and by name.
    units: HashMap<String, Id>,
    unit_decimals: HashMap<Id, u8>,
    categories: HashMap<String, Id>,
    brands: HashMap<String, Id>,
    tax_rates: HashMap<String, Id>,
    suppliers: HashMap<String, Id>,
}

impl Lookups {
    pub(crate) fn load(tx: &Transaction) -> Result<Lookups, AppError> {
        let currency: CurrencyFormat = settings::get(tx)?;
        let tax: TaxSettings = settings::get(tx)?;
        let mut units = HashMap::new();
        let mut unit_decimals = HashMap::new();
        for unit in catalog::list_units(tx, false)? {
            units.insert(name_key(&unit.name), unit.id.clone());
            units.insert(name_key(&unit.symbol), unit.id.clone());
            unit_decimals.insert(unit.id, unit.decimals);
        }
        Ok(Lookups {
            money_digits: currency.minor_digits,
            default_tax_rate: tax.default_rate_id,
            units,
            unit_decimals,
            categories: catalog::list_categories(tx, false)?
                .into_iter()
                .map(|category| (name_key(&category.name), category.id))
                .collect(),
            brands: catalog::list_brands(tx, false)?
                .into_iter()
                .map(|brand| (name_key(&brand.name), brand.id))
                .collect(),
            tax_rates: tax::list(tx, false)?
                .into_iter()
                .map(|rate| (name_key(&rate.name), rate.id))
                .collect(),
            suppliers: purchasing::list_suppliers(tx, false)?
                .into_iter()
                .map(|supplier| (name_key(&supplier.name), supplier.id))
                .collect(),
        })
    }
}

/// One product row, read and checked, before anything is saved. Empty cells are `None`.
struct Row {
    sku: Option<String>,
    name: String,
    barcodes: Option<Vec<String>>,
    unit: Option<Id>,
    category: Option<Id>,
    brand: Option<Id>,
    sale_price: Option<Money>,
    cost_price: Option<Money>,
    tax_rate: Option<Id>,
    weighed: Option<bool>,
    track_stock: Option<bool>,
    track_expiry: Option<bool>,
    reorder_level: Option<Qty>,
    reorder_qty: Option<Qty>,
    supplier: Option<Id>,
    scale_item_code: Option<String>,
    description: Option<String>,
}

/// Saves one product row.
pub(crate) fn save_row(
    tx: &Transaction,
    ctx: &RowCtx,
    lookups: &mut Lookups,
    check: &mut RowCheck,
) -> Result<RowAction, AppError> {
    let row = read_row(tx, ctx, lookups, check);
    check.ready()?;
    let existing = match ctx.request.match_by {
        MatchBy::Sku => match &row.sku {
            Some(sku) => product_with_sku(tx, sku)?,
            None => None,
        },
        MatchBy::Barcode => match row.barcodes.as_ref().and_then(|codes| codes.first()) {
            Some(code) => catalog::find_by_code(tx, code)?.map(|product| product.id),
            None => None,
        },
    };
    let (id, action) = match existing {
        Some(id) => {
            let action = update_existing(tx, ctx, &id, &row)?;
            (id, action)
        }
        None => (
            create_new(tx, ctx, lookups, check, &row)?,
            RowAction::Create,
        ),
    };
    if let (Some(level), Some(qty)) = (row.reorder_level, row.reorder_qty)
        && action != RowAction::Skip
        && catalog::get(tx, &id)?.track_stock
    {
        inventory::set_reorder_rule(tx, &id, level, qty)?;
    }
    let product = catalog::get(tx, &id)?;
    check.show("sku", product.sku);
    Ok(action)
}

/// Reads every cell of the row. Problems are noted on `check`.
fn read_row(tx: &Transaction, ctx: &RowCtx, lookups: &mut Lookups, check: &mut RowCheck) -> Row {
    let create_missing = ctx.request.create_missing;
    let digits = lookups.money_digits;
    let sku = check.text("sku");
    let name = check.required("name");
    let barcodes = match check.text("barcodes") {
        Some(text) => {
            let codes: Vec<String> = text.split('|').map(str::to_string).collect();
            let cleaned = clean_barcodes(&codes).ok();
            if cleaned.is_none() {
                check.fail("barcodes", "barcode_check_digit_wrong");
            }
            cleaned
        }
        None => None,
    };
    let unit = check.text("unit").and_then(|text| {
        find_or_make(
            tx,
            check,
            ("unit", &text),
            create_missing,
            &mut lookups.units,
            |tx, name| {
                let fields = UnitFields {
                    name: name.to_string(),
                    symbol: name.to_string(),
                    unit_code: None,
                    decimals: 0,
                };
                Ok(catalog::create_unit(tx, &fields)?)
            },
        )
    });
    let unit_decimals = unit
        .as_ref()
        .map_or(3, |id| lookups.unit_decimals.get(id).copied().unwrap_or(0));
    let category = check.text("category").and_then(|text| {
        let categories = &mut lookups.categories;
        find_or_make(
            tx,
            check,
            ("category", &text),
            create_missing,
            categories,
            |tx, name| Ok(catalog::create_category(tx, &CategoryFields::new(name))?),
        )
    });
    let brand = check.text("brand").and_then(|text| {
        find_or_make(
            tx,
            check,
            ("brand", &text),
            create_missing,
            &mut lookups.brands,
            |tx, name| Ok(catalog::create_brand(tx, name)?),
        )
    });
    let sale_price = check.money("sale_price", digits);
    if check.cell("sale_price").is_empty() {
        check.fail("sale_price", "cell_empty");
    }
    let tax_rate = check
        .text("tax_rate")
        .and_then(|text| known(check, "tax_rate", &text, &lookups.tax_rates));
    let supplier = check
        .text("supplier")
        .and_then(|text| known(check, "supplier", &text, &lookups.suppliers));
    Row {
        sku,
        name,
        barcodes,
        unit,
        category,
        brand,
        sale_price,
        cost_price: check.money("cost_price", digits),
        tax_rate,
        weighed: check.flag("weighed"),
        track_stock: check.flag("track_stock"),
        track_expiry: check.flag("track_expiry"),
        reorder_level: check.qty("reorder_level", unit_decimals),
        reorder_qty: check.qty("reorder_qty", unit_decimals),
        supplier,
        scale_item_code: check.text("scale_item_code"),
        description: check.text("description"),
    }
}

/// Changes the product with the file's values; empty cells keep what was saved.
/// A row that changes nothing is left out.
fn update_existing(
    tx: &Transaction,
    ctx: &RowCtx,
    id: &Id,
    row: &Row,
) -> Result<RowAction, AppError> {
    let product = catalog::get(tx, id)?;
    let saved = ProductUpdate::from_product(&product);
    let update = ProductUpdate {
        name: row.name.clone(),
        sku: row.sku.clone().unwrap_or(saved.sku.clone()),
        category_id: row.category.clone().unwrap_or(saved.category_id.clone()),
        brand_id: row.brand.clone().or(saved.brand_id.clone()),
        unit_id: row.unit.clone().unwrap_or(saved.unit_id.clone()),
        sale_price: row.sale_price.unwrap_or(saved.sale_price),
        cost_price: row.cost_price.unwrap_or(saved.cost_price),
        tax_rate_id: row.tax_rate.clone().unwrap_or(saved.tax_rate_id.clone()),
        is_weighed: row.weighed.unwrap_or(saved.is_weighed),
        scale_item_code: row
            .scale_item_code
            .clone()
            .or(saved.scale_item_code.clone()),
        track_stock: row.track_stock.unwrap_or(saved.track_stock),
        track_expiry: row.track_expiry.unwrap_or(saved.track_expiry),
        preferred_supplier_id: row.supplier.clone().or(saved.preferred_supplier_id.clone()),
        description: row.description.clone().or(saved.description.clone()),
    };
    let same_barcodes = row
        .barcodes
        .as_ref()
        .is_none_or(|codes| *codes == product.barcodes);
    if update == saved && same_barcodes && same_reorder(tx, id, row.reorder_level, row.reorder_qty)?
    {
        return Ok(RowAction::Skip);
    }
    catalog::update_product_with_reason(tx, id, update, (ctx.by, "import"), ctx.now)?;
    if let Some(codes) = &row.barcodes {
        replace_barcodes(tx, id, codes, ctx.now)?;
    }
    Ok(RowAction::Update)
}

/// Makes a new product. Empty cells take the product form's defaults.
fn create_new(
    tx: &Transaction,
    ctx: &RowCtx,
    lookups: &Lookups,
    check: &mut RowCheck,
    row: &Row,
) -> Result<Id, AppError> {
    let Some(tax_rate_id) = row
        .tax_rate
        .clone()
        .or_else(|| lookups.default_tax_rate.clone())
    else {
        check.fail("tax_rate", "tax_rate_missing");
        return Err(AppError::validation("row_has_errors"));
    };
    let unit_id = match &row.unit {
        Some(unit) => unit.clone(),
        None => Id::parse(UNIT_PIECE)?,
    };
    let new = NewProduct {
        sku: row.sku.clone(),
        category_id: row.category.clone(),
        brand_id: row.brand.clone(),
        sale_price: row.sale_price.unwrap_or(Money::zero()),
        cost_price: row.cost_price.unwrap_or(Money::zero()),
        is_weighed: row.weighed.unwrap_or(false),
        scale_item_code: row.scale_item_code.clone(),
        track_stock: row.track_stock.unwrap_or(true),
        track_expiry: row.track_expiry.unwrap_or(false),
        preferred_supplier_id: row.supplier.clone(),
        description: row.description.clone(),
        barcodes: row.barcodes.clone().unwrap_or_default(),
        ..NewProduct::new(ProductKind::Standard, &row.name, unit_id, tax_rate_id)
    };
    Ok(catalog::create_product(tx, new, ctx.now)?)
}

/// The product with this SKU, archived ones too.
pub(crate) fn product_with_sku(tx: &Transaction, sku: &str) -> Result<Option<Id>, AppError> {
    let found: Option<String> = tx
        .query_row(
            "SELECT id FROM catalog_v_products WHERE sku = ?1",
            [sku],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_error)?;
    found.map(|id| Id::parse(&id)).transpose()
}

/// True when the file's reorder rule is missing or the same as the saved one.
fn same_reorder(
    tx: &Transaction,
    id: &Id,
    level: Option<Qty>,
    qty: Option<Qty>,
) -> Result<bool, AppError> {
    let (Some(level), Some(qty)) = (level, qty) else {
        return Ok(true);
    };
    Ok(inventory::reorder_rule(tx, id)?.is_some_and(|rule| rule.level == level && rule.qty == qty))
}

/// The id for a name typed in the file. A name that doesn't exist yet is made when
/// "create missing" is on, otherwise it is an error on the cell.
fn find_or_make(
    tx: &Transaction,
    check: &mut RowCheck,
    (key, text): (&str, &str),
    create_missing: bool,
    known: &mut HashMap<String, Id>,
    make: impl Fn(&Transaction, &str) -> Result<Id, AppError>,
) -> Option<Id> {
    if let Some(found) = known.get(&name_key(text)) {
        return Some(found.clone());
    }
    if !create_missing {
        check.fail(key, &format!("{key}_unknown"));
        return None;
    }
    match make(tx, text) {
        Ok(id) => {
            known.insert(name_key(text), id.clone());
            Some(id)
        }
        Err(_) => {
            check.fail(key, &format!("{key}_unknown"));
            None
        }
    }
}

/// The id of a name that must exist already, such as a tax rate.
fn known(check: &mut RowCheck, key: &str, text: &str, names: &HashMap<String, Id>) -> Option<Id> {
    let found = names.get(&name_key(text)).cloned();
    if found.is_none() {
        check.fail(key, &format!("{key}_unknown"));
    }
    found
}
