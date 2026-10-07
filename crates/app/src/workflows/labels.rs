//! Barcode labels and shelf tags (CAT-12, CAT-13). Products are picked by hand, by
//! category, by "price changed since" or from a delivery; each label is drawn by
//! `printing` and shown as a picture first. Printing goes to the label printer as
//! TSPL, or to the Preview printer when no label printer is set up. A4 sheets are
//! laid out by the screen from the same pictures and printed through the system print
//! dialog (ADR-013).

use std::collections::HashMap;

use barcode::Symbology;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use catalog::{Product, ProductFilter, ProductKind, ProductPage};
use chrono::{DateTime, NaiveDate};
use pos_core::{AppError, Bitmap1, CurrencyFormat, Id, Money, Qty, Timestamp};
use pricing::{LocalTime, PriceLineIn, PricingProduct};
use printing::{
    LabelData, LabelSize, LabelTemplate, PromotionOnTag, ShelfTagData, TagTemplate, UnitPrice,
    escpos, rasterize, to_png, tspl,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use settings::HardwareSettings;
use ts_rs::TS;

use crate::devices::Devices;
use crate::receipt_words;
use crate::state::AppState;

/// The most labels of one product printed at once, so a typing slip can't print a roll.
const MAX_COPIES: u32 = 500;

/// How to choose the products (CAT-12, CAT-13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum LabelPick {
    /// These products, one label each.
    Products { ids: Vec<Id> },
    /// Every product in this category and the ones below it.
    Category { id: Id },
    /// Products whose sale price changed on this day or later (shop's local day).
    PriceChangedSince { date: NaiveDate },
    /// The products of one delivery, as many labels as pieces received.
    Delivery { id: Id },
}

/// A product to print, with how many labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LabelItem {
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub price: Money,
    pub copies: u32,
}

/// The picked products and the shop's currency, for the list on screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct LabelPicked {
    pub items: Vec<LabelItem>,
    pub currency: CurrencyFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LabelKind {
    /// A barcode label stuck on the product (CAT-12).
    Label,
    /// A tag on the shelf edge with a big price (CAT-13).
    ShelfTag,
}

/// The template: size and what it shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LabelSettings {
    pub kind: LabelKind,
    pub width_mm: u32,
    pub height_mm: u32,
    /// The gap between labels on the roll.
    pub gap_mm: u32,
    pub show_name: bool,
    pub show_price: bool,
    /// The price per kg or litre. Only for products sold by weight.
    pub show_unit_price: bool,
    pub show_barcode: bool,
    /// Printed on labels when set.
    pub packed_on: Option<NaiveDate>,
    pub expires_on: Option<NaiveDate>,
}

/// One product and how many labels of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LabelCopies {
    pub product_id: Id,
    pub copies: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LabelRequest {
    pub settings: LabelSettings,
    pub items: Vec<LabelCopies>,
}

/// One product's label as it prints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct LabelPicture {
    pub product_id: Id,
    pub copies: u32,
    /// A `data:image/png;base64,…` address, one printer dot per pixel (8 per mm).
    pub picture: String,
}

/// Where the labels went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct LabelsPrinted {
    pub labels: u32,
    /// True when no label printer is set up, so they went to the Preview printer.
    pub to_preview: bool,
}

/// The products to print and how many labels of each.
pub fn pick(state: &AppState, pick: &LabelPick) -> Result<LabelPicked, AppError> {
    let offset = state.clock.local_offset_minutes();
    state.db.read(|conn| {
        let mut wanted: Vec<(Id, u32)> = Vec::new();
        match pick {
            LabelPick::Products { ids } => wanted.extend(ids.iter().map(|id| (id.clone(), 1))),
            LabelPick::Category { id } => {
                let filter = ProductFilter {
                    category_id: Some(id.clone()),
                    ..ProductFilter::default()
                };
                let everything = ProductPage {
                    offset: 0,
                    limit: u32::MAX,
                };
                for row in catalog::list(conn, &filter, everything)?.rows {
                    wanted.push((row.id, 1));
                }
            }
            LabelPick::PriceChangedSince { date } => {
                let since = start_of_day(*date, offset)?;
                for id in catalog::sale_price_changed_since(conn, since)? {
                    wanted.push((id, 1));
                }
            }
            LabelPick::Delivery { id } => {
                for line in purchasing::get_grn(conn, id)?.lines {
                    wanted.push((line.product_id, pieces(line.qty)));
                }
            }
        }
        let mut items: Vec<LabelItem> = Vec::new();
        for (id, copies) in wanted {
            if let Some(item) = items.iter_mut().find(|item| item.product_id == id) {
                item.copies = item.copies.saturating_add(copies).min(MAX_COPIES);
                continue;
            }
            let product = catalog::get(conn, &id)?;
            if product.archived_at.is_some() || product.kind == ProductKind::VariantParent {
                continue;
            }
            items.push(LabelItem {
                product_id: product.id,
                name: product.name,
                sku: product.sku,
                price: product.sale_price,
                copies: copies.min(MAX_COPIES),
            });
        }
        Ok(LabelPicked {
            items,
            currency: settings::get(conn)?,
        })
    })
}

/// Every label as a picture, exactly as it prints (CAT-12: preview first).
pub fn preview(
    state: &AppState,
    devices: &Devices,
    request: &LabelRequest,
) -> Result<Vec<LabelPicture>, AppError> {
    let mut pictures = Vec::new();
    for (item, picture) in draw_all(state, devices, request)? {
        let png = to_png(&picture)?;
        pictures.push(LabelPicture {
            product_id: item.product_id.clone(),
            copies: item.copies,
            picture: format!("data:image/png;base64,{}", STANDARD.encode(&png)),
        });
    }
    Ok(pictures)
}

/// Prints every label: TSPL to the label printer, or the Preview printer when none is
/// set up (HW-06, HW-10).
pub fn print(
    state: &AppState,
    devices: &Devices,
    request: &LabelRequest,
) -> Result<LabelsPrinted, AppError> {
    let drawn = draw_all(state, devices, request)?;
    let hardware: HardwareSettings = state.db.read(|conn| Ok(settings::get(conn)?))?;
    let size = label_size(&request.settings)?;
    let mut labels = 0_u32;
    match &hardware.label_printer {
        Some(connection) => {
            let printer = devices.label_printer(connection);
            for (item, picture) in &drawn {
                devices.send(printer.as_ref(), &tspl::encode(picture, &size, item.copies))?;
                labels = labels.saturating_add(item.copies);
            }
        }
        None => {
            let printer = devices.receipt_printer(&HardwareSettings::default());
            for (item, picture) in &drawn {
                let bytes = escpos::encode(picture, &escpos::EscPosOpts::default())?;
                for _ in 0..item.copies {
                    devices.send(printer.as_ref(), &bytes)?;
                }
                labels = labels.saturating_add(item.copies);
            }
        }
    }
    tracing::info!(labels, "labels printed");
    Ok(LabelsPrinted {
        labels,
        to_preview: hardware.label_printer.is_none(),
    })
}

/// Draws one picture per product.
fn draw_all<'a>(
    state: &AppState,
    devices: &Devices,
    request: &'a LabelRequest,
) -> Result<Vec<(&'a LabelCopies, Bitmap1)>, AppError> {
    let size = label_size(&request.settings)?;
    let settings = &request.settings;
    let words = receipt_words::english();
    let now = LocalTime {
        at: state.clock.now(),
        offset_minutes: state.clock.local_offset_minutes(),
    };
    let mut drawn = Vec::with_capacity(request.items.len());
    for item in &request.items {
        if item.copies == 0 || item.copies > MAX_COPIES {
            return Err(AppError::Validation {
                field: Some("copies".into()),
                code: "label_copies_out_of_range".into(),
                params: [("max".to_string(), MAX_COPIES.to_string())].into(),
            });
        }
        let doc = state.db.read(|conn| {
            let product = catalog::get(conn, &item.product_id)?;
            let currency: CurrencyFormat = settings::get(conn)?;
            let unit_price = if settings.show_unit_price && product.is_weighed {
                let (symbol, _) = crate::workflows::stock::unit_of(conn, &product.unit_id)?;
                Some(UnitPrice {
                    price: product.sale_price,
                    unit: symbol,
                })
            } else {
                None
            };
            let (code, symbology) = code_to_print(&product);
            let barcode = if settings.show_barcode {
                let width = size.width_dots().saturating_sub(32);
                let height = (size.height_dots() / 4).clamp(40, 120);
                Some(barcode::bitmap(&code, symbology, width, height)?)
            } else {
                None
            };
            let barcode_text = if barcode.is_some() {
                code
            } else {
                String::new()
            };
            Ok(match settings.kind {
                LabelKind::Label => printing::label(
                    &LabelData {
                        name: product.name.clone(),
                        price: product.sale_price,
                        unit_price,
                        barcode,
                        barcode_text,
                        packed_on: settings.packed_on.map(|date| date.to_string()),
                        expires_on: settings.expires_on.map(|date| date.to_string()),
                    },
                    &LabelTemplate {
                        size,
                        currency,
                        show_name: settings.show_name,
                        show_price: settings.show_price,
                        show_unit_price: settings.show_unit_price,
                        show_barcode: settings.show_barcode,
                        show_packed_on: settings.packed_on.is_some(),
                        show_expires_on: settings.expires_on.is_some(),
                    },
                    &words,
                ),
                LabelKind::ShelfTag => {
                    let promotion = promotion_on_tag(conn, &product, now)?;
                    let unit_price = unit_price.map(|unit_price| UnitPrice {
                        price: promotion
                            .as_ref()
                            .map_or(unit_price.price, |offer| offer.price),
                        ..unit_price
                    });
                    printing::shelf_tag(
                        &ShelfTagData {
                            name: product.name.clone(),
                            price: product.sale_price,
                            unit_price,
                            barcode,
                            barcode_text,
                            promotion,
                        },
                        &TagTemplate {
                            size,
                            currency,
                            show_unit_price: settings.show_unit_price,
                            show_barcode: settings.show_barcode,
                        },
                        &words,
                    )
                }
            })
        })?;
        drawn.push((item, rasterize(&doc, &devices.fonts)?));
    }
    Ok(drawn)
}

/// 20 to 120 mm wide, 10 to 120 mm high, a gap of at most 20 mm.
fn label_size(settings: &LabelSettings) -> Result<LabelSize, AppError> {
    let fits = (20..=120).contains(&settings.width_mm)
        && (10..=120).contains(&settings.height_mm)
        && settings.gap_mm <= 20;
    if !fits {
        return Err(AppError::Validation {
            field: Some("size".into()),
            code: "label_size_out_of_range".into(),
            params: Default::default(),
        });
    }
    Ok(LabelSize {
        width_mm: settings.width_mm,
        height_mm: settings.height_mm,
        gap_mm: settings.gap_mm,
    })
}

/// The product's first barcode in its own symbology, or else its SKU as Code 128.
fn code_to_print(product: &Product) -> (String, Symbology) {
    let Some(code) = product.barcodes.first() else {
        return (product.sku.clone(), Symbology::Code128);
    };
    let digits = code.bytes().all(|byte| byte.is_ascii_digit());
    let symbology = match code.len() {
        13 if digits && barcode::is_valid_gtin(code) => Symbology::Ean13,
        8 if digits && barcode::is_valid_gtin(code) => Symbology::Ean8,
        12 if digits && barcode::is_valid_gtin(code) => Symbology::UpcA,
        _ => Symbology::Code128,
    };
    (code.clone(), symbology)
}

/// The offer price of one unit today and the offer's dates, if a promotion lowers it
/// (CAT-13). Tiers and price lists are left out: the shelf shows the price for everyone.
fn promotion_on_tag(
    conn: &Connection,
    product: &Product,
    now: LocalTime,
) -> Result<Option<PromotionOnTag>, AppError> {
    let infos = catalog::price_info(conn, std::slice::from_ref(&product.id))?;
    let Some(info) = infos.get(&product.id) else {
        return Ok(None);
    };
    let mut category_path = info.category_path.clone();
    category_path.reverse();
    let priced = PricingProduct {
        id: product.id.clone(),
        retail_price: info.sale_price,
        category_path,
        brand_id: info.brand_id.clone(),
    };
    let context = pricing::load_context(conn, std::slice::from_ref(&priced), None, None, now)?;
    let line = PriceLineIn {
        key: 0,
        product: priced,
        qty: Qty(1000),
        embedded_price: None,
        price_override: None,
        manual: None,
    };
    let lines = pricing::price_cart(&context, &[line], None)?;
    let Some(out) = lines.first() else {
        return Ok(None);
    };
    let Some(applied) = &out.promo else {
        return Ok(None);
    };
    if out.amount >= out.base {
        return Ok(None);
    }
    let promotions: HashMap<&Id, &pricing::Promotion> = context
        .promotions
        .iter()
        .map(|promotion| (&promotion.id, promotion))
        .collect();
    let promotion = promotions.get(&applied.promotion_id);
    let day = |at: Option<Timestamp>| {
        at.and_then(|at| local_date(at, now.offset_minutes))
            .map(|date| date.to_string())
            .unwrap_or_default()
    };
    Ok(Some(PromotionOnTag {
        price: out.amount,
        from: day(promotion
            .and_then(|promotion| promotion.starts_at)
            .or(Some(now.at))),
        to: day(promotion.and_then(|promotion| promotion.ends_at)),
    }))
}

/// The local calendar day of a moment.
fn local_date(at: Timestamp, offset_minutes: i32) -> Option<NaiveDate> {
    let local = at.0.checked_add(i64::from(offset_minutes) * 60_000)?;
    DateTime::from_timestamp_millis(local).map(|moment| moment.date_naive())
}

/// Midnight at the start of a local day, as a moment.
fn start_of_day(date: NaiveDate, offset_minutes: i32) -> Result<Timestamp, AppError> {
    let midnight = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::internal("midnight is always a time"))?
        .and_utc()
        .timestamp_millis();
    Ok(Timestamp(midnight - i64::from(offset_minutes) * 60_000))
}

/// Whole pieces for a received quantity, at least one label. Example: 2.5 kg → 3.
fn pieces(qty: Qty) -> u32 {
    let whole = (qty.0.max(1) + 999) / 1000;
    u32::try_from(whole).unwrap_or(MAX_COPIES).min(MAX_COPIES)
}
