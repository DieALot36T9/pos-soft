//! Black-and-white pictures: checking them, PNG previews, the shop logo and QR codes.

use std::io::Cursor;

use image::codecs::png::PngEncoder;
use image::imageops::{self, BiLevel, FilterType};
use image::{GrayImage, ImageEncoder, Luma};
use pos_core::Bitmap1;
use qrcode::{Color, QrCode};

use crate::PrintError;

/// White border around a QR code, in squares. The QR standard asks for 4.
const QR_QUIET_SQUARES: usize = 4;

/// Bytes in one row of the picture.
pub(crate) fn bytes_per_row(picture: &Bitmap1) -> usize {
    picture.width.div_ceil(8) as usize
}

/// Refuses a picture whose `bits` don't match its width and height.
pub(crate) fn check_picture(picture: &Bitmap1) -> Result<(), PrintError> {
    let expected = bytes_per_row(picture).checked_mul(picture.height as usize);
    if expected == Some(picture.bits.len()) {
        Ok(())
    } else {
        Err(PrintError::BadPicture)
    }
}

/// Is the dot at (x, y) black? Dots outside the picture are white.
pub(crate) fn is_black(picture: &Bitmap1, x: u32, y: u32) -> bool {
    if x >= picture.width || y >= picture.height {
        return false;
    }
    let index = y as usize * bytes_per_row(picture) + x as usize / 8;
    let byte = picture.bits.get(index).copied().unwrap_or(0);
    byte & (0x80 >> (x % 8)) != 0
}

/// Makes the dot at (x, y) black. Dots outside the picture are ignored.
pub(crate) fn set_black(picture: &mut Bitmap1, x: u32, y: u32) {
    if x >= picture.width || y >= picture.height {
        return;
    }
    let index = y as usize * bytes_per_row(picture) + x as usize / 8;
    if let Some(byte) = picture.bits.get_mut(index) {
        *byte |= 0x80 >> (x % 8);
    }
}

/// Saves the picture as a PNG, for previews on screen and golden tests (rule HW-10).
/// The preview is exactly the dots that print: black is 0, white is 255.
pub fn to_png(picture: &Bitmap1) -> Result<Vec<u8>, PrintError> {
    check_picture(picture)?;
    let mut gray = GrayImage::from_pixel(picture.width, picture.height, Luma([255]));
    for (x, y, pixel) in gray.enumerate_pixels_mut() {
        if is_black(picture, x, y) {
            *pixel = Luma([0]);
        }
    }
    let mut png = Cursor::new(Vec::new());
    PngEncoder::new(&mut png)
        .write_image(
            gray.as_raw(),
            picture.width,
            picture.height,
            image::ExtendedColorType::L8,
        )
        .map_err(|error| PrintError::Png(error.to_string()))?;
    Ok(png.into_inner())
}

/// Turns the shop's logo file (PNG or JPEG) into a black-and-white picture at most
/// `max_width_dots` wide. Greys become a pattern of dots (Floyd–Steinberg
/// dithering), and transparent parts print white. Settings shows the result as the
/// logo preview (rule SET-04).
pub fn logo_from_image(file: &[u8], max_width_dots: u32) -> Result<Bitmap1, PrintError> {
    let mut picture = image::load_from_memory(file)
        .map_err(|error| PrintError::LogoUnreadable(error.to_string()))?;
    if picture.width() > max_width_dots {
        // Keeps the shape: the height shrinks by the same amount.
        picture = picture.resize(max_width_dots, u32::MAX, FilterType::Triangle);
    }

    let with_alpha = picture.to_luma_alpha8();
    let mut gray = GrayImage::new(with_alpha.width(), with_alpha.height());
    for (x, y, pixel) in with_alpha.enumerate_pixels() {
        let [shade, alpha] = pixel.0;
        // Lay the logo on white paper: a see-through dot shows the paper.
        let on_paper = (u32::from(shade) * u32::from(alpha) + 255 * (255 - u32::from(alpha))) / 255;
        gray.put_pixel(x, y, Luma([on_paper as u8]));
    }
    imageops::dither(&mut gray, &BiLevel);

    let mut logo = Bitmap1::new(gray.width(), gray.height());
    for (x, y, pixel) in gray.enumerate_pixels() {
        if pixel.0[0] == 0 {
            set_black(&mut logo, x, y);
        }
    }
    Ok(logo)
}

/// Draws a QR code about `size_dots` wide, with its white border. Each square is a
/// whole number of dots, so the code may come out a little smaller than asked.
pub(crate) fn qr_picture(data: &str, size_dots: u32) -> Result<Bitmap1, PrintError> {
    let code = QrCode::new(data.as_bytes()).map_err(|_| PrintError::QrTooLong)?;
    let squares = code.width() + 2 * QR_QUIET_SQUARES;
    let dots_per_square = (size_dots as usize / squares).max(1);
    let side = (squares * dots_per_square) as u32;

    let mut picture = Bitmap1::new(side, side);
    for (index, color) in code.to_colors().into_iter().enumerate() {
        if color != Color::Dark {
            continue;
        }
        let square_x = index % code.width() + QR_QUIET_SQUARES;
        let square_y = index / code.width() + QR_QUIET_SQUARES;
        for dy in 0..dots_per_square {
            for dx in 0..dots_per_square {
                let x = (square_x * dots_per_square + dx) as u32;
                let y = (square_y * dots_per_square + dy) as u32;
                set_black(&mut picture, x, y);
            }
        }
    }
    Ok(picture)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hw_02_qr_code_has_square_dots_and_a_white_border() {
        let picture = qr_picture("T1-000123", 160).unwrap();
        // Version 1 is 21 squares, plus 4 each side = 29; 160 / 29 = 5 dots per square.
        assert_eq!((picture.width, picture.height), (145, 145));
        assert!(!is_black(&picture, 19, 19), "white border");
        assert!(
            is_black(&picture, 20, 20),
            "the corner finder pattern starts here"
        );
    }

    #[test]
    fn qr_text_that_does_not_fit_is_an_error() {
        let too_long = "x".repeat(5000);
        assert_eq!(qr_picture(&too_long, 160), Err(PrintError::QrTooLong));
    }

    #[test]
    fn dots_outside_the_picture_are_white_and_ignored() {
        let mut picture = Bitmap1::new(10, 2);
        set_black(&mut picture, 10, 0);
        set_black(&mut picture, 9, 1);
        assert!(!is_black(&picture, 10, 0));
        assert!(is_black(&picture, 9, 1));
        assert_eq!(picture.bits, vec![0, 0, 0, 0x40]);
    }
}
