//! Golden images: each printout must match its approved picture in `tests/golden/`
//! dot for dot. A changed picture needs the product owner's approval.
//!
//! When a picture differs, the test saves the new one next to it as
//! `<name>.new.png` (not committed) so you can compare the two.
//! After approval, replace the old picture: `UPDATE_GOLDEN=1 cargo nextest run -p printing`.

mod common;

use std::path::PathBuf;

use common::*;
use pos_core::Bitmap1;
use printing::{
    Doc, PaperWidth, label, load_fonts, logo_from_image, rasterize, receipt, return_receipt,
    shelf_tag, shift_report, to_png,
};

fn golden_path(name: &str, ending: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(format!("{name}{ending}"))
}

/// Draws the doc and compares it with the approved picture.
fn check_golden(name: &str, doc: &Doc) {
    let picture = rasterize(doc, &load_fonts()).expect("the doc draws");
    let png = to_png(&picture).expect("the picture saves as PNG");
    let approved_path = golden_path(name, ".png");
    let new_path = golden_path(name, ".new.png");

    if std::env::var("UPDATE_GOLDEN").is_ok_and(|value| value == "1") {
        std::fs::write(&approved_path, &png).expect("the approved picture saves");
        let _ = std::fs::remove_file(&new_path);
        return;
    }

    let Ok(approved_png) = std::fs::read(&approved_path) else {
        std::fs::write(&new_path, &png).expect("the new picture saves");
        panic!("{name}: no approved picture yet. The new one is at {new_path:?}");
    };
    let approved = from_png(&approved_png);
    if approved != picture {
        std::fs::write(&new_path, &png).expect("the new picture saves");
        panic!(
            "{name}: the picture changed ({}×{} now, {}×{} approved). \
             Compare {new_path:?} with {approved_path:?}",
            picture.width, picture.height, approved.width, approved.height
        );
    }
    let _ = std::fs::remove_file(&new_path);
}

/// Reads an approved PNG back into a 1-bit picture (black = 0 in the PNG).
fn from_png(png: &[u8]) -> Bitmap1 {
    let gray = image::load_from_memory(png)
        .expect("the approved picture is a PNG")
        .to_luma8();
    let mut picture = Bitmap1::new(gray.width(), gray.height());
    let bytes_per_row = gray.width().div_ceil(8);
    for (x, y, pixel) in gray.enumerate_pixels() {
        if pixel.0[0] < 128 {
            picture.bits[(y * bytes_per_row + x / 8) as usize] |= 0x80 >> (x % 8);
        }
    }
    picture
}

fn logo() -> Bitmap1 {
    logo_from_image(&sample_logo_png(), 576).expect("the sample logo reads")
}

#[test]
fn hw_03_receipt_80mm_with_logo_and_taka_sign() {
    let layout = sample_layout(PaperWidth::Mm80, Some(logo()));
    check_golden(
        "receipt_80mm",
        &receipt(&sample_sale(), &layout, &english_labels()),
    );
}

#[test]
fn hw_02_receipt_58mm_with_logo_and_taka_sign() {
    let layout = sample_layout(PaperWidth::Mm58, Some(logo()));
    check_golden(
        "receipt_58mm",
        &receipt(&sample_sale(), &layout, &english_labels()),
    );
}

#[test]
fn ret_06_return_receipt_80mm() {
    let layout = sample_layout(PaperWidth::Mm80, Some(logo()));
    check_golden(
        "return_80mm",
        &return_receipt(&sample_return(), &layout, &english_labels()),
    );
}

#[test]
fn ret_04_void_receipt_80mm() {
    let layout = sample_layout(PaperWidth::Mm80, None);
    check_golden(
        "void_80mm",
        &receipt(&sample_void(), &layout, &english_labels()),
    );
}

#[test]
fn shf_08_z_report_80mm() {
    let layout = sample_layout(PaperWidth::Mm80, None);
    check_golden(
        "z_report_80mm",
        &shift_report(&sample_z_report(), &layout, &english_labels()),
    );
}

#[test]
fn shf_08_z_report_58mm() {
    let layout = sample_layout(PaperWidth::Mm58, None);
    check_golden(
        "z_report_58mm",
        &shift_report(&sample_z_report(), &layout, &english_labels()),
    );
}

#[test]
fn cat_12_label_58mm() {
    check_golden(
        "label_58x40mm",
        &label(&sample_label(), &label_template(58, 40), &english_labels()),
    );
}

#[test]
fn cat_12_label_80mm() {
    check_golden(
        "label_80x50mm",
        &label(&sample_label(), &label_template(80, 50), &english_labels()),
    );
}

#[test]
fn cat_13_shelf_tag_with_promotion_80mm() {
    check_golden(
        "shelf_tag_80x40mm",
        &shelf_tag(
            &sample_shelf_tag(),
            &tag_template(80, 40),
            &english_labels(),
        ),
    );
}
