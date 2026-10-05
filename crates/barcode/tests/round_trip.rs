//! Rule CAT-12: printed labels must scan correctly. Every picture this module draws is
//! read back with the `rxing` decoder (a port of ZXing) and must give the same code.

use barcode::{SvgOpts, Symbology, bitmap, svg};
use pos_core::Bitmap1;
use rxing::BarcodeFormat;

const BLACK: u8 = 0;
const WHITE: u8 = 255;

/// Every symbology with a few codes it must carry.
fn samples() -> Vec<(Symbology, &'static str, BarcodeFormat)> {
    vec![
        (Symbology::Ean13, "4006381333931", BarcodeFormat::EAN_13),
        (Symbology::Ean13, "2000000000015", BarcodeFormat::EAN_13),
        (Symbology::Ean13, "2100123012343", BarcodeFormat::EAN_13),
        (Symbology::Ean8, "96385074", BarcodeFormat::EAN_8),
        (Symbology::UpcA, "036000291452", BarcodeFormat::UPC_A),
        (Symbology::Code128, "SKU-0001", BarcodeFormat::CODE_128),
        (
            Symbology::Code128,
            "P000123 milk 1L",
            BarcodeFormat::CODE_128,
        ),
    ]
}

/// Read a barcode from grey pixels (0 is black, 255 is white).
fn decode(pixels: Vec<u8>, width: u32, height: u32, format: BarcodeFormat) -> String {
    let result = rxing::helpers::detect_in_luma(pixels, width, height, Some(format))
        .unwrap_or_else(|error| panic!("could not read the {format:?} barcode: {error:?}"));
    result.getText().to_string()
}

/// Turn a 1-bit picture into grey pixels for the decoder.
fn bitmap_pixels(picture: &Bitmap1) -> Vec<u8> {
    let bytes_per_row = picture.width.div_ceil(8) as usize;
    let mut pixels = Vec::new();
    for row in 0..picture.height as usize {
        for column in 0..picture.width as usize {
            let byte = picture.bits[row * bytes_per_row + column / 8];
            let is_black = byte & (0x80 >> (column % 8)) != 0;
            pixels.push(if is_black { BLACK } else { WHITE });
        }
    }
    pixels
}

/// Draw the SVG's black rectangles into grey pixels. The module only writes plain
/// `<rect x=".." y="0" width=".." height=".." fill="#000000"/>` bars, so a simple
/// reader is enough here.
fn svg_pixels(text: &str) -> (Vec<u8>, u32, u32) {
    let view_box = attribute(text, "viewBox");
    let sizes: Vec<u32> = view_box
        .split(' ')
        .map(|part| part.parse().expect("viewBox holds whole numbers"))
        .collect();
    let (width, height) = (sizes[2], sizes[3]);
    let mut row = vec![WHITE; width as usize];
    for rect in text.split("<rect").skip(1) {
        if attribute(rect, "fill") != "#000000" {
            continue;
        }
        let x: usize = attribute(rect, "x").parse().expect("x is a whole number");
        let bar_width: usize = attribute(rect, "width")
            .parse()
            .expect("width is a whole number");
        row[x..x + bar_width].fill(BLACK);
    }
    let mut pixels = Vec::new();
    for _ in 0..height {
        pixels.extend_from_slice(&row);
    }
    (pixels, width, height)
}

fn attribute<'a>(text: &'a str, name: &str) -> &'a str {
    let start = text
        .find(&format!(" {name}=\""))
        .expect("the attribute is there")
        + name.len()
        + 3;
    let length = text[start..].find('"').expect("the attribute value ends");
    &text[start..start + length]
}

#[test]
fn cat12_bitmaps_scan_back_to_the_same_code() {
    for (symbology, code, format) in samples() {
        let picture = bitmap(code, symbology, 384, 80).unwrap();
        assert_eq!((picture.width, picture.height), (384, 80));
        let text = decode(
            bitmap_pixels(&picture),
            picture.width,
            picture.height,
            format,
        );
        assert_eq!(text, code, "{symbology:?}");
    }
}

#[test]
fn cat12_bitmaps_with_one_dot_per_bar_still_scan() {
    // EAN-13 is 95 bars plus quiet zones: the narrowest picture we allow.
    let picture = bitmap("4006381333931", Symbology::Ean13, 117, 40).unwrap();
    let text = decode(bitmap_pixels(&picture), 117, 40, BarcodeFormat::EAN_13);
    assert_eq!(text, "4006381333931");
}

#[test]
fn cat12_svgs_scan_back_to_the_same_code() {
    let options = SvgOpts {
        module_width: 3,
        height: 90,
    };
    for (symbology, code, format) in samples() {
        let text = svg(code, symbology, &options).unwrap();
        assert!(text.starts_with("<svg"), "{text}");
        assert!(text.contains("xmlns=\"http://www.w3.org/2000/svg\""));
        let (pixels, width, height) = svg_pixels(&text);
        assert_eq!(height, 90);
        assert_eq!(decode(pixels, width, height, format), code, "{symbology:?}");
    }
}
