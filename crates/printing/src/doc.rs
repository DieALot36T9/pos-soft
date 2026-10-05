//! The document model: a printout described as a list of simple blocks, top to
//! bottom. Layouts build a `Doc`; `rasterize` draws it.

use pos_core::Bitmap1;

/// A printout before it is drawn: its width in printer dots and its blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc {
    /// 384 for 58 mm paper, 576 for 80 mm paper, or a label's width.
    pub width_dots: u32,
    pub blocks: Vec<Block>,
}

/// One part of a printout. Blocks are drawn one under the other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// Text that wraps onto more lines when it is too long.
    Text {
        text: String,
        size: TextSize,
        bold: bool,
        align: Align,
    },
    /// Text on the left and a value on the right. Example: "Milk 1L      180.00".
    /// The left text wraps; the right value stays on the first line.
    Row {
        left: String,
        right: String,
        size: TextSize,
        bold: bool,
    },
    /// A table row. Each cell is (text, weight, alignment); a cell's share of the
    /// width is its weight divided by the sum of all weights.
    Columns {
        cells: Vec<(String, u16, Align)>,
        size: TextSize,
        bold: bool,
    },
    /// A line across the paper: `----` (single) or `====` (double).
    Rule { style: RuleStyle },
    /// A black-and-white picture, such as the shop logo.
    Image { bitmap: Bitmap1, align: Align },
    /// A barcode picture, centred. `app` gets it from the `barcode` crate.
    Barcode { bitmap: Bitmap1 },
    /// A QR code, centred, about `size_dots` wide.
    Qr { data: String, size_dots: u32 },
    /// Blank space, in dots.
    Feed { dots: u32 },
}

/// Text sizes. The same size in dots on every paper width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSize {
    /// 18 dots: details such as "2 × 90.00".
    Small,
    /// 22 dots: about 32 characters on 58 mm paper and 48 on 80 mm.
    Normal,
    /// 30 dots: the total and headings.
    Large,
    /// 56 dots: the price on a shelf tag.
    Huge,
}

impl TextSize {
    /// The font size in dots.
    pub(crate) fn font_dots(self) -> u32 {
        match self {
            TextSize::Small => 18,
            TextSize::Normal => 22,
            TextSize::Large => 30,
            TextSize::Huge => 56,
        }
    }

    /// The height of one line of text, with a little space between lines.
    pub(crate) fn line_dots(self) -> u32 {
        self.font_dots() * 5 / 4
    }
}

/// Where text or a picture sits across the paper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// How a `Rule` looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleStyle {
    /// A dashed line: `----`.
    Single,
    /// Two solid lines: `====`.
    Double,
}

/// Receipt paper width (rule HW-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperWidth {
    Mm58,
    Mm80,
}

impl PaperWidth {
    /// The printable width in dots: 384 for 58 mm paper, 576 for 80 mm paper.
    pub fn dots(self) -> u32 {
        match self {
            PaperWidth::Mm58 => 384,
            PaperWidth::Mm80 => 576,
        }
    }
}

/// The size of a sticky label or shelf tag, in whole millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelSize {
    pub width_mm: u32,
    pub height_mm: u32,
    /// The gap between two labels on the roll. Example: 2
    pub gap_mm: u32,
}

/// Label printers print 8 dots per millimetre (203 dots per inch).
pub(crate) const DOTS_PER_MM: u32 = 8;

impl LabelSize {
    /// The label's width in printer dots (8 per millimetre).
    pub fn width_dots(&self) -> u32 {
        self.width_mm * DOTS_PER_MM
    }

    /// The label's height in printer dots (8 per millimetre).
    pub fn height_dots(&self) -> u32 {
        self.height_mm * DOTS_PER_MM
    }
}
