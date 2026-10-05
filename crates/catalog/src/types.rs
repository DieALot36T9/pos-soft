//! The plain data shapes that go in and out of the catalog's public functions.

use pos_core::{Id, Money, Timestamp};
use serde::{Deserialize, Serialize};

use crate::error::{CatalogError, invalid};

/// The id of the built-in "Uncategorised" category (rule CAT-01).
pub const UNCATEGORISED: &str = "00000000000000000000000010";
/// Ids of the built-in units (rule CAT-02). Pass one to `Id::parse`.
pub const UNIT_PIECE: &str = "00000000000000000000000001";
pub const UNIT_KG: &str = "00000000000000000000000002";
pub const UNIT_G: &str = "00000000000000000000000003";
pub const UNIT_LITRE: &str = "00000000000000000000000004";
pub const UNIT_ML: &str = "00000000000000000000000005";
pub const UNIT_PACK: &str = "00000000000000000000000006";

/// What sort of product a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductKind {
    /// An ordinary product.
    Standard,
    /// Holds shared details for its variants. It is never sold itself (rule CAT-06).
    VariantParent,
    /// One combination of a parent's attributes, e.g. "Shampoo 200 ml Mango".
    Variant,
    /// A bundle of other products with its own price and no stock (rule CAT-07).
    Kit,
}

impl ProductKind {
    /// The text stored in the `kind` column.
    pub fn as_str(self) -> &'static str {
        match self {
            ProductKind::Standard => "standard",
            ProductKind::VariantParent => "variant_parent",
            ProductKind::Variant => "variant",
            ProductKind::Kit => "kit",
        }
    }

    /// Reads the text stored in the `kind` column.
    pub fn parse(text: &str) -> Result<ProductKind, CatalogError> {
        match text {
            "standard" => Ok(ProductKind::Standard),
            "variant_parent" => Ok(ProductKind::VariantParent),
            "variant" => Ok(ProductKind::Variant),
            "kit" => Ok(ProductKind::Kit),
            _ => Err(invalid("kind", "kind_invalid")),
        }
    }
}

/// A new product as typed on the product form (rule CAT-01).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewProduct {
    pub kind: ProductKind,
    pub name: String,
    /// Left empty, the catalog makes one: "P000001".
    pub sku: Option<String>,
    /// Left empty, the product goes in "Uncategorised".
    pub category_id: Option<Id>,
    pub brand_id: Option<Id>,
    pub unit_id: Id,
    pub sale_price: Money,
    pub cost_price: Money,
    /// A tax rate from the tax module. `app` checks it exists.
    pub tax_rate_id: Id,
    pub is_weighed: bool,
    pub scale_item_code: Option<String>,
    /// Ignored for kits and variant parents: they never hold stock.
    pub track_stock: bool,
    pub track_expiry: bool,
    /// A supplier from the purchasing module. `app` checks it exists.
    pub preferred_supplier_id: Option<Id>,
    pub description: Option<String>,
    /// The first one becomes the primary barcode.
    pub barcodes: Vec<String>,
}

impl NewProduct {
    /// A product with the usual defaults: no SKU yet, prices 0, stock tracked, no barcodes.
    pub fn new(kind: ProductKind, name: &str, unit_id: Id, tax_rate_id: Id) -> NewProduct {
        NewProduct {
            kind,
            name: name.to_string(),
            sku: None,
            category_id: None,
            brand_id: None,
            unit_id,
            sale_price: Money::zero(),
            cost_price: Money::zero(),
            tax_rate_id,
            is_weighed: false,
            scale_item_code: None,
            track_stock: true,
            track_expiry: false,
            preferred_supplier_id: None,
            description: None,
            barcodes: Vec::new(),
        }
    }
}

/// Everything about one product, as shown on the product page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    pub id: Id,
    pub kind: ProductKind,
    pub parent_id: Option<Id>,
    /// For variants. Example: "200 ml / Mango".
    pub variant_label: Option<String>,
    pub name: String,
    pub sku: String,
    pub category_id: Id,
    pub brand_id: Option<Id>,
    pub unit_id: Id,
    pub sale_price: Money,
    pub cost_price: Money,
    pub tax_rate_id: Id,
    pub is_weighed: bool,
    pub scale_item_code: Option<String>,
    pub track_stock: bool,
    pub track_expiry: bool,
    pub preferred_supplier_id: Option<Id>,
    /// File name inside the images folder. The thumbnail name comes from `thumbnail_file`.
    pub image_path: Option<String>,
    pub description: Option<String>,
    pub archived_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    /// Primary barcode first.
    pub barcodes: Vec<String>,
    /// Variant attributes as (name, value), sorted by name. Example: ("Size", "200 ml").
    pub attributes: Vec<(String, String)>,
}

/// The editable fields of a product. Barcodes, kit parts and the image have their own functions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductUpdate {
    pub name: String,
    pub sku: String,
    pub category_id: Id,
    pub brand_id: Option<Id>,
    pub unit_id: Id,
    pub sale_price: Money,
    pub cost_price: Money,
    pub tax_rate_id: Id,
    pub is_weighed: bool,
    pub scale_item_code: Option<String>,
    pub track_stock: bool,
    pub track_expiry: bool,
    pub preferred_supplier_id: Option<Id>,
    pub description: Option<String>,
}

impl ProductUpdate {
    /// The product's current values, ready to be edited.
    pub fn from_product(product: &Product) -> ProductUpdate {
        ProductUpdate {
            name: product.name.clone(),
            sku: product.sku.clone(),
            category_id: product.category_id.clone(),
            brand_id: product.brand_id.clone(),
            unit_id: product.unit_id.clone(),
            sale_price: product.sale_price,
            cost_price: product.cost_price,
            tax_rate_id: product.tax_rate_id.clone(),
            is_weighed: product.is_weighed,
            scale_item_code: product.scale_item_code.clone(),
            track_stock: product.track_stock,
            track_expiry: product.track_expiry,
            preferred_supplier_id: product.preferred_supplier_id.clone(),
            description: product.description.clone(),
        }
    }
}

/// One price that changed: the old and the new amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceChange {
    pub old: Money,
    pub new: Money,
}

/// Which prices an update changed. `None` means that price stayed the same.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceChanges {
    pub sale_price: Option<PriceChange>,
    pub cost_price: Option<PriceChange>,
}

/// Which price a history row is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceField {
    SalePrice,
    CostPrice,
}

impl PriceField {
    /// The text stored in the `field` column.
    pub fn as_str(self) -> &'static str {
        match self {
            PriceField::SalePrice => "sale_price",
            PriceField::CostPrice => "cost_price",
        }
    }
}

/// One row of a product's price history (rule CAT-09).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceHistoryRow {
    pub field: PriceField,
    pub old: Money,
    pub new: Money,
    pub changed_at: Timestamp,
    pub changed_by: Option<Id>,
    /// "manual", "grn" or "import".
    pub reason: Option<String>,
}

/// Which products `list` shows. The default shows every active product.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductFilter {
    /// Part of the name or SKU. Example: "mil".
    pub text: Option<String>,
    /// Products in this category or any category below it.
    pub category_id: Option<Id>,
    pub brand_id: Option<Id>,
    pub kind: Option<ProductKind>,
    pub include_archived: bool,
}

/// Which slice of a long list to return. Example: offset 50, limit 50 is the second page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductPage {
    pub offset: u32,
    pub limit: u32,
}

/// One row of the product list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductRow {
    pub id: Id,
    pub kind: ProductKind,
    pub name: String,
    pub sku: String,
    pub category_id: Id,
    pub category_name: String,
    pub brand_name: Option<String>,
    pub unit_symbol: String,
    pub sale_price: Money,
    pub cost_price: Money,
    pub archived_at: Option<Timestamp>,
}

/// One page of the product list, plus how many rows match in total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductList {
    pub rows: Vec<ProductRow>,
    pub total: u64,
}

/// What checkout needs to add a scanned product to the cart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaleProduct {
    pub id: Id,
    /// Standard, variant or kit. Variant parents are never returned.
    pub kind: ProductKind,
    pub name: String,
    pub sku: String,
    pub unit_symbol: String,
    /// How many decimals a quantity may have. Piece is 0, so 1.5 pieces is refused.
    pub unit_decimals: u8,
    pub sale_price: Money,
    pub tax_rate_id: Id,
    pub is_weighed: bool,
    pub track_stock: bool,
    pub track_expiry: bool,
    pub category_id: Id,
    pub brand_id: Option<Id>,
    /// Thumbnail file name inside the images folder, if the product has a picture.
    pub thumbnail: Option<String>,
}

/// One search result (rules CAT-05 and CHK-03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    pub sale_price: Money,
    pub unit_symbol: String,
    pub thumbnail: Option<String>,
    /// True when the search text is exactly this product's barcode or SKU.
    pub exact_match: bool,
}

/// What pricing needs about a product. `app` turns it into pricing's own input type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductPriceInfo {
    pub sale_price: Money,
    pub cost_price: Money,
    pub tax_rate_id: Id,
    /// The product's category first, then its parent, up to the top.
    pub category_path: Vec<Id>,
    pub brand_id: Option<Id>,
}

/// A unit products are counted or weighed in (rule CAT-02).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unit {
    pub id: Id,
    pub name: String,
    pub symbol: String,
    /// UN/ECE Recommendation 20 code. Example: "KGM" for kilogram.
    pub unit_code: Option<String>,
    /// 0 to 3. Example: kg has 3, so 1.250 kg is allowed.
    pub decimals: u8,
    /// Built-in units can't be archived.
    pub is_system: bool,
    pub archived_at: Option<Timestamp>,
}

/// The fields an admin types for a unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitFields {
    pub name: String,
    pub symbol: String,
    pub unit_code: Option<String>,
    pub decimals: u8,
}

/// A product category. Categories form a tree (rule CAT-03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    pub id: Id,
    pub parent_id: Option<Id>,
    pub name: String,
    pub sort: i64,
    pub archived_at: Option<Timestamp>,
}

/// The fields an admin types for a category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryFields {
    pub name: String,
    /// Empty for a top-level category.
    pub parent_id: Option<Id>,
    /// Lower numbers are shown first.
    pub sort: i64,
}

impl CategoryFields {
    /// A top-level category with the given name.
    pub fn new(name: &str) -> CategoryFields {
        CategoryFields {
            name: name.to_string(),
            parent_id: None,
            sort: 0,
        }
    }
}

/// A brand. Brands are a flat list (rule CAT-03).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Brand {
    pub id: Id,
    pub name: String,
    pub archived_at: Option<Timestamp>,
}
