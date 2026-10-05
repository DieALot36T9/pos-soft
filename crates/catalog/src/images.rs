//! Product pictures, resized with the `image` crate and stored as files (rule CAT-10).
//! The database keeps only the file name, relative to the images folder `app` passes in.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, ImageReader, RgbImage};
use pos_core::Id;
use rusqlite::{Transaction, named_params};

use crate::error::{CatalogError, invalid};
use crate::read::get;

/// Rule CAT-10: pictures over 10 MB are refused.
const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
/// The main picture fits in a 512 × 512 square, the thumbnail in 96 × 96.
const MAIN_SIZE: u32 = 512;
const THUMBNAIL_SIZE: u32 = 96;
/// JPEG quality: small files that still look sharp on screen.
const JPEG_QUALITY: u8 = 85;

/// Rule CAT-10: reads a PNG, JPG or WebP picture (up to 10 MB), saves a 512 px copy and a
/// 96 px thumbnail in `images_dir`, and stores the file name on the product.
/// Pictures smaller than 512 px are not enlarged.
pub fn set_image(
    tx: &Transaction,
    product_id: &Id,
    source: &Path,
    images_dir: &Path,
) -> Result<(), CatalogError> {
    get(tx, product_id)?;
    let picture = read_picture(source)?;

    std::fs::create_dir_all(images_dir).map_err(|error| file_error(images_dir, &error))?;
    let file_name = format!("{product_id}.jpg");
    save_jpeg(&shrink(&picture, MAIN_SIZE), &images_dir.join(&file_name))?;
    save_jpeg(
        &shrink(&picture, THUMBNAIL_SIZE),
        &images_dir.join(thumbnail_file(&file_name)),
    )?;

    tx.execute(
        "UPDATE catalog_products SET image_path = :file_name WHERE id = :id",
        named_params! { ":id": product_id.as_str(), ":file_name": file_name },
    )?;
    Ok(())
}

/// The thumbnail's file name for a picture's file name: "01HX….jpg" → "01HX….thumb.jpg".
pub fn thumbnail_file(image_file: &str) -> String {
    let stem = image_file.strip_suffix(".jpg").unwrap_or(image_file);
    format!("{stem}.thumb.jpg")
}

/// Opens the picture, refusing big files and formats other than PNG, JPG and WebP.
fn read_picture(source: &Path) -> Result<DynamicImage, CatalogError> {
    let size = std::fs::metadata(source)
        .map_err(|error| file_error(source, &error))?
        .len();
    if size > MAX_FILE_BYTES {
        return Err(invalid("image", "image_too_large"));
    }
    let reader = ImageReader::open(source)
        .map_err(|error| file_error(source, &error))?
        .with_guessed_format()
        .map_err(|error| file_error(source, &error))?;
    let allowed = matches!(
        reader.format(),
        Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP)
    );
    if !allowed {
        return Err(invalid("image", "image_not_supported"));
    }
    reader
        .decode()
        .map_err(|_| invalid("image", "image_not_supported"))
}

/// Makes the picture fit in a `size` × `size` square, keeping its shape.
fn shrink(picture: &DynamicImage, size: u32) -> DynamicImage {
    if picture.width() <= size && picture.height() <= size {
        picture.clone()
    } else {
        picture.resize(size, size, FilterType::Lanczos3)
    }
}

/// Saves as JPEG. JPEG has no see-through parts, so those are painted white first.
fn save_jpeg(picture: &DynamicImage, path: &Path) -> Result<(), CatalogError> {
    let file = File::create(path).map_err(|error| file_error(path, &error))?;
    let mut encoder = JpegEncoder::new_with_quality(BufWriter::new(file), JPEG_QUALITY);
    encoder
        .encode_image(&on_white(picture))
        .map_err(|error| file_error(path, &error))
}

/// Blends every pixel onto a white background, so see-through corners don't turn black.
fn on_white(picture: &DynamicImage) -> RgbImage {
    let rgba = picture.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [red, green, blue, alpha] = rgba.get_pixel(x, y).0;
        image::Rgb([blend(red, alpha), blend(green, alpha), blend(blue, alpha)])
    })
}

/// colour × alpha + white × (1 − alpha), in whole numbers and rounded.
fn blend(colour: u8, alpha: u8) -> u8 {
    let mixed = (u32::from(colour) * u32::from(alpha) + 255 * (255 - u32::from(alpha)) + 127) / 255;
    u8::try_from(mixed).unwrap_or(u8::MAX)
}

fn file_error(path: &Path, error: &dyn std::fmt::Display) -> CatalogError {
    CatalogError::File(format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_name_sits_next_to_the_picture() {
        assert_eq!(thumbnail_file("01HX.jpg"), "01HX.thumb.jpg");
    }

    #[test]
    fn see_through_pixels_become_white() {
        assert_eq!(blend(0, 0), 255);
        assert_eq!(blend(10, 255), 10);
        assert_eq!(blend(0, 128), 127);
    }
}
