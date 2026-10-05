//! Layouts: turn receipt, report and label data into a `Doc`. The data structs are
//! owned here; `app` fills them from sales, returns, reports and products.
//! Layouts only arrange and format. Every amount arrives already worked out.

pub(crate) mod label;
pub(crate) mod receipt;
pub(crate) mod receipt_data;
pub(crate) mod report;

use pos_core::Bitmap1;

use crate::doc::{Align, Block, RuleStyle, TextSize};

/// Space under the last block, so the cutter doesn't cut through the footer.
const BOTTOM_FEED_DOTS: u32 = 24;

/// Text in the given size and alignment.
fn text(text: impl Into<String>, size: TextSize, bold: bool, align: Align) -> Block {
    Block::Text {
        text: text.into(),
        size,
        bold,
        align,
    }
}

/// Small centred text, for header and footer lines.
fn small_centered(words: impl Into<String>) -> Block {
    text(words, TextSize::Small, false, Align::Center)
}

/// "Label      value" in the given size.
fn row(left: impl Into<String>, right: impl Into<String>, size: TextSize, bold: bool) -> Block {
    Block::Row {
        left: left.into(),
        right: right.into(),
        size,
        bold,
    }
}

fn rule(style: RuleStyle) -> Block {
    Block::Rule { style }
}

fn feed(dots: u32) -> Block {
    Block::Feed { dots }
}

/// A barcode picture with its digits under it, if there are any.
fn barcode_with_digits(blocks: &mut Vec<Block>, barcode: &Bitmap1, digits: &str) {
    blocks.push(Block::Barcode {
        bitmap: barcode.clone(),
    });
    if !digits.is_empty() {
        blocks.push(small_centered(digits));
    }
}
