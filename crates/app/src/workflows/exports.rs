//! Exports and import templates (IO-01 step 1, IO-03). Exports use the same column
//! headers as the import, so an export imports back unchanged. CSV files start with a
//! BOM so Excel shows symbols like ৳; money has the currency's decimals and a `.`.

use std::collections::HashMap;
use std::path::Path;

use catalog::{ProductFilter, ProductKind, ProductPage};
use chrono::NaiveDate;
use io::{Table, XlsxCell, XlsxSheet};
use pos_core::{AppError, CurrencyFormat, Id, Money, Qty};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::audit_actions::DATA_EXPORT;
use crate::audit_log;
use crate::state::AppState;
use crate::workflows::import_file::{Field, ImportEntity, sql_error, yes_no};
use crate::workflows::import_products;
use crate::workflows::{import_contacts, import_stock};

/// What can be exported (IO-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ExportEntity {
    Products,
    Customers,
    Suppliers,
    StockLevels,
    PriceListPrices,
    Batches,
}

impl ExportEntity {
    fn as_str(self) -> &'static str {
        match self {
            ExportEntity::Products => "products",
            ExportEntity::Customers => "customers",
            ExportEntity::Suppliers => "suppliers",
            ExportEntity::StockLevels => "stock_levels",
            ExportEntity::PriceListPrices => "price_list_prices",
            ExportEntity::Batches => "batches",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FileFormat {
    Csv,
    Xlsx,
}

/// What to export and where.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExportRequest {
    pub entity: ExportEntity,
    pub format: FileFormat,
    pub file: String,
    /// The price list, for price list prices.
    pub price_list_id: Option<Id>,
}

/// One cell before it is written. Quantities keep their unit's decimals, so a CSV
/// file never writes more decimals than the import accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Cell {
    Empty,
    Text(String),
    Int(i64),
    Money(Money),
    Qty(Qty, u8),
    Date(NaiveDate),
}

/// A sheet before it is written.
struct Sheet {
    name: &'static str,
    headers: Vec<String>,
    rows: Vec<Vec<Cell>>,
}

/// Writes the export and returns how many rows it holds. Audited, because exports can
/// hold customer details.
pub fn export(state: &AppState, by: &Id, request: &ExportRequest) -> Result<u32, AppError> {
    let (sheet, digits) = state.db.read(|conn| {
        let currency: CurrencyFormat = settings::get(conn)?;
        let sheet = match request.entity {
            ExportEntity::Products => products(conn)?,
            ExportEntity::Customers => customers(conn)?,
            ExportEntity::Suppliers => suppliers(conn)?,
            ExportEntity::StockLevels => stock_levels(conn)?,
            ExportEntity::PriceListPrices => {
                let list = request
                    .price_list_id
                    .as_ref()
                    .ok_or_else(|| AppError::Validation {
                        field: Some("price_list_id".into()),
                        code: "price_list_needed".into(),
                        params: Default::default(),
                    })?;
                price_list_prices(conn, list)?
            }
            ExportEntity::Batches => batches(conn)?,
        };
        Ok((sheet, currency.minor_digits))
    })?;
    let rows = u32::try_from(sheet.rows.len()).unwrap_or(u32::MAX);
    write(&sheet, request.format, digits, Path::new(&request.file))?;
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let details = serde_json::json!({
            "entity": request.entity.as_str(),
            "format": match request.format { FileFormat::Csv => "csv", FileFormat::Xlsx => "xlsx" },
            "rows": rows,
        });
        audit_log::record(tx, clock, DATA_EXPORT, details, Some(by.clone()))
    })?;
    Ok(rows)
}

/// Writes an empty import file with the right headers and one example row (IO-01 step 1).
pub fn template(
    state: &AppState,
    entity: ImportEntity,
    format: FileFormat,
    file: &Path,
) -> Result<(), AppError> {
    let currency: CurrencyFormat = state.db.read(|conn| Ok(settings::get(conn)?))?;
    let fields = entity.fields();
    let example: Vec<Cell> = fields
        .iter()
        .map(|field| example_cell(entity, field.key))
        .collect();
    let sheet = Sheet {
        name: "Import",
        headers: headers(fields, &[]),
        rows: vec![example],
    };
    write(&sheet, format, currency.minor_digits, file)
}

/// A value for the template's example row. Example: "Milk 1L" for a product name.
fn example_cell(entity: ImportEntity, key: &str) -> Cell {
    let text = |value: &str| Cell::Text(value.to_string());
    match (entity, key) {
        (ImportEntity::Products, "sku") => text("P000123"),
        (ImportEntity::Products, "name") => text("Milk 1L"),
        (ImportEntity::Products, "barcodes") => text("8901234567890"),
        (ImportEntity::Products, "category") => text("Dairy"),
        (ImportEntity::Products, "unit") => text("pc"),
        (ImportEntity::Products, "sale_price") => Cell::Money(Money(9000)),
        (ImportEntity::Products, "cost_price") => Cell::Money(Money(7500)),
        (ImportEntity::Products, "weighed") => text("no"),
        (ImportEntity::Products, "track_stock") => text("yes"),
        (ImportEntity::Products, "track_expiry") => text("no"),
        (ImportEntity::Products, "reorder_level") => Cell::Qty(Qty(10_000), 0),
        (ImportEntity::Products, "reorder_qty") => Cell::Qty(Qty(24_000), 0),
        (ImportEntity::Customers, "phone") => text("01711000000"),
        (ImportEntity::Customers | ImportEntity::Suppliers, "name") => text("Rahim Traders"),
        (ImportEntity::Customers, "credit_limit") => Cell::Money(Money(500_000)),
        (ImportEntity::Customers, "loyalty_opt_out") => text("no"),
        (ImportEntity::Suppliers, "phone") => text("01811000000"),
        (ImportEntity::Suppliers, "payment_terms_days") => Cell::Int(30),
        (_, "product") => text("P000123"),
        (_, "qty") => Cell::Qty(Qty(12_000), 0),
        (_, "unit_cost") => Cell::Money(Money(7500)),
        (_, "batch_no") => text("B-0001"),
        (_, "expiry_date") => NaiveDate::from_ymd_opt(2027, 3, 31).map_or(Cell::Empty, Cell::Date),
        (_, "price") => Cell::Money(Money(8500)),
        (_, "barcode") => text("8901234567890"),
        _ => Cell::Empty,
    }
}

fn headers(fields: &[Field], extra: &[&str]) -> Vec<String> {
    fields
        .iter()
        .map(|field| field.header)
        .chain(extra.iter().copied())
        .map(str::to_string)
        .collect()
}

fn write(sheet: &Sheet, format: FileFormat, digits: u8, file: &Path) -> Result<(), AppError> {
    match format {
        FileFormat::Csv => {
            let table = Table {
                headers: sheet.headers.clone(),
                rows: sheet
                    .rows
                    .iter()
                    .map(|row| row.iter().map(|cell| cell_text(cell, digits)).collect())
                    .collect(),
            };
            io::write_csv(file, &table)?;
        }
        FileFormat::Xlsx => {
            let typed = XlsxSheet {
                name: sheet.name.to_string(),
                headers: sheet.headers.clone(),
                rows: sheet
                    .rows
                    .iter()
                    .map(|row| row.iter().map(|cell| xlsx_cell(cell, digits)).collect())
                    .collect(),
            };
            io::write_xlsx(file, &[typed])?;
        }
    }
    Ok(())
}

/// The text a CSV cell gets. Money "12.50", quantities with the unit's decimals.
fn cell_text(cell: &Cell, digits: u8) -> String {
    match cell {
        Cell::Empty => String::new(),
        Cell::Text(text) => text.clone(),
        Cell::Int(number) => number.to_string(),
        Cell::Money(amount) => io::decimal_text(amount.0, digits),
        Cell::Qty(qty, decimals) => qty_text(*qty, *decimals),
        Cell::Date(date) => date.to_string(),
    }
}

fn xlsx_cell(cell: &Cell, digits: u8) -> XlsxCell {
    match cell {
        Cell::Empty => XlsxCell::Empty,
        Cell::Text(text) => XlsxCell::Text(text.clone()),
        Cell::Int(number) => XlsxCell::Int(*number),
        Cell::Money(amount) => XlsxCell::Money {
            amount: *amount,
            minor_digits: digits,
        },
        Cell::Qty(qty, _) => XlsxCell::Qty(*qty),
        Cell::Date(date) => XlsxCell::Date(*date),
    }
}

/// A quantity with exactly the unit's decimals. Example: (Qty(1500), 3) → "1.500",
/// (Qty(2000), 0) → "2". A quantity that doesn't fit the unit keeps all 3 decimals.
fn qty_text(qty: Qty, decimals: u8) -> String {
    let decimals = decimals.min(3);
    let step = 10_i64.pow(u32::from(3 - decimals));
    if qty.0 % step == 0 {
        io::decimal_text(qty.0 / step, decimals)
    } else {
        io::decimal_text(qty.0, 3)
    }
}

fn text(value: impl Into<String>) -> Cell {
    Cell::Text(value.into())
}

fn optional(value: Option<&str>) -> Cell {
    match value {
        Some(text) => Cell::Text(text.to_string()),
        None => Cell::Empty,
    }
}

/// Every product that isn't archived, with its names looked up.
fn all_products(conn: &Connection) -> Result<Vec<catalog::Product>, AppError> {
    let everything = ProductPage {
        offset: 0,
        limit: u32::MAX,
    };
    let rows = catalog::list(conn, &ProductFilter::default(), everything)?.rows;
    let mut products = Vec::with_capacity(rows.len());
    for row in rows {
        products.push(catalog::get(conn, &row.id)?);
    }
    Ok(products)
}

fn unit_list(conn: &Connection) -> Result<HashMap<Id, (String, u8)>, AppError> {
    Ok(catalog::list_units(conn, true)?
        .into_iter()
        .map(|unit| (unit.id, (unit.symbol, unit.decimals)))
        .collect())
}

/// IO-03: products with all barcodes in one column, separated by `|`.
fn products(conn: &Connection) -> Result<Sheet, AppError> {
    let units = unit_list(conn)?;
    let categories: HashMap<Id, String> = catalog::list_categories(conn, true)?
        .into_iter()
        .map(|category| (category.id, category.name))
        .collect();
    let brands: HashMap<Id, String> = catalog::list_brands(conn, true)?
        .into_iter()
        .map(|brand| (brand.id, brand.name))
        .collect();
    let rates: HashMap<Id, String> = tax::list(conn, true)?
        .into_iter()
        .map(|rate| (rate.id, rate.name))
        .collect();
    let suppliers: HashMap<Id, String> = purchasing::list_suppliers(conn, true)?
        .into_iter()
        .map(|supplier| (supplier.id, supplier.name))
        .collect();
    let mut rows = Vec::new();
    for product in all_products(conn)? {
        if product.kind == ProductKind::VariantParent {
            continue;
        }
        let (symbol, decimals) = units.get(&product.unit_id).cloned().unwrap_or_default();
        let rule = inventory::reorder_rule(conn, &product.id)?;
        let name = |names: &HashMap<Id, String>, id: Option<&Id>| {
            optional(id.and_then(|id| names.get(id)).map(String::as_str))
        };
        rows.push(vec![
            text(product.sku),
            text(product.name),
            text(product.barcodes.join("|")),
            name(&categories, Some(&product.category_id)),
            name(&brands, product.brand_id.as_ref()),
            text(symbol),
            Cell::Money(product.sale_price),
            Cell::Money(product.cost_price),
            name(&rates, Some(&product.tax_rate_id)),
            text(yes_no(product.is_weighed)),
            text(yes_no(product.track_stock)),
            text(yes_no(product.track_expiry)),
            rule.as_ref()
                .map_or(Cell::Empty, |rule| Cell::Qty(rule.level, decimals)),
            rule.as_ref()
                .map_or(Cell::Empty, |rule| Cell::Qty(rule.qty, decimals)),
            name(&suppliers, product.preferred_supplier_id.as_ref()),
            optional(product.scale_item_code.as_deref()),
            optional(product.description.as_deref()),
        ]);
    }
    Ok(Sheet {
        name: "Products",
        headers: headers(import_products::FIELDS, &[]),
        rows,
    })
}

/// IO-03: customers with their points and balance (positive: they owe the shop).
fn customers(conn: &Connection) -> Result<Sheet, AppError> {
    let mut statement = conn
        .prepare(
            "SELECT id FROM customers_v_customers WHERE archived_at IS NULL ORDER BY lower(name)",
        )
        .map_err(sql_error)?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)?;
    let mut rows = Vec::with_capacity(ids.len());
    for id in ids {
        let found = customers::get(conn, &Id::parse(&id)?)?;
        let customer = found.customer;
        rows.push(vec![
            text(customer.phone),
            text(customer.name),
            optional(customer.email.as_deref()),
            optional(customer.address.as_deref()),
            Cell::Money(customer.credit_limit),
            text(yes_no(customer.loyalty_opt_out)),
            optional(customer.notes.as_deref()),
            Cell::Int(found.points),
            Cell::Money(found.credit),
        ]);
    }
    Ok(Sheet {
        name: "Customers",
        headers: headers(import_contacts::CUSTOMER_FIELDS, &["Points", "Balance"]),
        rows,
    })
}

/// IO-03: suppliers with what the shop owes them.
fn suppliers(conn: &Connection) -> Result<Sheet, AppError> {
    let mut rows = Vec::new();
    for supplier in purchasing::list_suppliers(conn, false)? {
        let balance = purchasing::balance(conn, &supplier.id)?;
        rows.push(vec![
            text(supplier.name),
            optional(supplier.contact_person.as_deref()),
            optional(supplier.phone.as_deref()),
            optional(supplier.email.as_deref()),
            optional(supplier.address.as_deref()),
            optional(supplier.tax_number.as_deref()),
            Cell::Int(i64::from(supplier.payment_terms_days)),
            optional(supplier.notes.as_deref()),
            Cell::Money(balance),
        ]);
    }
    Ok(Sheet {
        name: "Suppliers",
        headers: headers(import_contacts::SUPPLIER_FIELDS, &["Balance"]),
        rows,
    })
}

/// IO-03: stock levels of every product that keeps stock, with its value at cost.
fn stock_levels(conn: &Connection) -> Result<Sheet, AppError> {
    let units = unit_list(conn)?;
    let mut rows = Vec::new();
    for product in all_products(conn)? {
        if !keeps_stock(&product) {
            continue;
        }
        let (symbol, decimals) = units.get(&product.unit_id).cloned().unwrap_or_default();
        let on_hand = inventory::on_hand(conn, &product.id)?;
        rows.push(vec![
            text(product.sku),
            text(product.name),
            text(symbol),
            Cell::Qty(on_hand, decimals),
            Cell::Money(product.cost_price),
            Cell::Money(pos_core::mul_qty(product.cost_price, on_hand)?),
        ]);
    }
    Ok(Sheet {
        name: "Stock",
        headers: ["SKU", "Name", "Unit", "On hand", "Cost price", "Value"]
            .iter()
            .map(|header| header.to_string())
            .collect(),
        rows,
    })
}

/// IO-03: one price list's prices. The same columns as its import, plus the name.
fn price_list_prices(conn: &Connection, list: &Id) -> Result<Sheet, AppError> {
    let mut rows = Vec::new();
    for item in pricing::price_list_items(conn, list)? {
        let product = catalog::get(conn, &item.product_id)?;
        rows.push(vec![
            text(product.sku),
            Cell::Money(item.price),
            text(product.name),
        ]);
    }
    Ok(Sheet {
        name: "Prices",
        headers: headers(import_stock::PRICE_FIELDS, &["Name"]),
        rows,
    })
}

/// IO-03: every batch with stock. The same columns as the opening stock import.
fn batches(conn: &Connection) -> Result<Sheet, AppError> {
    let units = unit_list(conn)?;
    let mut rows = Vec::new();
    for product in all_products(conn)? {
        if !keeps_stock(&product) {
            continue;
        }
        let decimals = units
            .get(&product.unit_id)
            .map_or(3, |(_, decimals)| *decimals);
        for batch in inventory::batches_of(conn, &product.id)? {
            rows.push(vec![
                text(product.sku.clone()),
                Cell::Qty(batch.on_hand, decimals),
                Cell::Money(batch.unit_cost),
                optional(batch.batch_no.as_deref()),
                batch.expiry_date.map_or(Cell::Empty, Cell::Date),
                text(product.name.clone()),
            ]);
        }
    }
    Ok(Sheet {
        name: "Batches",
        headers: headers(import_stock::OPENING_FIELDS, &["Name"]),
        rows,
    })
}

fn keeps_stock(product: &catalog::Product) -> bool {
    matches!(product.kind, ProductKind::Standard | ProductKind::Variant) && product.track_stock
}
