---
tags: [module]
crate: catalog
milestone: M4
---
# Module: catalog

**Purpose:** everything about *what* the shop sells: products, variants, kits, barcodes, units, categories, brands, images, search, price history.

**Requirements:** [[Req - Catalog]], parts of [[Req - Checkout]]. **Depends on:** `core`, `rusqlite`, `serde`, `thiserror`, `ts-rs` (TypeScript types for the screens, ADR-014), `image` (reading and resizing pictures; PNG, JPEG and WebP only). **Dev-only:** `criterion` (the search benchmark).

**Status:** built in task B-catalog (Rust). The `app` commands and the Products, Categories, Brands and Units screens came in C-M4 (`ui/src/modules/catalog/`).

## Tables
```sql
CREATE TABLE catalog_units (
  id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, symbol TEXT NOT NULL,
  unit_code TEXT,                       -- optional UN/ECE Rec 20 code: H87 piece, KGM, GRM, LTR, MLT (see Reuse Map)
  decimals INTEGER NOT NULL CHECK (decimals BETWEEN 0 AND 3),
  is_system INTEGER NOT NULL DEFAULT 0, archived_at INTEGER
);
CREATE TABLE catalog_categories (
  id TEXT PRIMARY KEY, parent_id TEXT REFERENCES catalog_categories(id),
  name TEXT NOT NULL, sort INTEGER NOT NULL DEFAULT 0, archived_at INTEGER
);
CREATE TABLE catalog_brands (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, archived_at INTEGER);

CREATE TABLE catalog_products (
  id                   TEXT PRIMARY KEY,
  kind                 TEXT NOT NULL CHECK (kind IN ('standard','variant_parent','variant','kit')),
  parent_id            TEXT REFERENCES catalog_products(id),     -- for variants
  variant_label        TEXT,                                     -- "200 ml / Mango"
  name                 TEXT NOT NULL,
  sku                  TEXT NOT NULL UNIQUE,
  category_id          TEXT NOT NULL REFERENCES catalog_categories(id),
  brand_id             TEXT REFERENCES catalog_brands(id),
  unit_id              TEXT NOT NULL REFERENCES catalog_units(id),
  sale_price_minor     INTEGER NOT NULL DEFAULT 0 CHECK (sale_price_minor >= 0),
  cost_price_minor     INTEGER NOT NULL DEFAULT 0 CHECK (cost_price_minor >= 0),
  tax_rate_id          TEXT NOT NULL,                            -- tax module (no FK)
  is_weighed           INTEGER NOT NULL DEFAULT 0,
  scale_item_code      TEXT UNIQUE,
  track_stock          INTEGER NOT NULL DEFAULT 1,
  track_expiry         INTEGER NOT NULL DEFAULT 0,
  preferred_supplier_id TEXT,                                    -- purchasing (no FK)
  image_path           TEXT,
  description          TEXT,
  archived_at          INTEGER,
  created_at           INTEGER NOT NULL,
  updated_at           INTEGER NOT NULL
);
CREATE INDEX catalog_products_category ON catalog_products(category_id);
CREATE INDEX catalog_products_parent ON catalog_products(parent_id);

CREATE TABLE catalog_product_attributes (       -- variant attributes: Size=200 ml, Flavour=Mango
  product_id TEXT NOT NULL REFERENCES catalog_products(id),
  name TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY (product_id, name)
);
CREATE TABLE catalog_barcodes (
  code TEXT PRIMARY KEY,
  product_id TEXT NOT NULL REFERENCES catalog_products(id),
  is_primary INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL
);
CREATE INDEX catalog_barcodes_product ON catalog_barcodes(product_id);
CREATE TABLE catalog_kit_components (
  kit_id TEXT NOT NULL REFERENCES catalog_products(id),
  component_id TEXT NOT NULL REFERENCES catalog_products(id),
  qty_milli INTEGER NOT NULL CHECK (qty_milli > 0),
  PRIMARY KEY (kit_id, component_id)
);
CREATE TABLE catalog_price_changes (
  id TEXT PRIMARY KEY, product_id TEXT NOT NULL REFERENCES catalog_products(id),
  field TEXT NOT NULL CHECK (field IN ('sale_price','cost_price')),
  old_minor INTEGER NOT NULL, new_minor INTEGER NOT NULL,
  changed_at INTEGER NOT NULL, changed_by TEXT, reason TEXT          -- 'manual','grn','import'
);
CREATE TABLE catalog_sequences (name TEXT PRIMARY KEY, next INTEGER NOT NULL); -- 'sku', 'internal_barcode'

CREATE VIRTUAL TABLE catalog_search USING fts5(
  product_id UNINDEXED, name, sku, barcodes, category, brand,
  tokenize = 'unicode61 remove_diacritics 2', prefix = '2 3 4'
);
-- Which search row belongs to which product. FTS5 can't look rows up by product_id
-- quickly, and VACUUM may renumber the products table's hidden rowid, so this links them.
CREATE TABLE catalog_search_rows (
  product_id TEXT PRIMARY KEY REFERENCES catalog_products(id),
  search_rowid INTEGER NOT NULL UNIQUE
);
CREATE VIEW catalog_v_products AS
  SELECT p.id, p.kind, p.name, p.sku, p.category_id, c.name AS category_name,
         p.brand_id, b.name AS brand_name, u.symbol AS unit_symbol, u.decimals AS unit_decimals,
         p.sale_price_minor, p.cost_price_minor, p.tax_rate_id, p.track_stock, p.archived_at
  FROM catalog_products p
  JOIN catalog_categories c ON c.id = p.category_id
  LEFT JOIN catalog_brands b ON b.id = p.brand_id
  JOIN catalog_units u ON u.id = p.unit_id;
CREATE VIEW catalog_v_categories AS SELECT id, parent_id, name FROM catalog_categories;
```
The full SQL is `crates/catalog/migrations/001_init.sql`. It also has `CHECK (x IN (0, 1))` on the yes/no columns and a few extra indexes (category parent, product brand and name, kit component, price history by product).

Seeded system units, with fixed ids so code can name them (`catalog::UNIT_PIECE` … `UNIT_PACK`): piece `pc` (0 decimals, `H87`), kg (3, `KGM`), g (0, `GRM`), litre `L` (3, `LTR`), ml (0, `MLT`), pack (0, no code). Seeded category: "Uncategorised" (`catalog::UNCATEGORISED`). Sequences `sku` and `internal_barcode` start at 1.

`image_path` holds a **file name** inside the images folder `app` passes in (`<product id>.jpg`); the thumbnail is `<product id>.thumb.jpg` (`catalog::thumbnail_file`). Keeping only the name means the data folder can move.

## Public API
Data-changing functions take `&rusqlite::Transaction`; reading functions take `&rusqlite::Connection` (a transaction works too). Every function returns `Result<_, CatalogError>`, and `CatalogError` turns into `AppError` (codes below).
```rust
// products
pub fn create_product(tx, NewProduct, now: Timestamp) -> Result<Id>;          // blank SKU → "P000001"; no category → Uncategorised
pub fn update_product(tx, &Id, ProductUpdate, by: &Id, now) -> Result<PriceChanges>; // writes price history (reason "manual")
pub fn update_product_with_reason(tx, &Id, ProductUpdate, (by: &Id, reason: &str), now) -> Result<PriceChanges>; // reason "manual", "grn" or "import" (C-M12 import)
pub fn sale_price_changed_since(conn, since: Timestamp) -> Result<Vec<Id>>;       // products whose sale price changed at or after `since` (shelf tags, CAT-13, C-M12)
pub fn archive_product(tx, &Id, now) -> Result<()>;
pub fn unarchive_product(tx, &Id, now) -> Result<()>;                         // back on sale; unit, category and brand must be active (C-M4)
pub fn delete_unused_product(tx, &Id, used_elsewhere: bool) -> Result<()>;    // app passes the "used" check
pub fn get(conn, &Id) -> Result<Product>;                                     // with barcodes (primary first) and attributes
pub fn list(conn, &ProductFilter, ProductPage) -> Result<ProductList>;       // sorted by name; category filter includes sub-categories; `parent_id` lists a parent's variants (C-M4)
// selling
pub fn find_by_code(conn, code: &str) -> Result<Option<SaleProduct>>;        // barcode, then SKU; never archived or variant parents
pub fn find_by_scale_code(conn, item_code: &str) -> Result<Option<SaleProduct>>;
pub fn search(conn, q: &str, limit: u32) -> Result<Vec<SearchHit>>;          // exact barcode/SKU first, then FTS ranked by bm25
pub fn rebuild_search(tx) -> Result<()>;                                     // rewrites every search row (after a restore, or if search looks wrong)
pub fn price_info(conn, ids: &[Id]) -> Result<HashMap<Id, ProductPriceInfo>>; // price, cost, tax id, category path (own first), brand
// barcodes and numbers
pub fn add_barcode(tx, &Id, code: &str, now) -> Result<()>;                 // uniqueness here; check digit validated by app via barcode crate
pub fn remove_barcode(tx, code: &str) -> Result<()>;                        // next oldest barcode becomes primary
pub fn next_internal_number(tx) -> Result<u64>;                             // app builds the EAN-13 with prefix and check digit
// prices
pub fn set_cost(tx, &Id, Money, reason: &str, by: &Id, now) -> Result<()>;  // reason: "manual", "grn" or "import"
pub fn price_history(conn, &Id) -> Result<Vec<PriceHistoryRow>>;           // oldest first (CAT-09)
// kits and variants
pub fn kit_components(conn, kit: &Id) -> Result<Vec<(Id, Qty)>>;
pub fn set_kit_components(tx, kit: &Id, &[(Id, Qty)]) -> Result<()>;      // replaces the list
pub fn generate_variants(tx, parent: &Id, attrs: &[(String, Vec<String>)], now) -> Result<Vec<Id>>; // skips combinations that exist
// images
pub fn set_image(tx, &Id, source: &Path, images_dir: &Path) -> Result<()>; // PNG/JPG/WebP ≤ 10 MB → 512 px JPEG + 96 px thumbnail
pub fn thumbnail_file(image_file: &str) -> String;
// units, categories, brands
pub fn create_unit(tx, &UnitFields) -> Result<Id>;  update_unit(tx, &Id, &UnitFields);  archive_unit(tx, &Id, now);  list_units(conn, include_archived);
pub fn create_category(tx, &CategoryFields) -> Result<Id>;  update_category(tx, &Id, &CategoryFields);  archive_category(tx, &Id, now);  list_categories(conn, include_archived);
pub fn create_brand(tx, name) -> Result<Id>;  rename_brand(tx, &Id, name);  archive_brand(tx, &Id, now);  list_brands(conn, include_archived);
pub fn migrations() -> ModuleMigrations;
```
**Data shapes** (all plain structs with `serde` and `ts_rs::TS`, exported to `ui/src/generated/`; see `src/types.rs`): `NewProduct` (`NewProduct::new(kind, name, unit_id, tax_rate_id)` gives the defaults), `Product`, `ProductUpdate` (`ProductUpdate::from_product(&product)` to start an edit), `ProductKind` (`standard`, `variant_parent`, `variant`, `kit`), `PriceChanges`/`PriceChange`, `PriceField`, `PriceHistoryRow`, `ProductFilter`, `ProductPage { offset, limit }`, `ProductList { rows, total }`, `ProductRow`, `SaleProduct`, `SearchHit`, `ProductPriceInfo`, `Unit`/`UnitFields`, `Category`/`CategoryFields` (`CategoryFields::new(name)`), `Brand`.

**Rules the functions enforce**
- Name required, at most 120 characters (products, units, categories, brands). Prices ≥ 0. A weighed product needs a unit with decimals.
- Unit, category and brand must exist and be active when a product is saved.
- SKU, barcode and scale item code each belong to one product. A generated SKU skips numbers typed by hand.
- Kits and variant parents never track stock (`track_stock` is saved as false). A variant parent can't have barcodes or be a kit component.
- Variants are made only by `generate_variants` (`create_product` refuses kind `variant`). Name = parent name + values ("Shampoo 200 ml Mango"); label = values joined by " / ".
- Archiving a unit is refused for built-in units and units used by an active product; changing decimals is refused while any product uses the unit. Archiving a category is refused for "Uncategorised", with active children, or with active products. A category can't move under itself or its own child.
- Delete is refused when `used_elsewhere`, when the product is a kit component, or when it has variants.

**Error codes** (`AppError` `code`): validation: `name_missing`, `name_too_long`, `sku_missing`, `price_negative`, `price_reason_invalid`, `weighed_unit_needs_decimals`, `variant_needs_parent`, `parent_not_sellable`, `barcode_invalid`, `not_a_kit`, `kit_contains_itself`, `kit_contains_kit`, `kit_qty_not_positive`, `kit_component_repeated`, `not_a_variant_parent`, `variant_attribute_missing`, `variant_attribute_repeated`, `symbol_missing`, `unit_decimals_invalid`, `category_parent_loop`, `image_too_large`, `image_not_supported`. Conflict: `barcode_in_use` (params `code`, `product_name`, `sku`), and with param `value`: `sku_in_use`, `scale_item_code_in_use`, `product_in_use`, `product_has_variants`, `unit_name_in_use`, `unit_in_use`, `unit_is_built_in`, `brand_name_in_use`, `category_is_built_in`, `category_has_children`, `category_has_products`. Not found: entity `product`, `unit`, `category`, `brand` or `barcode`. Database and file errors are `Internal`.

## Tests
Every test name starts with the requirement it covers. Run them with `cargo nextest run -p catalog`.
- `tests/products.rs`: CAT-01 SKU auto-generation (and skipping hand-typed SKUs), duplicate SKU, name rules, Uncategorised default, weighed needs decimals, negative prices, scale item code; **CAT-04 duplicate barcode message** (names the owner and SKU), primary barcode, removed barcodes reusable; **CAT-08 archive hides from search and `find_by_code`**, archived only listed on request, brought back with `unarchive_product` (and refused while its category is archived), delete refused when used; **CAT-09 price change rows written only when prices change**, `set_cost` reason; CAT-11 internal numbers; CHK-01 barcode then SKU; CHK-02 unknown code.
- `tests/search.rs`: CAT-05 **prefix search**, every word must match, **accented letters**, finds by SKU/barcode/category/brand, **exact match ranked first**, **search row updated on barcode change and on category/brand rename**, and on product edits; `rebuild_search`; CHK-03 symbols ignored and limit respected.
- `tests/variants_kits.rs`: CAT-06 **variant generation creates the cartesian product**, generating again adds only new combinations, only parents have variants, parent holds no stock; CHK-15 **parent not sellable**, variants found by search and barcode; CHK-14 kit components saved; CAT-07 **kit cycles refused** (contains itself, contains another kit), quantities above zero and listed once, only kits have components.
- `tests/units_categories_brands.rs`: CAT-02 built-in units, adding units, unit in use, built-in units stay; CAT-03 category tree and loops, archive refused with products or children, brands unique; CAT-01 unknown unit or archived category refused.
- `tests/images.rs`: CAT-10 resize to 512 px + 96 px thumbnail, small pictures not enlarged, WebP and JPEG read, other files and files over 10 MB refused.
- `tests/contract.rs`: migrations labelled; every table and view has the `catalog_` prefix; `catalog_v_products` and `catalog_v_categories` columns; `price_info`; `list` filters (also by parent) and paging; `get`; `SearchHit` and `SaleProduct` fields; error codes in `AppError`.
- `tests/benchmark.rs`: **NFR-P2/NFR-P1: `search` ≤ 150 ms and `find_by_code` ≤ 2 ms over 50k products** (80k barcodes). It builds 50,000 products, so it is ignored by default: `cargo nextest run -p catalog --release --run-ignored only`. `cargo bench -p catalog` (criterion, `benches/search.rs`) gives detailed timings. Measured in task B-catalog (Linux cloud machine, release build): `search` "mil" about 10 ms, "rice sug" about 0.8 ms, "p0012" about 0.5 ms; `find_by_code` about 4 µs by barcode or SKU. The debug build also passes the budgets.
- Unit tests next to the code: FTS query building, cartesian product, attribute checks, SKU format, name checks, LIKE escaping, white background blending, error conversion, built-in ids.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/catalog/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/catalog/` |
| **Screen folder** | `ui/src/modules/catalog/` (products, categories, brands, units, labels and shelf-tag screens) |
| **Requirements** | [[Req - Catalog]] CAT-01 to CAT-13; [[Req - Checkout]] CHK-01 to CHK-03, CHK-14, CHK-15 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Checks that tax rates and suppliers exist (done by `app`), the images folder path, the user id |
| **Used by** (re-run their tests when the public API changes) | Scanning, cart pricing, completing a sale (product snapshots), receiving goods (`set_cost`), imports, product save, labels, and reports (`catalog_v_products`) |
| **Test it on its own** | `cargo nextest run -p catalog` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Update the product's search row in the **same transaction** as every product, barcode, category or brand change, or search goes stale.
- A barcode belongs to one product only.
- Never hard-delete a product that was ever used. Archive it.
- Cost changes go through `set_cost`, so price history is recorded.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- **Images:** files are written before the transaction commits. If the caller rolls back, the new files stay on disk (they overwrite the product's own `<id>.jpg`, so nothing else breaks). Old files of deleted products aren't removed yet. There is no "remove image" function yet. (The screen gets the picture as a data address from `app`, so caching is no problem.) Lists and search don't show thumbnails yet.
- **Benchmark in CI:** `tests/benchmark.rs` is ignored by default. The nightly performance job (`xtask bench`, [[Test Strategy]]) should run it with `--release --run-ignored only` once that job exists.
- **Kit stock:** "available kit quantity" (CAT-07) needs stock, so it belongs to `inventory`/`app`, using `kit_components`.
- **Labels and shelf tags** (CAT-12, CAT-13) were built in C-M12: `app::workflows::labels` with `printing`, and the screen `ui/src/modules/catalog/pages/LabelsPage.tsx`. The catalog added `sale_price_changed_since` for "prices changed since", and `update_product_with_reason` so imports record "import" in the price history. A unit price for packs ("per litre" on a 500 ml bottle) needs the pack's content, which products don't keep yet.
- **Unarchive:** products can be brought back (`unarchive_product`, C-M4). Units, categories and brands can't yet; add functions if the shop asks.
- **Product list columns from other modules:** the list has no stock column and no "low stock" or "no barcode" filters ([[Back Office Screens]]). Stock comes with the inventory screens (C-M5); "no barcode" needs a `ProductFilter` field.
- **Barcode order:** the first barcode added stays the main one. Removing it makes the next oldest the main one; there is no "make this the main barcode" yet.
- **Price-list prices** on the Prices tab come with the pricing screens (C-M8).
