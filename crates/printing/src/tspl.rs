//! TSPL bytes for TSC-style label printers (rule HW-06). No maintained Rust crate
//! writes TSPL, so these few commands are written here (see the Reuse Map).

use pos_core::Bitmap1;

use crate::doc::LabelSize;
use crate::pictures::is_black;

/// Turns a picture into TSPL: label size and gap, clear, the picture, print.
/// Rows below the label's height are left out, so they never spill onto the next label.
/// `copies` of 0 prints one.
pub fn encode(b: &Bitmap1, label: &LabelSize, copies: u32) -> Vec<u8> {
    let width_bytes = b.width.div_ceil(8);
    let rows = b.height.min(label.height_dots());

    let mut bytes = Vec::new();
    bytes.extend_from_slice(
        format!(
            "SIZE {} mm,{} mm\r\nGAP {} mm,0 mm\r\nCLS\r\nBITMAP 0,0,{width_bytes},{rows},0,",
            label.width_mm, label.height_mm, label.gap_mm
        )
        .as_bytes(),
    );
    for y in 0..rows {
        for byte_index in 0..width_bytes {
            // In TSPL a 0 bit prints a black dot, the opposite of ESC/POS.
            let mut byte = 0xFF_u8;
            for bit in 0..8 {
                if is_black(b, byte_index * 8 + bit, y) {
                    byte &= !(0x80 >> bit);
                }
            }
            bytes.push(byte);
        }
    }
    bytes.extend_from_slice(format!("\r\nPRINT 1,{}\r\n", copies.max(1)).as_bytes());
    bytes
}
