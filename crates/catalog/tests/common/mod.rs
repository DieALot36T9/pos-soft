//! Shared helpers for the catalog tests: an in-memory database and quick product builders.

// Each test file uses a different part of these helpers.
#![allow(dead_code)]

use catalog::{NewProduct, ProductKind, UNIT_PIECE};
use pos_core::{Id, Money, Timestamp};
use rusqlite::Connection;

/// 2026-03-05 10:00 UTC, the test clock used across the project.
pub const NOW: Timestamp = Timestamp(1_772_704_800_000);

/// A fresh in-memory database built only by this module's own migrations (rule R6).
pub fn test_db() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .unwrap();
    for migration in catalog::migrations().migrations {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
}

/// The id of a built-in unit or category, from one of the catalog's constants.
pub fn id(text: &str) -> Id {
    Id::parse(text).unwrap()
}

/// A user id for "changed by".
pub fn user() -> Id {
    id("01HX5Z3J8Q9W2E4R6T8Y0V1K2M")
}

/// A tax rate id. The tax module owns tax rates; the catalog only stores the id.
pub fn vat() -> Id {
    id("01HX5Z3J8Q9W2E4R6T8Y0V1VAT")
}

/// A standard product sold by the piece, with a sale price in minor units.
pub fn standard(name: &str, sale_price: i64) -> NewProduct {
    NewProduct {
        sale_price: Money(sale_price),
        ..NewProduct::new(ProductKind::Standard, name, id(UNIT_PIECE), vat())
    }
}

/// Saves `product` and returns its id.
pub fn create(connection: &mut Connection, product: NewProduct) -> Id {
    let tx = connection.transaction().unwrap();
    let product_id = catalog::create_product(&tx, product, NOW).unwrap();
    tx.commit().unwrap();
    product_id
}

/// Saves a standard product with one barcode and returns its id.
pub fn create_with_barcode(
    connection: &mut Connection,
    name: &str,
    barcode: &str,
    sale_price: i64,
) -> Id {
    let product = NewProduct {
        barcodes: vec![barcode.to_string()],
        ..standard(name, sale_price)
    };
    create(connection, product)
}

/// Product names found by `search`, best first.
pub fn search_names(connection: &Connection, query: &str) -> Vec<String> {
    catalog::search(connection, query, 20)
        .unwrap()
        .into_iter()
        .map(|hit| hit.name)
        .collect()
}

/// Words used to make realistic product names for the reference database.
const WORDS: &[&str] = &[
    "Milk",
    "Rice",
    "Sugar",
    "Oil",
    "Soap",
    "Shampoo",
    "Biscuit",
    "Tea",
    "Coffee",
    "Juice",
    "Flour",
    "Lentil",
    "Salt",
    "Spice",
    "Noodles",
    "Chips",
    "Butter",
    "Cheese",
    "Yoghurt",
    "Bread",
    "Egg",
    "Chicken",
    "Beef",
    "Fish",
    "Potato",
    "Onion",
    "Garlic",
    "Ginger",
    "Mango",
    "Lemon",
    "Orange",
    "Apple",
    "Banana",
    "Toothpaste",
    "Detergent",
    "Tissue",
    "Candle",
    "Battery",
    "Honey",
    "Jam",
];
const SIZES: &[&str] = &[
    "100g", "250g", "500g", "1kg", "2kg", "200ml", "500ml", "1L", "2L", "5L",
];

/// Fills the database with `count` products and about 1.6 barcodes each, like the
/// NFR "reference data" (50,000 products, 80,000 barcodes). The same every time.
pub fn seed_reference(connection: &mut Connection, count: u32) {
    let tx = connection.transaction().unwrap();
    let mut categories = Vec::new();
    for number in 0..40 {
        let fields = catalog::CategoryFields::new(&format!("Category {number}"));
        categories.push(catalog::create_category(&tx, &fields).unwrap());
    }
    let mut brands = Vec::new();
    for number in 0..200 {
        brands.push(catalog::create_brand(&tx, &format!("Brand {number}")).unwrap());
    }
    for number in 0..count {
        let index = number as usize;
        let word = WORDS[index % WORDS.len()];
        let second = WORDS[(index / WORDS.len()) % WORDS.len()];
        let size = SIZES[(index / 7) % SIZES.len()];
        let mut barcodes = vec![format!("89{number:011}")];
        if number % 5 < 3 {
            barcodes.push(format!("20{number:011}"));
        }
        let product = NewProduct {
            category_id: Some(categories[index % categories.len()].clone()),
            brand_id: Some(brands[index % brands.len()].clone()),
            barcodes,
            ..standard(
                &format!("{word} {second} {size} {number}"),
                1000 + i64::from(number),
            )
        };
        catalog::create_product(&tx, product, NOW).unwrap();
    }
    tx.commit().unwrap();
}
