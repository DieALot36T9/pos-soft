//! A black-and-white picture. `barcode` draws it; `printing` prints it.

/// A 1-bit picture. Each row takes `(width + 7) / 8` bytes. The first pixel of a
/// row is the highest bit of its first byte. A set bit (1) is black.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitmap1 {
    pub width: u32,
    pub height: u32,
    pub bits: Vec<u8>,
}

impl Bitmap1 {
    /// An all-white picture of the given size.
    pub fn new(width: u32, height: u32) -> Bitmap1 {
        let bytes_per_row = width.div_ceil(8) as usize;
        Bitmap1 {
            width,
            height,
            bits: vec![0; bytes_per_row * height as usize],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_padded_to_whole_bytes() {
        let picture = Bitmap1::new(10, 3);
        assert_eq!(picture.bits.len(), 2 * 3);
        assert!(picture.bits.iter().all(|byte| *byte == 0));
    }
}
