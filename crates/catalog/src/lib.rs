//! Products, variants, kits, barcodes, units, categories, brands, images, search and price history.
//!
//! Page: `docs/05 Modules/Module - catalog.md`. Functions that change data take the
//! caller's `&Transaction`; functions that read take a `&Connection` (rule R3).

mod barcodes;
mod categories;
mod checks;
mod error;
mod images;
mod kits;
mod lookup;
mod prices;
mod products;
mod read;
mod rows;
mod search;
mod sequences;
mod types;
mod units;
mod variants;

pub use barcodes::{add_barcode, remove_barcode};
pub use categories::{
    archive_brand, archive_category, create_brand, create_category, list_brands, list_categories,
    rename_brand, update_category,
};
pub use error::CatalogError;
pub use images::{set_image, thumbnail_file};
pub use kits::{kit_components, set_kit_components};
pub use lookup::{find_by_code, find_by_scale_code, price_info};
pub use prices::{price_history, set_cost};
pub use products::{archive_product, create_product, delete_unused_product, update_product};
pub use read::{get, list};
pub use search::{rebuild_search, search};
pub use sequences::next_internal_number;
pub use types::{
    Brand, Category, CategoryFields, NewProduct, PriceChange, PriceChanges, PriceField,
    PriceHistoryRow, Product, ProductFilter, ProductKind, ProductList, ProductPage,
    ProductPriceInfo, ProductRow, ProductUpdate, SaleProduct, SearchHit, UNCATEGORISED, UNIT_G,
    UNIT_KG, UNIT_LITRE, UNIT_ML, UNIT_PACK, UNIT_PIECE, Unit, UnitFields,
};
pub use units::{archive_unit, create_unit, list_units, update_unit};
pub use variants::generate_variants;

use pos_core::{Migration, ModuleMigrations};

/// This module's database migrations, oldest first. `app` passes them to `db::migrate`.
pub fn migrations() -> ModuleMigrations {
    ModuleMigrations {
        module: "catalog",
        migrations: MIGRATIONS,
    }
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/001_init.sql"),
}];
