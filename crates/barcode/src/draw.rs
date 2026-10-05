//! Barcode pictures for labels and shelf tags (Rule CAT-12). The `barcoders` crate
//! turns a code into bars; this file adds the blank margins scanners need
//! (quiet zones) and draws the bars as SVG or as a 1-bit picture for printing.

use barcoders::generators::svg::SVG;
use barcoders::sym::code128::Code128;
use barcoders::sym::ean8::EAN8;
use barcoders::sym::ean13::EAN13;
use pos_core::{AppError, Bitmap1};

use crate::gtin::is_valid_gtin;

/// The kinds of barcode this module can draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbology {
    /// 13 digits, check digit included. Most shop products and our internal codes.
    Ean13,
    /// 8 digits, check digit included. Small packets.
    Ean8,
    /// 12 digits, check digit included. North American products.
    UpcA,
    /// Letters, digits and signs (printable ASCII). Used for SKUs.
    Code128,
}

/// Size of an SVG barcode, in SVG units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvgOpts {
    /// Width of the thinnest bar. Example: 2.
    pub module_width: u32,
    /// Height of the bars. Example: 60.
    pub height: u32,
}

impl Default for SvgOpts {
    fn default() -> SvgOpts {
        SvgOpts {
            module_width: 2,
            height: 60,
        }
    }
}

/// Draws the barcode as an SVG picture, quiet zones included. Example use: label previews.
pub fn svg(code: &str, symbology: Symbology, options: &SvgOpts) -> Result<String, AppError> {
    if options.module_width == 0 || options.height == 0 {
        return Err(AppError::validation("barcode_size_invalid"));
    }
    let bars = bars_with_quiet_zones(code, symbology)?;
    SVG::new(options.height)
        .xdim(options.module_width)
        .xmlns("http://www.w3.org/2000/svg".to_string())
        .generate(&bars)
        .map_err(|error| AppError::internal(format!("barcoders could not draw {code}: {error}")))
}

/// Draws the barcode as a black-and-white picture of exactly this many printer dots,
/// for the `printing` module. Every bar is a whole number of dots wide, so the
/// printed barcode stays sharp and scans. The barcode is centred left to right.
pub fn bitmap(
    code: &str,
    symbology: Symbology,
    width_dots: u32,
    height_dots: u32,
) -> Result<Bitmap1, AppError> {
    if width_dots == 0 || height_dots == 0 {
        return Err(AppError::validation("barcode_size_invalid"));
    }
    let bars = bars_with_quiet_zones(code, symbology)?;
    let bar_count =
        u32::try_from(bars.len()).map_err(|_| AppError::validation("barcode_too_wide"))?;
    if bar_count > width_dots {
        return Err(AppError::validation("barcode_too_wide"));
    }
    let dots_per_bar = width_dots / bar_count;
    let left_margin = (width_dots - bar_count * dots_per_bar) / 2;

    let mut picture = Bitmap1::new(width_dots, height_dots);
    let bytes_per_row = width_dots.div_ceil(8) as usize;
    let mut first_row = vec![0_u8; bytes_per_row];
    for (index, bar) in bars.iter().enumerate() {
        if *bar != 1 {
            continue;
        }
        // `index` is below bar_count, which fits in u32 (checked above).
        let start = left_margin + index as u32 * dots_per_bar;
        for x in start..start + dots_per_bar {
            first_row[(x / 8) as usize] |= 0x80 >> (x % 8);
        }
    }
    for row in picture.bits.chunks_mut(bytes_per_row) {
        row.copy_from_slice(&first_row);
    }
    Ok(picture)
}

/// The bars (1 = black, 0 = white), one per module, with a blank quiet zone on both sides.
fn bars_with_quiet_zones(code: &str, symbology: Symbology) -> Result<Vec<u8>, AppError> {
    let not_drawable = || AppError::validation("barcode_not_drawable");
    let bars = match symbology {
        Symbology::Ean13 if code.len() == 13 && is_valid_gtin(code) => {
            EAN13::new(code).map_err(|_| not_drawable())?.encode()
        }
        Symbology::Ean8 if code.len() == 8 && is_valid_gtin(code) => {
            EAN8::new(code).map_err(|_| not_drawable())?.encode()
        }
        // A UPC-A is an EAN-13 starting with 0: the bars are the same.
        Symbology::UpcA if code.len() == 12 && is_valid_gtin(code) => {
            EAN13::new(format!("0{code}"))
                .map_err(|_| not_drawable())?
                .encode()
        }
        Symbology::Code128 if is_printable_ascii(code) => {
            // `Ɓ` tells barcoders to use Code 128 character set B (all printable ASCII).
            Code128::new(format!("Ɓ{code}"))
                .map_err(|_| not_drawable())?
                .encode()
        }
        _ => return Err(not_drawable()),
    };
    let quiet = quiet_zone(symbology);
    let mut padded = vec![0; quiet];
    padded.extend_from_slice(&bars);
    padded.resize(padded.len() + quiet, 0);
    Ok(padded)
}

/// Blank modules needed on each side, from the GS1 specifications (EAN-13 needs
/// 11 on the left and 7 on the right; we use 11 on both sides to keep it simple).
fn quiet_zone(symbology: Symbology) -> usize {
    match symbology {
        Symbology::Ean13 => 11,
        Symbology::Ean8 => 7,
        Symbology::UpcA => 9,
        Symbology::Code128 => 10,
    }
}

fn is_printable_ascii(code: &str) -> bool {
    !code.is_empty() && code.bytes().all(|byte| (b' '..=b'~').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cat12_ean13_is_95_bars_plus_quiet_zones() {
        let bars = bars_with_quiet_zones("4006381333931", Symbology::Ean13).unwrap();
        assert_eq!(bars.len(), 11 + 95 + 11);
        assert!(bars[..11].iter().all(|bar| *bar == 0));
        assert_eq!(bars[11..14], [1, 0, 1]); // the start guard
    }

    #[test]
    fn cat12_bitmap_bars_are_whole_dots_and_centred() {
        // 117 modules into 240 dots: 2 dots per module, 6 dots left over, 3 on each side.
        let picture = bitmap("4006381333931", Symbology::Ean13, 240, 2).unwrap();
        let first_black = 3 + 11 * 2;
        let row = &picture.bits[0..30];
        let is_black = |x: usize| row[x / 8] & (0x80 >> (x % 8)) != 0;
        assert!(!is_black(first_black - 1));
        assert!(is_black(first_black) && is_black(first_black + 1));
        assert!(!is_black(first_black + 2));
        assert_eq!(picture.bits[0..30], picture.bits[30..60]);
    }

    #[test]
    fn cat12_zero_sizes_are_refused() {
        let error = bitmap("96385074", Symbology::Ean8, 0, 10).unwrap_err();
        assert_eq!(error, AppError::validation("barcode_size_invalid"));
        let options = SvgOpts {
            module_width: 0,
            height: 10,
        };
        assert!(svg("96385074", Symbology::Ean8, &options).is_err());
    }

    #[test]
    fn cat12_wrong_length_for_the_symbology_is_refused() {
        assert!(bars_with_quiet_zones("96385074", Symbology::Ean13).is_err());
        assert!(bars_with_quiet_zones("4006381333931", Symbology::UpcA).is_err());
        assert!(bars_with_quiet_zones("", Symbology::Code128).is_err());
    }
}
