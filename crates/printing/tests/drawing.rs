//! Drawing rules every printout relies on: long names wrap, numbers line up on
//! the right, centred text is centred, and a long receipt draws quickly.

mod common;

use std::time::{Duration, Instant};

use common::*;
use pos_core::{Money, Qty};
use printing::{
    Align, Block, Doc, PaperWidth, ReceiptLine, TextSize, load_fonts, rasterize, receipt,
};

fn row(left: &str, right: &str) -> Block {
    Block::Row {
        left: left.into(),
        right: right.into(),
        size: TextSize::Normal,
        bold: false,
    }
}

/// The rightmost black dot in rows `from..to`, if any.
fn right_edge(picture: &pos_core::Bitmap1, from: u32, to: u32) -> Option<u32> {
    (0..picture.width)
        .rev()
        .find(|x| (from..to).any(|y| is_black(picture, *x, y)))
}

#[test]
fn hw_02_long_product_names_wrap_onto_more_lines() {
    let fonts = load_fonts();
    let short = Doc {
        width_dots: 384,
        blocks: vec![row("Milk 1L", "180.00")],
    };
    let long = Doc {
        width_dots: 384,
        blocks: vec![row(
            "Fresh Basmati Rice Premium Long Grain Family Pack 5kg",
            "650.00",
        )],
    };
    let short_height = rasterize(&short, &fonts).unwrap().height;
    let long_height = rasterize(&long, &fonts).unwrap().height;
    assert!(
        long_height >= 2 * short_height,
        "{long_height} vs {short_height}"
    );
}

#[test]
fn hw_02_a_word_longer_than_the_line_is_broken_not_cut_off() {
    let fonts = load_fonts();
    let doc = Doc {
        width_dots: 200,
        blocks: vec![Block::Text {
            text: "Supercalifragilisticexpialidocious".into(),
            size: TextSize::Normal,
            bold: false,
            align: Align::Left,
        }],
    };
    let picture = rasterize(&doc, &fonts).unwrap();
    assert!(picture.height > 40, "broken onto a second line");
}

#[test]
fn hw_02_numbers_line_up_on_the_right() {
    let fonts = load_fonts();
    let doc = Doc {
        width_dots: 576,
        blocks: vec![row("Milk 1L", "180.00"), row("Rice", "1,650.00")],
    };
    let picture = rasterize(&doc, &fonts).unwrap();
    let half = picture.height / 2;
    let first = right_edge(&picture, 0, half).unwrap();
    let second = right_edge(&picture, half, picture.height).unwrap();
    assert!(first >= 576 - 6, "close to the right edge: {first}");
    assert!(first.abs_diff(second) <= 2, "{first} vs {second}");
}

#[test]
fn hw_02_centred_text_is_in_the_middle() {
    let fonts = load_fonts();
    let doc = Doc {
        width_dots: 576,
        blocks: vec![Block::Text {
            text: "Total".into(),
            size: TextSize::Large,
            bold: true,
            align: Align::Center,
        }],
    };
    let picture = rasterize(&doc, &fonts).unwrap();
    let left = (0..576)
        .find(|x| (0..picture.height).any(|y| is_black(&picture, *x, y)))
        .unwrap();
    let right = right_edge(&picture, 0, picture.height).unwrap();
    assert!((left + right).abs_diff(576) <= 4, "{left}..{right}");
}

#[test]
fn nfr_p4_a_200_line_receipt_draws_within_200_ms() {
    let fonts = load_fonts();
    let mut sale = sample_sale();
    sale.lines = (0..200)
        .map(|n| ReceiptLine {
            name: format!("Product number {n} with a longer name"),
            qty: Qty(2000),
            unit: String::new(),
            unit_price: Money(12550 + n),
            amount: Money(25100 + 2 * n),
            discount: None,
        })
        .collect();
    let doc = receipt(
        &sale,
        &sample_layout(PaperWidth::Mm80, None),
        &english_labels(),
    );
    // The first picture after start-up also fills the letter cache; measure that one.
    let started = Instant::now();
    let picture = rasterize(&doc, &fonts).unwrap();
    let took = started.elapsed();
    assert!(picture.height > 200 * 40);
    // The budget is for the shipped (optimised) build, which takes about 30 ms.
    // Test builds without optimisation (`cargo nextest run`) run about 5 times
    // slower, so they get three times the budget. To check the real budget:
    // `cargo nextest run -p printing --cargo-profile release nfr_p4`.
    let budget = if cfg!(debug_assertions) {
        Duration::from_millis(600)
    } else {
        Duration::from_millis(200)
    };
    assert!(took <= budget, "took {took:?}");
}
