//! Draws a `Doc` as a black-and-white picture (ADR-012). First every block gets its
//! place on the page; then text and lines are drawn onto a `tiny-skia` mask (how
//! dark each dot is); then the page is cut to 1 bit and the pictures are copied in.

use cosmic_text::{Buffer, FontSystem};
use pos_core::Bitmap1;
use tiny_skia::{FillRule, Mask, PathBuilder, Rect, Transform};

use crate::PrintError;
use crate::doc::{Align, Block, Doc, RuleStyle, TextSize};
use crate::fonts::{FontTools, Fonts};
use crate::pictures::{bytes_per_row, check_picture, is_black, qr_picture, set_black};
use crate::text::{draw_text, shape, text_height, text_width};

/// Space between a row's text and its value, and between table columns.
const GAP_DOTS: u32 = 12;
/// Height of a `Rule` block. The line sits in the middle.
const RULE_DOTS: u32 = 12;
/// No real printout comes near these; they stop a bad `Doc` from using all memory.
const MAX_WIDTH_DOTS: u32 = 4096;
const MAX_HEIGHT_DOTS: u32 = 32_000;
/// A drawn dot at least this dark (out of 255) prints black.
const BLACK_FROM: u8 = 128;

/// Something to draw, at its final place on the page.
enum Piece {
    Text {
        buffer: Buffer,
        x: i32,
        y: i32,
    },
    Box {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    Picture {
        picture: Bitmap1,
        x: u32,
        y: u32,
    },
}

/// Draws the doc as a black-and-white picture, `doc.width_dots` wide and as tall as
/// its blocks need.
pub fn rasterize(doc: &Doc, fonts: &Fonts) -> Result<Bitmap1, PrintError> {
    let mut tools = fonts.lock();
    let FontTools { system, swash } = &mut *tools;

    let width = doc.width_dots;
    if width > MAX_WIDTH_DOTS {
        return Err(PrintError::TooBig { width, height: 0 });
    }
    let mut pieces = Vec::new();
    let mut height: u32 = 0;
    for block in &doc.blocks {
        let block_height = place_block(block, width, height, system, &mut pieces)?;
        height = height.saturating_add(block_height);
        if height > MAX_HEIGHT_DOTS {
            return Err(PrintError::TooBig { width, height });
        }
    }
    if width == 0 || height == 0 {
        return Ok(Bitmap1::new(width, height));
    }

    let mut page = Mask::new(width, height).ok_or(PrintError::TooBig { width, height })?;
    for piece in &pieces {
        match piece {
            Piece::Text { buffer, x, y } => draw_text(&mut page, buffer, *x, *y, system, swash),
            Piece::Box {
                x,
                y,
                width,
                height,
            } => draw_box(&mut page, *x, *y, *width, *height),
            Piece::Picture { .. } => {}
        }
    }

    let mut picture = to_one_bit(&page);
    // Pictures are already black and white, so they go straight into the result.
    for piece in &pieces {
        if let Piece::Picture {
            picture: part,
            x,
            y,
        } = piece
        {
            copy_black_dots(&mut picture, part, *x, *y);
        }
    }
    Ok(picture)
}

/// Works out where a block's parts go, starting at `top`. Returns the block's height.
fn place_block(
    block: &Block,
    width: u32,
    top: u32,
    system: &mut FontSystem,
    pieces: &mut Vec<Piece>,
) -> Result<u32, PrintError> {
    let height = match block {
        Block::Text {
            text,
            size,
            bold,
            align,
        } => {
            let buffer = shape(system, text, *size, *bold, Some(width), *align);
            let height = text_height(&buffer, *size);
            pieces.push(Piece::Text {
                buffer,
                x: 0,
                y: top as i32,
            });
            height
        }
        Block::Row {
            left,
            right,
            size,
            bold,
        } => place_row(left, right, *size, *bold, width, top, system, pieces),
        Block::Columns { cells, size, bold } => {
            place_columns(cells, *size, *bold, width, top, system, pieces)
        }
        Block::Rule { style } => {
            place_rule(*style, width, top, pieces);
            RULE_DOTS
        }
        Block::Image { bitmap, align } => {
            place_picture(bitmap.clone(), *align, width, top, pieces)?
        }
        Block::Barcode { bitmap } => {
            place_picture(bitmap.clone(), Align::Center, width, top, pieces)?
        }
        Block::Qr { data, size_dots } => place_picture(
            qr_picture(data, *size_dots)?,
            Align::Center,
            width,
            top,
            pieces,
        )?,
        Block::Feed { dots } => *dots,
    };
    Ok(height)
}

/// Text on the left, a value on the right. The left text wraps in the space the
/// value leaves; the value stays on the first line.
#[allow(clippy::too_many_arguments)] // all plain values; a struct would only add noise
fn place_row(
    left: &str,
    right: &str,
    size: TextSize,
    bold: bool,
    width: u32,
    top: u32,
    system: &mut FontSystem,
    pieces: &mut Vec<Piece>,
) -> u32 {
    let right_buffer = shape(system, right, size, bold, None, Align::Left);
    let right_width = text_width(&right_buffer);
    let left_width = if right.is_empty() {
        width
    } else {
        // Never squeeze the left text below a third of the line.
        width.saturating_sub(right_width + GAP_DOTS).max(width / 3)
    };
    let left_buffer = shape(system, left, size, bold, Some(left_width), Align::Left);
    let height = text_height(&left_buffer, size).max(text_height(&right_buffer, size));
    pieces.push(Piece::Text {
        buffer: left_buffer,
        x: 0,
        y: top as i32,
    });
    pieces.push(Piece::Text {
        buffer: right_buffer,
        x: width.saturating_sub(right_width) as i32,
        y: top as i32,
    });
    height
}

/// A table row: each cell gets its share of the width by weight.
fn place_columns(
    cells: &[(String, u16, Align)],
    size: TextSize,
    bold: bool,
    width: u32,
    top: u32,
    system: &mut FontSystem,
    pieces: &mut Vec<Piece>,
) -> u32 {
    // A weight of 0 counts as 1, so every cell gets some room. Worked out in u64,
    // because width × weight can be bigger than a u32 holds.
    let weights: Vec<u64> = cells
        .iter()
        .map(|(_, weight, _)| u64::from(*weight).max(1))
        .collect();
    let total_weight: u64 = weights.iter().sum();
    let mut weight_so_far = 0;
    let mut height = 0;
    for (index, (text, _, align)) in cells.iter().enumerate() {
        // Each share is at most `width`, so it fits back into a u32.
        let start = (u64::from(width) * weight_so_far / total_weight) as u32;
        weight_so_far += weights[index];
        let end = (u64::from(width) * weight_so_far / total_weight) as u32;
        let is_last = index + 1 == cells.len();
        let cell_width = if is_last {
            end - start
        } else {
            (end - start).saturating_sub(GAP_DOTS)
        };
        let buffer = shape(system, text, size, bold, Some(cell_width), *align);
        height = height.max(text_height(&buffer, size));
        pieces.push(Piece::Text {
            buffer,
            x: start as i32,
            y: top as i32,
        });
    }
    height
}

/// A dashed line (single) or two solid lines (double), 2 dots thick.
fn place_rule(style: RuleStyle, width: u32, top: u32, pieces: &mut Vec<Piece>) {
    match style {
        RuleStyle::Single => {
            for x in (0..width).step_by(10) {
                pieces.push(Piece::Box {
                    x,
                    y: top + 5,
                    width: 6.min(width - x),
                    height: 2,
                });
            }
        }
        RuleStyle::Double => {
            for y in [top + 3, top + 7] {
                pieces.push(Piece::Box {
                    x: 0,
                    y,
                    width,
                    height: 2,
                });
            }
        }
    }
}

/// A black-and-white picture. Returns its height.
fn place_picture(
    picture: Bitmap1,
    align: Align,
    width: u32,
    top: u32,
    pieces: &mut Vec<Piece>,
) -> Result<u32, PrintError> {
    check_picture(&picture)?;
    let room = width.saturating_sub(picture.width);
    let x = match align {
        Align::Left => 0,
        Align::Center => room / 2,
        Align::Right => room,
    };
    let height = picture.height;
    pieces.push(Piece::Picture { picture, x, y: top });
    Ok(height)
}

/// Fills a black box. Whole dots only, so no soft edges.
fn draw_box(page: &mut Mask, x: u32, y: u32, width: u32, height: u32) {
    let Some(rect) = Rect::from_xywh(x as f32, y as f32, width as f32, height as f32) else {
        return;
    };
    let path = PathBuilder::from_rect(rect);
    page.fill_path(&path, FillRule::Winding, false, Transform::identity());
}

/// Cuts the page to black and white: dots at least half dark print black.
fn to_one_bit(page: &Mask) -> Bitmap1 {
    let mut picture = Bitmap1::new(page.width(), page.height());
    let row_bytes = bytes_per_row(&picture);
    let page_rows = page.data().chunks_exact(page.width() as usize);
    for (row, picture_row) in page_rows.zip(picture.bits.chunks_exact_mut(row_bytes)) {
        for (eight_dots, byte) in row.chunks(8).zip(picture_row.iter_mut()) {
            // Most of a printout is white paper: skip eight white dots at once.
            if eight_dots == [0; 8] {
                continue;
            }
            for (bit, darkness) in eight_dots.iter().enumerate() {
                if *darkness >= BLACK_FROM {
                    *byte |= 0x80 >> bit;
                }
            }
        }
    }
    picture
}

/// Copies the black dots of `part` into `picture`, with its top-left corner at (x, y).
fn copy_black_dots(picture: &mut Bitmap1, part: &Bitmap1, x: u32, y: u32) {
    for part_y in 0..part.height {
        for part_x in 0..part.width {
            if is_black(part, part_x, part_y) {
                set_black(picture, x + part_x, y + part_y);
            }
        }
    }
}
