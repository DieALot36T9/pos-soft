//! Sticky labels (rule CAT-12) and shelf tags (rule CAT-13) for label printers.
//! Sizes are in millimetres; the picture is 8 dots per millimetre.

use pos_core::{Bitmap1, CurrencyFormat, Money};

use super::{barcode_with_digits, feed, row, small_centered, text};
use crate::doc::{Align, Doc, LabelSize, TextSize};
use crate::labels::{ReceiptLabels, fill};
use crate::numbers::money_with_symbol;

/// Which fields a product label shows, and its size (rule CAT-12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelTemplate {
    pub size: LabelSize,
    pub currency: CurrencyFormat,
    pub show_name: bool,
    pub show_price: bool,
    pub show_unit_price: bool,
    pub show_barcode: bool,
    pub show_packed_on: bool,
    pub show_expires_on: bool,
}

/// A price per unit, worked out by `app`. Example: Money(75000) and "kg" → "৳750.00 per kg".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitPrice {
    pub price: Money,
    pub unit: String,
}

/// What one product label prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelData {
    pub name: String,
    pub price: Money,
    pub unit_price: Option<UnitPrice>,
    /// The barcode picture, from the `barcode` crate.
    pub barcode: Option<Bitmap1>,
    /// The digits printed under the barcode. Empty means none.
    pub barcode_text: String,
    /// Dates already written the shop's way by `app`. Example: "2026-10-04".
    pub packed_on: Option<String>,
    pub expires_on: Option<String>,
}

/// Which fields a shelf tag shows, and its size (rule CAT-13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagTemplate {
    pub size: LabelSize,
    pub currency: CurrencyFormat,
    pub show_unit_price: bool,
    pub show_barcode: bool,
}

/// What one shelf tag prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelfTagData {
    pub name: String,
    /// The normal price.
    pub price: Money,
    /// The unit price of the price shown in big text: the promotion price if there is one.
    pub unit_price: Option<UnitPrice>,
    pub barcode: Option<Bitmap1>,
    pub barcode_text: String,
    pub promotion: Option<PromotionOnTag>,
}

/// An active promotion: its price and dates, written the shop's way by `app`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionOnTag {
    pub price: Money,
    pub from: String,
    pub to: String,
}

/// Lays out a product label: name, price, unit price, barcode and dates, as the
/// template chooses. Anything taller than the label is cut off by `tspl::encode`.
pub fn label(d: &LabelData, tpl: &LabelTemplate, labels: &ReceiptLabels) -> Doc {
    let mut blocks = vec![feed(8)];
    if tpl.show_name {
        blocks.push(text(&d.name, TextSize::Normal, true, Align::Center));
    }
    if tpl.show_price {
        let price = money_with_symbol(d.price, &tpl.currency);
        blocks.push(text(price, TextSize::Large, true, Align::Center));
    }
    if tpl.show_unit_price
        && let Some(unit_price) = &d.unit_price
    {
        blocks.push(small_centered(unit_price_text(
            unit_price,
            &tpl.currency,
            labels,
        )));
    }
    if tpl.show_barcode
        && let Some(barcode) = &d.barcode
    {
        blocks.push(feed(4));
        barcode_with_digits(&mut blocks, barcode, &d.barcode_text);
    }
    let packed = d
        .packed_on
        .as_ref()
        .filter(|_| tpl.show_packed_on)
        .map(|date| format!("{}: {date}", labels.packed_on));
    let expires = d
        .expires_on
        .as_ref()
        .filter(|_| tpl.show_expires_on)
        .map(|date| format!("{}: {date}", labels.expires_on));
    match (packed, expires) {
        (Some(packed), Some(expires)) => blocks.push(row(packed, expires, TextSize::Small, false)),
        (Some(one_date), None) | (None, Some(one_date)) => blocks.push(small_centered(one_date)),
        (None, None) => {}
    }
    Doc {
        width_dots: tpl.size.width_dots(),
        blocks,
    }
}

/// Lays out a shelf tag: name and a big price. During a promotion, the big price
/// is the promotion price, with the normal price and the offer dates under it.
pub fn shelf_tag(d: &ShelfTagData, tpl: &TagTemplate, labels: &ReceiptLabels) -> Doc {
    let currency = &tpl.currency;
    let mut blocks = vec![
        feed(8),
        text(&d.name, TextSize::Normal, true, Align::Center),
    ];
    match &d.promotion {
        Some(promotion) => {
            let promotion_price = money_with_symbol(promotion.price, currency);
            blocks.push(text(promotion_price, TextSize::Huge, true, Align::Center));
            let normal_price = money_with_symbol(d.price, currency);
            blocks.push(small_centered(fill(
                &labels.regular_price,
                &[("price", &normal_price)],
            )));
            blocks.push(small_centered(fill(
                &labels.offer_dates,
                &[("from", &promotion.from), ("to", &promotion.to)],
            )));
        }
        None => {
            let price = money_with_symbol(d.price, currency);
            blocks.push(text(price, TextSize::Huge, true, Align::Center));
        }
    }
    if tpl.show_unit_price
        && let Some(unit_price) = &d.unit_price
    {
        blocks.push(small_centered(unit_price_text(
            unit_price, currency, labels,
        )));
    }
    if tpl.show_barcode
        && let Some(barcode) = &d.barcode
    {
        blocks.push(feed(4));
        barcode_with_digits(&mut blocks, barcode, &d.barcode_text);
    }
    Doc {
        width_dots: tpl.size.width_dots(),
        blocks,
    }
}

/// "৳750.00 per kg"
fn unit_price_text(
    unit_price: &UnitPrice,
    currency: &CurrencyFormat,
    labels: &ReceiptLabels,
) -> String {
    let price = money_with_symbol(unit_price.price, currency);
    fill(
        &labels.unit_price,
        &[("price", &price), ("unit", &unit_price.unit)],
    )
}
