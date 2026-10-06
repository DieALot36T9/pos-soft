//! Changing the till's cart (Req - Checkout, Workflows "Scan code"): scanning, search,
//! adding products and weights, quantities and removing lines. Every change is one
//! transaction and gives back the whole repriced cart.

use catalog::{Product, ProductKind, SaleProduct};
use pos_core::{AppError, Clock, CurrencyFormat, Id, Money, Qty};
use rusqlite::{Connection, Transaction};
use sales::{Cart, LineSource, NewCartLine};
use settings::{CheckoutSettings, ExpiredPolicy, InventorySettings, ScaleRules};

use crate::state::AppState;
use crate::workflows::cart_view::{self, CartView, keeps_stock, local_today};
use crate::workflows::checkout::{SaleHit, ScanResult, WeighProduct, own_cart};
use crate::workflows::scan_input::{
    looks_like_code, qty_for_sticker_price, split_multiplier, to_barcode_rule,
};
use crate::workflows::till::{conflict, conflict_with};

/// Search results at the till (CHK-03) never show more than this.
const SEARCH_LIMIT: u32 = 20;

/// Reads what was typed or scanned into the scan box (CHK-01, CHK-02, CHK-03, CHK-08).
/// "3*8901234567890" adds 3 at once (CHK-04). Text that isn't a code is searched.
pub fn scan(state: &AppState, user: &Id, input: &str) -> Result<ScanResult, AppError> {
    let (multiplier, code) = split_multiplier(input)?;
    if code.is_empty() {
        return Err(AppError::validation("scan_empty"));
    }
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let rules: ScaleRules = settings::get(tx)?;
        let currency: CurrencyFormat = settings::get(tx)?;
        let barcode_rules: Vec<barcode::ScaleRule> = rules.0.iter().map(to_barcode_rule).collect();
        match barcode::parse(code, &barcode_rules, currency.minor_digits) {
            barcode::Scanned::Plain(plain) => match catalog::find_by_code(tx, &plain)? {
                Some(product) if product.is_weighed => Ok(needs_weight(&product)),
                Some(product) => {
                    let qty = multiplier.unwrap_or(Qty(1000));
                    add(tx, clock, user, &product.id, qty, LineSource::Scan, None)
                }
                None if looks_like_code(&plain) => Err(unknown_code(code)),
                None => {
                    let hits = search_in(tx, state, &plain)?;
                    if hits.is_empty() {
                        Err(unknown_code(code))
                    } else {
                        Ok(ScanResult::Search { hits })
                    }
                }
            },
            barcode::Scanned::Weighed { item_code, qty } => {
                let product = catalog::find_by_scale_code(tx, &item_code)?
                    .ok_or_else(|| unknown_code(code))?;
                add(
                    tx,
                    clock,
                    user,
                    &product.id,
                    qty,
                    LineSource::ScaleLabel,
                    None,
                )
            }
            barcode::Scanned::Priced { item_code, price } => {
                let product = catalog::find_by_scale_code(tx, &item_code)?
                    .ok_or_else(|| unknown_code(code))?;
                let qty = qty_for_sticker_price(price, product.sale_price)?;
                add(
                    tx,
                    clock,
                    user,
                    &product.id,
                    qty,
                    LineSource::ScaleLabel,
                    Some(price),
                )
            }
        }
    })
}

/// Products for the search list under the scan box (CHK-03). Archived products and
/// variant parents never show.
pub fn search(state: &AppState, text: &str) -> Result<Vec<SaleHit>, AppError> {
    if text.trim().chars().count() < 2 {
        return Ok(Vec::new());
    }
    state.db.read(|conn| search_in(conn, state, text))
}

/// Adds a product picked from the search list. A weighed product asks for its weight.
pub fn add_product(
    state: &AppState,
    user: &Id,
    product_id: &Id,
    qty: Option<Qty>,
) -> Result<ScanResult, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let product = sellable(tx, product_id)?;
        if product.is_weighed {
            let price = catalog::price_info(tx, std::slice::from_ref(product_id))?;
            return Ok(ScanResult::NeedsWeight {
                product: WeighProduct {
                    product_id: product.id.clone(),
                    name: product.name.clone(),
                    unit_symbol: unit_symbol(tx, &product)?,
                    unit_price: price[product_id].sale_price,
                },
            });
        }
        let qty = qty.unwrap_or(Qty(1000));
        add(tx, clock, user, product_id, qty, LineSource::Search, None)
    })
}

/// Adds a weighed product with its weight from the scale or typed in (CHK-06, CHK-07).
/// Each weighing is its own line.
pub fn add_weighed(
    state: &AppState,
    user: &Id,
    product_id: &Id,
    qty: Qty,
    source: LineSource,
) -> Result<CartView, AppError> {
    let clock = state.clock.as_ref();
    let added = state.db.write(|tx| {
        let product = sellable(tx, product_id)?;
        if !product.is_weighed {
            return Err(AppError::validation("product_not_weighed"));
        }
        add(tx, clock, user, product_id, qty, source, None)
    })?;
    match added {
        ScanResult::Added { cart, .. } => Ok(cart),
        _ => Err(AppError::internal("adding a weight gave no cart")),
    }
}

/// The unit symbol of a product sold by weight, to read the scale in the right unit.
pub fn weighed_unit(state: &AppState, product_id: &Id) -> Result<String, AppError> {
    state.db.read(|conn| {
        let product = sellable(conn, product_id)?;
        unit_symbol(conn, &product)
    })
}

/// Changes a line's quantity (CHK-04). 0 removes the line; a piece can't be 1.5.
pub fn set_qty(state: &AppState, user: &Id, line_id: &Id, qty: Qty) -> Result<CartView, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let cart = own_cart(tx, clock, user)?;
        let line = cart
            .lines
            .iter()
            .find(|line| line.id == *line_id)
            .ok_or_else(|| not_found_line(line_id))?;
        if qty.0 > 0 {
            let product = catalog::get(tx, &line.product_id)?;
            check_decimals(tx, &product, qty)?;
            let others = qty_in_cart(&cart, &line.product_id).0 - line.qty.0;
            check_stock(tx, clock, &product, Qty(others + qty.0))?;
        }
        sales::set_qty(tx, line_id, qty, clock.now())?;
        cart_view::cart_view(tx, clock, sales::get_cart(tx, &cart.id)?)
    })
}

/// Takes the selected line out of the cart (CHK-05).
pub fn remove_line(state: &AppState, user: &Id, line_id: &Id) -> Result<CartView, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let cart = own_cart(tx, clock, user)?;
        if !cart.lines.iter().any(|line| line.id == *line_id) {
            return Err(not_found_line(line_id));
        }
        sales::remove_line(tx, line_id, clock.now())?;
        cart_view::cart_view(tx, clock, sales::get_cart(tx, &cart.id)?)
    })
}

/// Puts a product in the cart: checks the quantity and stock first, then reprices.
fn add(
    tx: &Transaction,
    clock: &dyn Clock,
    user: &Id,
    product_id: &Id,
    qty: Qty,
    source: LineSource,
    sticker_price: Option<Money>,
) -> Result<ScanResult, AppError> {
    let cart = own_cart(tx, clock, user)?;
    let product = sellable(tx, product_id)?;
    check_decimals(tx, &product, qty)?;
    check_stock(
        tx,
        clock,
        &product,
        Qty(qty_in_cart(&cart, product_id).0 + qty.0),
    )?;
    let checkout: CheckoutSettings = settings::get(tx)?;
    // Weighed items and scale stickers always get their own line (Module - sales).
    let merge = checkout.merge_scans && !product.is_weighed && sticker_price.is_none();
    let line = NewCartLine {
        product_id: product_id.clone(),
        qty,
        source,
        embedded_price: sticker_price,
    };
    let line_id = sales::add_line(tx, &cart.id, line, merge, clock.now())?;
    let cart = cart_view::cart_view(tx, clock, sales::get_cart(tx, &cart.id)?)?;
    Ok(ScanResult::Added { cart, line_id })
}

/// Rule CHK-21: with negative stock off, selling more than the stock is refused; with
/// expired batches set to "block", a product with expired stock is refused.
fn check_stock(
    conn: &Connection,
    clock: &dyn Clock,
    product: &Product,
    total_qty: Qty,
) -> Result<(), AppError> {
    if !keeps_stock(product) {
        return Ok(());
    }
    let rules: InventorySettings = settings::get(conn)?;
    let available = inventory::availability(conn, &product.id, local_today(clock)?)?;
    if rules.expired_policy == ExpiredPolicy::Block && available.has_expired_batch {
        return Err(conflict("batch_expired"));
    }
    if !rules.allow_negative && available.on_hand < total_qty {
        return Err(conflict_with(
            "not_enough_stock",
            "available_milli",
            available.on_hand.0,
        ));
    }
    Ok(())
}

/// A piece can't be sold in parts: the quantity may have only the unit's decimals.
fn check_decimals(conn: &Connection, product: &Product, qty: Qty) -> Result<(), AppError> {
    let unit = catalog::list_units(conn, true)?
        .into_iter()
        .find(|unit| unit.id == product.unit_id)
        .ok_or_else(|| AppError::internal("a product's unit is gone"))?;
    let step = 10_i64.pow(3_u32.saturating_sub(u32::from(unit.decimals)));
    if qty.0 % step != 0 {
        return Err(AppError::Validation {
            field: Some("qty".into()),
            code: "qty_decimals_too_many".into(),
            params: [("decimals".to_string(), unit.decimals.to_string())].into(),
        });
    }
    Ok(())
}

/// The product, if it may be sold: not archived and not a variant parent (CHK-15).
fn sellable(conn: &Connection, product_id: &Id) -> Result<Product, AppError> {
    let product = catalog::get(conn, product_id)?;
    if product.archived_at.is_some() || product.kind == ProductKind::VariantParent {
        return Err(conflict("product_not_sellable"));
    }
    Ok(product)
}

fn search_in(conn: &Connection, state: &AppState, text: &str) -> Result<Vec<SaleHit>, AppError> {
    let images = state.paths.images();
    let mut hits = Vec::new();
    for hit in catalog::search(conn, text, SEARCH_LIMIT)? {
        hits.push(SaleHit {
            on_hand: inventory::on_hand(conn, &hit.product_id)?,
            thumbnail: hit
                .thumbnail
                .as_deref()
                .and_then(|file| crate::workflows::products::image_address(&images.join(file))),
            product_id: hit.product_id,
            name: hit.name,
            sku: hit.sku,
            sale_price: hit.sale_price,
            unit_symbol: hit.unit_symbol,
        });
    }
    Ok(hits)
}

fn needs_weight(product: &SaleProduct) -> ScanResult {
    ScanResult::NeedsWeight {
        product: WeighProduct {
            product_id: product.id.clone(),
            name: product.name.clone(),
            unit_symbol: product.unit_symbol.clone(),
            unit_price: product.sale_price,
        },
    }
}

fn unit_symbol(conn: &Connection, product: &Product) -> Result<String, AppError> {
    catalog::list_units(conn, true)?
        .into_iter()
        .find(|unit| unit.id == product.unit_id)
        .map(|unit| unit.symbol)
        .ok_or_else(|| AppError::internal("a product's unit is gone"))
}

fn qty_in_cart(cart: &Cart, product_id: &Id) -> Qty {
    Qty(cart
        .lines
        .iter()
        .filter(|line| line.product_id == *product_id)
        .map(|line| line.qty.0)
        .sum())
}

/// CHK-02: "No product with code 123". The code is shown back to the cashier.
fn unknown_code(code: &str) -> AppError {
    AppError::NotFound {
        entity: "product_code".into(),
        key: code.trim().to_string(),
    }
}

fn not_found_line(line_id: &Id) -> AppError {
    AppError::NotFound {
        entity: "cart_line".into(),
        key: line_id.to_string(),
    }
}
