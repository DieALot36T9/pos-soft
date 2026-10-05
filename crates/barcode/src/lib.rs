//! Reads scale labels, checks check digits, makes internal barcodes and barcode pictures.
//! Pure code: no tables, no files, no devices. `app` passes in the shop's settings.
//!
//! Page: `docs/05 Modules/Module - barcode.md`.

mod draw;
mod gtin;
mod rule;
mod scan;

pub use draw::{SvgOpts, Symbology, bitmap, svg};
pub use gtin::{ean13_check_digit, internal_ean13, is_valid_gtin};
pub use rule::{ScaleKind, ScaleRule, default_rules, rules_overlap, validate_rule};
pub use scan::{Scanned, parse};
