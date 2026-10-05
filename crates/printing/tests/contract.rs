//! Contract tests: the public API behaves as the module page says
//! (`docs/05 Modules/Module - printing.md`). Other parts of the app rely on these shapes.

mod common;

use common::*;
use pos_core::{AppError, Bitmap1};
use printing::{
    Align, Block, Doc, DrawerPin, EscPosOpts, LabelSize, PaperWidth, PrintError, TextSize, escpos,
    label, load_fonts, logo_from_image, rasterize, receipt, return_receipt, shelf_tag,
    shift_report, to_png, tspl,
};

fn text(text: &str) -> Block {
    Block::Text {
        text: text.into(),
        size: TextSize::Normal,
        bold: false,
        align: Align::Left,
    }
}

#[test]
fn hw_02_paper_widths_are_384_and_576_dots() {
    assert_eq!(PaperWidth::Mm58.dots(), 384);
    assert_eq!(PaperWidth::Mm80.dots(), 576);
}

#[test]
fn hw_02_the_picture_is_as_wide_as_the_paper() {
    let fonts = load_fonts();
    for paper in [PaperWidth::Mm58, PaperWidth::Mm80] {
        let doc = receipt(
            &sample_sale(),
            &sample_layout(paper, None),
            &english_labels(),
        );
        assert_eq!(doc.width_dots, paper.dots());
        let picture = rasterize(&doc, &fonts).unwrap();
        assert_eq!(picture.width, paper.dots());
        assert!(picture.height > 400, "a whole receipt is long");
    }
}

#[test]
fn an_empty_doc_is_an_empty_picture() {
    let picture = rasterize(
        &Doc {
            width_dots: 576,
            blocks: vec![],
        },
        &load_fonts(),
    )
    .unwrap();
    assert_eq!((picture.width, picture.height), (576, 0));
}

#[test]
fn feed_adds_exactly_that_many_white_rows() {
    let picture = rasterize(
        &Doc {
            width_dots: 384,
            blocks: vec![Block::Feed { dots: 40 }],
        },
        &load_fonts(),
    )
    .unwrap();
    assert_eq!(picture.height, 40);
    assert_eq!(black_dots(&picture), 0);
}

#[test]
fn a_doc_too_big_to_draw_is_refused() {
    let fonts = load_fonts();
    let too_wide = Doc {
        width_dots: 100_000,
        blocks: vec![text("Hello")],
    };
    assert!(matches!(
        rasterize(&too_wide, &fonts),
        Err(PrintError::TooBig { .. })
    ));
    let too_tall = Doc {
        width_dots: 576,
        blocks: vec![Block::Feed { dots: u32::MAX }, text("Hello")],
    };
    assert!(matches!(
        rasterize(&too_tall, &fonts),
        Err(PrintError::TooBig { .. })
    ));
}

#[test]
fn columns_with_huge_weights_still_share_the_width() {
    let cells = (0..20)
        .map(|n| (format!("{n}"), u16::MAX, Align::Right))
        .collect();
    let doc = Doc {
        width_dots: 4096,
        blocks: vec![Block::Columns {
            cells,
            size: TextSize::Small,
            bold: false,
        }],
    };
    let picture = rasterize(&doc, &load_fonts()).unwrap();
    assert!(black_dots(&picture) > 0);
}

#[test]
fn hw_03_the_taka_sign_is_drawn() {
    // Noto Sans has no ৳, so the bundled Bengali currency-sign font must draw it.
    // Without it, the sign would come out blank (or as an empty box).
    let fonts = load_fonts();
    let draw = |words: &str| {
        let doc = Doc {
            width_dots: 200,
            blocks: vec![text(words)],
        };
        black_dots(&rasterize(&doc, &fonts).unwrap())
    };
    assert!(draw("৳") > 20);
    assert!(draw("₹") > 20);
    assert!(draw("€") > 20);
}

#[test]
fn hw_10_png_preview_has_the_same_dots_as_the_picture() {
    let fonts = load_fonts();
    let doc = receipt(
        &sample_sale(),
        &sample_layout(PaperWidth::Mm58, None),
        &english_labels(),
    );
    let picture = rasterize(&doc, &fonts).unwrap();
    let png = to_png(&picture).unwrap();
    let decoded = image::load_from_memory(&png).unwrap().to_luma8();
    assert_eq!(decoded.dimensions(), (picture.width, picture.height));
    for (x, y, pixel) in decoded.enumerate_pixels() {
        assert_eq!(pixel.0[0] == 0, is_black(&picture, x, y), "dot {x},{y}");
    }
}

#[test]
fn logo_is_black_and_white_and_fits_the_paper() {
    let shrunk = logo_from_image(&sample_logo_png(), 160).unwrap();
    assert_eq!(
        (shrunk.width, shrunk.height),
        (160, 53),
        "smaller, same shape"
    );

    let logo = logo_from_image(&sample_logo_png(), 576).unwrap();
    assert_eq!((logo.width, logo.height), (240, 80), "never made bigger");
    assert!(is_black(&logo, 1, 1), "the dark frame prints black");
    assert!(!is_black(&logo, 8, 8), "transparent parts print white");
    // The grey band is dithered: a mix of black and white dots.
    let band: Vec<bool> = (100..200).map(|x| is_black(&logo, x, 40)).collect();
    assert!(band.iter().any(|black| *black) && band.iter().any(|black| !*black));
}

#[test]
fn a_file_that_is_not_a_picture_is_a_logo_error_the_user_can_fix() {
    let error = logo_from_image(b"not a picture", 576).unwrap_err();
    assert!(matches!(error, PrintError::LogoUnreadable(_)));
    let app_error: AppError = error.into();
    assert!(matches!(
        app_error,
        AppError::Validation { ref code, .. } if code == "logo_unreadable"
    ));
}

#[test]
fn every_layout_makes_a_doc_at_its_width() {
    let labels = english_labels();
    let layout = sample_layout(PaperWidth::Mm80, None);
    assert_eq!(receipt(&sample_sale(), &layout, &labels).width_dots, 576);
    assert_eq!(
        return_receipt(&sample_return(), &layout, &labels).width_dots,
        576
    );
    assert_eq!(
        shift_report(&sample_z_report(), &layout, &labels).width_dots,
        576
    );
    // Labels and tags: 8 dots per mm (203 dpi).
    assert_eq!(
        label(&sample_label(), &label_template(58, 40), &labels).width_dots,
        464
    );
    assert_eq!(
        shelf_tag(&sample_shelf_tag(), &tag_template(80, 40), &labels).width_dots,
        640
    );
}

#[test]
fn label_size_in_dots() {
    let size = LabelSize {
        width_mm: 40,
        height_mm: 30,
        gap_mm: 2,
    };
    assert_eq!((size.width_dots(), size.height_dots()), (320, 240));
}

#[test]
fn hw_04_drawer_pulse_is_escpos_esc_p_with_its_two_timing_bytes() {
    // ESC p m t1 t2: pin, then 25 × 2 ms on and 250 × 2 ms off. Without t1 and t2
    // the printer would take the next two bytes it receives as the timings.
    assert_eq!(
        escpos::drawer_pulse(DrawerPin::Pin2).unwrap(),
        vec![0x1B, b'p', 0, 25, 250]
    );
    assert_eq!(
        escpos::drawer_pulse(DrawerPin::Pin5).unwrap(),
        vec![0x1B, b'p', 1, 25, 250]
    );
}

#[test]
fn escpos_starts_with_init_and_ends_with_cut() {
    let bytes = escpos::encode(&fake_barcode(16, 4), &EscPosOpts::default()).unwrap();
    assert_eq!(&bytes[..2], &[0x1B, b'@'], "ESC @ starts the printer fresh");
    assert_eq!(
        &bytes[bytes.len() - 4..],
        &[0x1D, b'V', b'A', 0],
        "GS V A 0 cuts"
    );
}

#[test]
fn a_picture_whose_data_doesnt_match_its_size_is_refused() {
    let broken = Bitmap1 {
        width: 16,
        height: 4,
        bits: vec![0; 3],
    };
    assert!(matches!(
        escpos::encode(&broken, &EscPosOpts::default()),
        Err(PrintError::BadPicture)
    ));
    assert!(matches!(to_png(&broken), Err(PrintError::BadPicture)));
}

#[test]
fn hw_06_tspl_has_size_gap_bitmap_and_print() {
    let size = LabelSize {
        width_mm: 40,
        height_mm: 30,
        gap_mm: 2,
    };
    let bytes = tspl::encode(&fake_barcode(320, 240), &size, 2);
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.starts_with("SIZE 40 mm,30 mm\r\nGAP 2 mm,0 mm\r\nCLS\r\nBITMAP 0,0,40,240,0,"));
    assert!(text.ends_with("\r\nPRINT 1,2\r\n"));
}

#[test]
fn internal_print_errors_become_internal_app_errors() {
    let app_error: AppError = PrintError::BadPicture.into();
    assert!(matches!(app_error, AppError::Internal { .. }));
}
