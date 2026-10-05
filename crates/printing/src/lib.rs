//! Turns documents into a picture and then into printer bytes: receipts, reports, labels and shelf tags.
//! Pure: it never talks to devices. Sending the bytes is the `hardware` crate's job.
//!
//! Page: `docs/05 Modules/Module - printing.md`. Design: ADR-012 (receipts printed as pictures).

mod doc;
mod error;
pub mod escpos;
mod fonts;
mod labels;
mod layouts;
mod numbers;
mod pictures;
mod raster;
mod text;
pub mod tspl;

pub use self::escpos::{DrawerPin, EscPosOpts};
pub use doc::{Align, Block, Doc, LabelSize, PaperWidth, RuleStyle, TextSize};
pub use error::PrintError;
pub use fonts::{Fonts, load_fonts};
pub use labels::ReceiptLabels;
pub use layouts::label::{
    LabelData, LabelTemplate, PromotionOnTag, ShelfTagData, TagTemplate, UnitPrice, label,
    shelf_tag,
};
pub use layouts::receipt::{receipt, return_receipt};
pub use layouts::receipt_data::{
    CustomerOnReceipt, LineDiscount, PaymentLine, ReceiptData, ReceiptLayout, ReceiptLine,
    ReturnData, ShopHeader, TaxSummaryLine,
};
pub use layouts::report::shift_report;
pub use pictures::{logo_from_image, to_png};
pub use raster::rasterize;
