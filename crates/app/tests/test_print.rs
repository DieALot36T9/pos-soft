//! "Test print" and the Preview printer (rules HW-02, HW-03, HW-08, HW-10).
//!
//! The test receipt must match its approved picture in `tests/golden/` dot for dot.
//! A changed picture needs the product owner's approval. When a picture differs,
//! the test saves the new one as `<name>.new.png` (not committed) so you can compare.
//! After approval: `UPDATE_GOLDEN=1 cargo nextest run -p app`.

use std::path::PathBuf;

use app::devices::Devices;
use app::preview;
use app::workflows::test_print;
use hardware::{HwError, MockPrinter, PrinterConfig, printer_from};
use pos_core::{AppError, Bitmap1};
use settings::PaperWidth;

fn golden_path(name: &str, ending: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(format!("{name}{ending}"))
}

/// Compares the picture with the approved one, or saves it when UPDATE_GOLDEN=1.
fn check_golden(name: &str, picture: &Bitmap1) {
    let png = printing::to_png(picture).expect("the picture saves as PNG");
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
    if &approved != picture {
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

#[test]
fn hw_02_test_receipt_80mm_matches_the_approved_picture() {
    let devices = Devices::new(None);
    let picture = test_print::picture(PaperWidth::Mm80, &devices.fonts).expect("it draws");
    assert_eq!(picture.width, 576);
    check_golden("test_receipt_80mm", &picture);
}

#[test]
fn hw_02_test_receipt_58mm_matches_the_approved_picture() {
    let devices = Devices::new(None);
    let picture = test_print::picture(PaperWidth::Mm58, &devices.fonts).expect("it draws");
    assert_eq!(picture.width, 384);
    check_golden("test_receipt_58mm", &picture);
}

#[test]
fn hw_10_test_print_on_the_preview_printer_shows_the_exact_picture() {
    let devices = Devices::new(None);
    let printer = printer_from(&PrinterConfig::Preview, &devices.preview);

    test_print::run(printer.as_ref(), PaperWidth::Mm80, &devices.fonts).expect("it prints");

    let jobs = devices.preview.take_jobs();
    assert_eq!(jobs.len(), 1);
    let expected = test_print::picture(PaperWidth::Mm80, &devices.fonts).expect("it draws");
    assert_eq!(jobs[0].pictures, vec![expected]);
    // A test print never opens the cash drawer.
    assert!(!jobs[0].drawer_opened);
}

#[test]
fn hw_08_a_printer_that_fails_gives_a_hardware_error() {
    let devices = Devices::new(None);
    let printer = MockPrinter::new();
    printer.fail_next(HwError::NotConnected {
        device: hardware::Device::Printer,
        detail: "unplugged".into(),
    });

    let result = test_print::run(&printer, PaperWidth::Mm58, &devices.fonts);

    match result {
        Err(AppError::Hardware { device, code, .. }) => {
            assert_eq!(device, "printer");
            assert_eq!(code, "not_connected");
        }
        other => panic!("expected a hardware error, got {other:?}"),
    }
}

#[test]
fn hw_10_printouts_come_as_png_pictures_and_are_saved() {
    let folder = tempfile::tempdir().expect("a temporary folder");
    let devices = Devices::new(Some(folder.path().to_path_buf()));
    let printer = printer_from(&PrinterConfig::Preview, &devices.preview);
    test_print::run(printer.as_ref(), PaperWidth::Mm58, &devices.fonts).expect("it prints");

    let printouts = preview::take_printouts(&devices).expect("the printouts are read");

    assert_eq!(printouts.len(), 1);
    let printout = &printouts[0];
    assert!(!printout.drawer_opened);
    assert_eq!(printout.pictures.len(), 1);
    assert!(printout.pictures[0].starts_with("data:image/png;base64,"));
    assert_eq!(printout.saved_files.len(), 1);
    let saved = std::fs::read(&printout.saved_files[0]).expect("the PNG was saved");
    let expected = test_print::picture(PaperWidth::Mm58, &devices.fonts).expect("it draws");
    assert_eq!(from_png(&saved), expected);
    // Each printout is handed out once.
    assert!(
        preview::take_printouts(&devices)
            .expect("read again")
            .is_empty()
    );
}

#[test]
fn hw_10_a_drawer_pulse_shows_in_the_printout() {
    let devices = Devices::new(None);
    let printer = printer_from(&PrinterConfig::Preview, &devices.preview);
    let bytes = printing::escpos::drawer_pulse(printing::DrawerPin::Pin2).expect("pulse bytes");
    printer.send(&bytes).expect("sent");

    let printouts = preview::take_printouts(&devices).expect("the printouts are read");

    assert_eq!(printouts.len(), 1);
    assert!(printouts[0].drawer_opened);
    assert!(printouts[0].pictures.is_empty());
}
