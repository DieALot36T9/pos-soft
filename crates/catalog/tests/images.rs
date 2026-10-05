//! Product images (rule CAT-10).

#![allow(clippy::unwrap_used)]

mod common;

use std::path::{Path, PathBuf};

use catalog::CatalogError;
use common::{create, standard, test_db};
use image::{ImageFormat, RgbaImage};

/// A new, empty folder for one test.
fn temp_folder() -> PathBuf {
    let folder = std::env::temp_dir().join(format!("catalog-images-{}", pos_core::Id::new()));
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

fn write_picture(path: &Path, width: u32, height: u32, format: ImageFormat) {
    let picture = RgbaImage::from_pixel(width, height, image::Rgba([200, 30, 30, 255]));
    if format == ImageFormat::Jpeg {
        // JPEG has no see-through parts, so it is saved without them.
        image::DynamicImage::ImageRgba8(picture)
            .to_rgb8()
            .save_with_format(path, format)
            .unwrap();
    } else {
        picture.save_with_format(path, format).unwrap();
    }
}

#[test]
fn cat_10_image_is_resized_to_512_with_a_96_thumbnail() {
    let folder = temp_folder();
    let source = folder.join("photo.png");
    write_picture(&source, 1024, 768, ImageFormat::Png);
    let images = folder.join("images");

    let mut db = test_db();
    let milk = create(&mut db, standard("Milk", 100));
    let tx = db.transaction().unwrap();
    catalog::set_image(&tx, &milk, &source, &images).unwrap();

    let file_name = catalog::get(&tx, &milk).unwrap().image_path.unwrap();
    let main = image::open(images.join(&file_name)).unwrap();
    assert_eq!((main.width(), main.height()), (512, 384));
    let thumbnail = image::open(images.join(catalog::thumbnail_file(&file_name))).unwrap();
    assert_eq!((thumbnail.width(), thumbnail.height()), (96, 72));

    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn cat_10_small_images_are_not_enlarged_and_webp_and_jpg_are_read() {
    let folder = temp_folder();
    let mut db = test_db();
    let milk = create(&mut db, standard("Milk", 100));
    let tx = db.transaction().unwrap();

    for (name, format) in [
        ("small.webp", ImageFormat::WebP),
        ("small.jpg", ImageFormat::Jpeg),
    ] {
        let source = folder.join(name);
        write_picture(&source, 300, 200, format);
        catalog::set_image(&tx, &milk, &source, &folder).unwrap();
        let file_name = catalog::get(&tx, &milk).unwrap().image_path.unwrap();
        let main = image::open(folder.join(&file_name)).unwrap();
        assert_eq!((main.width(), main.height()), (300, 200), "{name}");
    }

    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn cat_10_other_files_are_refused() {
    let folder = temp_folder();
    let source = folder.join("notes.png");
    std::fs::write(&source, b"this is not a picture").unwrap();
    // A GIF picture: only its first bytes matter, because GIF isn't allowed at all.
    let gif = folder.join("anim.gif");
    std::fs::write(&gif, b"GIF89a\x01\x00\x01\x00\x00\x00\x00;").unwrap();

    let mut db = test_db();
    let milk = create(&mut db, standard("Milk", 100));
    let tx = db.transaction().unwrap();

    let error = catalog::set_image(&tx, &milk, &source, &folder).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "image_not_supported",
            ..
        }
    ));
    let error = catalog::set_image(&tx, &milk, &gif, &folder).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "image_not_supported",
            ..
        }
    ));

    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn cat_10_files_over_10_mb_are_refused() {
    let folder = temp_folder();
    let source = folder.join("huge.png");
    let file = std::fs::File::create(&source).unwrap();
    file.set_len(10 * 1024 * 1024 + 1).unwrap();

    let mut db = test_db();
    let milk = create(&mut db, standard("Milk", 100));
    let tx = db.transaction().unwrap();
    let error = catalog::set_image(&tx, &milk, &source, &folder).unwrap_err();
    assert!(matches!(
        error,
        CatalogError::Invalid {
            code: "image_too_large",
            ..
        }
    ));

    std::fs::remove_dir_all(&folder).unwrap();
}
