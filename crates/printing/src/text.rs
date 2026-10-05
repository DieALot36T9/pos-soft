//! Text: `cosmic-text` shapes it with our font and breaks it into lines; `swash`
//! (inside `cosmic-text`) draws each letter, and we copy the letters onto the page.

use cosmic_text::{
    Attrs, Buffer, Family, FontSystem, Hinting, Metrics, Shaping, SwashCache, SwashContent,
    SwashImage, Weight,
};
use tiny_skia::Mask;

use crate::doc::{Align, TextSize};
use crate::fonts::FAMILY;

/// Shapes text with our font. `width` limits the line length (and so wraps the
/// text); `None` keeps it on one line.
pub(crate) fn shape(
    system: &mut FontSystem,
    text: &str,
    size: TextSize,
    bold: bool,
    width: Option<u32>,
    align: Align,
) -> Buffer {
    let metrics = Metrics::new(size.font_dots() as f32, size.line_dots() as f32);
    let mut buffer = Buffer::new_empty(metrics);
    // Puts letters on whole dots, which keeps small text sharp on a printer.
    buffer.set_hinting(Hinting::Enabled);
    buffer.set_size(width.map(|dots| dots as f32), None);
    let weight = if bold { Weight::BOLD } else { Weight::NORMAL };
    let attrs = Attrs::new().family(Family::Name(FAMILY)).weight(weight);
    let line_align = match align {
        Align::Left => cosmic_text::Align::Left,
        Align::Center => cosmic_text::Align::Center,
        Align::Right => cosmic_text::Align::Right,
    };
    buffer.set_text(text, &attrs, Shaping::Advanced, Some(line_align));
    buffer.shape_until_scroll(system, false);
    buffer
}

/// The height of shaped text: its number of lines times the line height.
pub(crate) fn text_height(buffer: &Buffer, size: TextSize) -> u32 {
    buffer.layout_runs().count() as u32 * size.line_dots()
}

/// The width of the longest line of shaped text, in whole dots.
pub(crate) fn text_width(buffer: &Buffer) -> u32 {
    buffer
        .layout_runs()
        .map(|run| run.line_w.ceil() as u32)
        .max()
        .unwrap_or(0)
}

/// Draws shaped text onto the page with its top-left corner at (x, y).
/// The page holds how dark each dot is (0 white to 255 black); where letters
/// touch, the darker value stays.
pub(crate) fn draw_text(
    page: &mut Mask,
    buffer: &Buffer,
    x: i32,
    y: i32,
    system: &mut FontSystem,
    swash: &mut SwashCache,
) {
    let page_width = page.width() as i32;
    let page_height = page.height() as i32;
    let dots = page.data_mut();
    for run in buffer.layout_runs() {
        for glyph in run.glyphs {
            let placed = glyph.physical((0., run.line_y), 1.0);
            // `swash` draws each letter once and keeps it in its cache.
            let Some(letter) = swash.get_image(system, placed.cache_key) else {
                continue;
            };
            let left = x + placed.x + letter.placement.left;
            let top = y + placed.y - letter.placement.top;
            copy_letter(dots, page_width, page_height, letter, left, top);
        }
    }
}

/// Copies one drawn letter onto the page with its top-left corner at (left, top).
/// Parts that fall off the page are skipped.
fn copy_letter(
    dots: &mut [u8],
    page_width: i32,
    page_height: i32,
    letter: &SwashImage,
    left: i32,
    top: i32,
) {
    // Bytes per dot, and where the darkness is: a plain mask, or RGBA (emoji).
    let (step, darkness_at) = match letter.content {
        SwashContent::Mask => (1, 0),
        SwashContent::Color => (4, 3),
        SwashContent::SubpixelMask => return,
    };
    let letter_width = letter.placement.width as i32;
    let first_column = (-left).max(0);
    let end_column = letter_width.min(page_width - left);
    if first_column >= end_column {
        return;
    }
    for row in 0..letter.placement.height as i32 {
        let page_y = top + row;
        if page_y < 0 || page_y >= page_height {
            continue;
        }
        let page_start = (page_y * page_width + left + first_column) as usize;
        let page_end = page_start + (end_column - first_column) as usize;
        let letter_start = ((row * letter_width + first_column) * step + darkness_at) as usize;
        let (Some(page_dots), Some(letter_dots)) = (
            dots.get_mut(page_start..page_end),
            letter.data.get(letter_start..),
        ) else {
            continue;
        };
        for (dot, darkness) in page_dots
            .iter_mut()
            .zip(letter_dots.iter().step_by(step as usize))
        {
            if *darkness > *dot {
                *dot = *darkness;
            }
        }
    }
}
