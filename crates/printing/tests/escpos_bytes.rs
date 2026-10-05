//! ESC/POS and TSPL bytes (rules HW-02, HW-04, HW-06). Byte snapshots are kept
//! by `insta` in `tests/snapshots/`. A changed snapshot needs a careful look:
//! these bytes go straight to the printer.

mod common;

use common::*;
use pos_core::Bitmap1;
use printing::{DrawerPin, EscPosOpts, LabelSize, escpos, tspl};

/// Shows bytes as hex, 16 to a line, so a snapshot is easy to read.
fn hex(bytes: &[u8]) -> String {
    bytes
        .chunks(16)
        .map(|chunk| {
            chunk
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A 12×3 picture: a black top row, a checked middle row, a white bottom row.
fn tiny_picture() -> Bitmap1 {
    Bitmap1 {
        width: 12,
        height: 3,
        bits: vec![0xFF, 0xF0, 0xAA, 0xA0, 0x00, 0x00],
    }
}

#[test]
fn hw_02_header_bytes_init_then_gs_v_0_with_size() {
    let opts = EscPosOpts {
        cut: false,
        drawer: None,
        band_height: 256,
    };
    let bytes = escpos::encode(&tiny_picture(), &opts).unwrap();
    // ESC @, then GS v 0, mode 0, 2 bytes wide, 3 rows high, then the rows as they are.
    assert_eq!(
        bytes,
        vec![
            0x1B, 0x40, 0x1D, 0x76, 0x30, 0x00, 0x02, 0x00, 0x03, 0x00, 0xFF, 0xF0, 0xAA, 0xA0,
            0x00, 0x00
        ]
    );
    insta::assert_snapshot!("header_no_cut", hex(&bytes));
}

#[test]
fn hw_02_band_splitting_sends_the_picture_in_strips() {
    let opts = EscPosOpts {
        cut: false,
        drawer: None,
        band_height: 2,
    };
    let bytes = escpos::encode(&tiny_picture(), &opts).unwrap();
    // Two strips: rows 1–2, then row 3.
    assert_eq!(
        bytes,
        vec![
            0x1B, 0x40, //
            0x1D, 0x76, 0x30, 0x00, 0x02, 0x00, 0x02, 0x00, 0xFF, 0xF0, 0xAA, 0xA0, //
            0x1D, 0x76, 0x30, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00,
        ]
    );
    insta::assert_snapshot!("two_bands", hex(&bytes));
}

#[test]
fn hw_02_a_tall_picture_needs_the_two_byte_height() {
    let picture = Bitmap1::new(576, 300);
    let opts = EscPosOpts {
        cut: false,
        drawer: None,
        band_height: 1000,
    };
    let bytes = escpos::encode(&picture, &opts).unwrap();
    // 72 bytes wide (0x48 0x00), 300 rows high (0x2C 0x01).
    assert_eq!(
        &bytes[2..10],
        &[0x1D, 0x76, 0x30, 0x00, 0x48, 0x00, 0x2C, 0x01]
    );
    assert_eq!(bytes.len(), 2 + 8 + 72 * 300);
}

#[test]
fn a_band_height_of_zero_is_treated_as_one_row() {
    let opts = EscPosOpts {
        cut: false,
        drawer: None,
        band_height: 0,
    };
    let bytes = escpos::encode(&tiny_picture(), &opts).unwrap();
    assert_eq!(bytes.len(), 2 + 3 * (8 + 2), "three strips of one row");
}

#[test]
fn hw_02_cut_after_the_picture() {
    let opts = EscPosOpts {
        cut: true,
        drawer: None,
        band_height: 256,
    };
    let bytes = escpos::encode(&tiny_picture(), &opts).unwrap();
    assert_eq!(&bytes[bytes.len() - 4..], &[0x1D, 0x56, 0x41, 0x00]);
    insta::assert_snapshot!("with_cut", hex(&bytes));
}

#[test]
fn hw_04_drawer_opens_after_the_cut() {
    let opts = EscPosOpts {
        cut: true,
        drawer: Some(DrawerPin::Pin5),
        band_height: 256,
    };
    let bytes = escpos::encode(&tiny_picture(), &opts).unwrap();
    insta::assert_snapshot!("with_cut_and_drawer_pin_5", hex(&bytes));
    let cut_at = bytes
        .windows(4)
        .position(|window| window == [0x1D, 0x56, 0x41, 0x00])
        .expect("a cut");
    let drawer_at = bytes
        .windows(3)
        .position(|window| window == [0x1B, b'p', 1])
        .expect("a drawer pulse");
    assert!(drawer_at > cut_at);
}

#[test]
fn hw_04_drawer_pulse_bytes() {
    insta::assert_snapshot!(
        "drawer_pin_2",
        hex(&escpos::drawer_pulse(DrawerPin::Pin2).unwrap())
    );
    insta::assert_snapshot!(
        "drawer_pin_5",
        hex(&escpos::drawer_pulse(DrawerPin::Pin5).unwrap())
    );
}

#[test]
fn hw_06_tspl_label_bytes() {
    let size = LabelSize {
        width_mm: 2,
        height_mm: 1,
        gap_mm: 2,
    };
    // A 16×8 picture fits a 2×1 mm label (8 dots per mm).
    let mut picture = fake_barcode(16, 8);
    picture.bits[0] = 0xF0;
    let bytes = tspl::encode(&picture, &size, 1);
    let (head, rest) = bytes.split_at(bytes.len() - 13 - 16);
    // Line ends shown as `\r\n`, because TSPL needs both bytes and insta ignores `\r`.
    let commands = String::from_utf8_lossy(head).replace("\r\n", "\\r\\n\n");
    insta::assert_snapshot!("tspl_2x1mm", commands);
    // In TSPL a 0 bit prints black, so the bits are flipped.
    assert_eq!(rest[0], !0xF0);
    assert_eq!(&rest[16..], b"\r\nPRINT 1,1\r\n");
}

#[test]
fn hw_06_tspl_crops_rows_below_the_label() {
    let size = LabelSize {
        width_mm: 2,
        height_mm: 1,
        gap_mm: 2,
    };
    let bytes = tspl::encode(&fake_barcode(16, 20), &size, 1);
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("BITMAP 0,0,2,8,0,"),
        "only the 8 rows that fit"
    );
}

#[test]
fn hw_06_tspl_prints_at_least_one_copy() {
    let size = LabelSize {
        width_mm: 2,
        height_mm: 1,
        gap_mm: 2,
    };
    let bytes = tspl::encode(&fake_barcode(16, 8), &size, 0);
    assert!(bytes.ends_with(b"PRINT 1,1\r\n"));
}
