---
tags: [module]
crate: catalog
milestone: M4
---
# Module: catalog

**Purpose:** everything about *what* the shop sells: products, variants, kits, barcodes, units, categories, brands, images, search, price history.

**Requirements:** [[Req - Catalog]], parts of [[Req - Checkout]]. **Depends on:** `core`, `rusqlite`, `image` (resizing).

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
Seeded system units: piece(0), kg(3), g(0), litre(3), ml(0), pack(0). Seeded category: "Uncategorised".

## Public API (main functions)
```rust
pub fn create_product(tx, NewProduct, by: &Id, now) -> Result<Id>;
pub fn update_product(tx, &Id, ProductUpdate, by: &Id, now) -> Result<PriceChanges>;
pub fn archive_product(tx, &Id, now) -> Result<()>;
pub fn delete_unused_product(tx, &Id, used_elsewhere: bool) -> Result<()>;   // app passes the "used" check
pub fn get(conn, &Id) -> Result<Product>;
pub fn list(conn, &ProductFilter, Page) -> Result<Paged<ProductRow>>;
pub fn find_by_code(conn, code: &str) -> Result<Option<SaleProduct>>;       // barcode, then SKU
pub fn find_by_scale_code(conn, item_code: &str) -> Result<Option<SaleProduct>>;
pub fn search(conn, q: &str, limit: u32) -> Result<Vec<SearchHit>>;         // FTS + exact first
pub fn price_info(conn, ids: &[Id]) -> Result<HashMap<Id, ProductPriceInfo>>; // price, tax id, category path, brand (app maps it to pricing's own input type)
pub fn add_barcode(tx, &Id, code: &str, now) -> Result<()>;                 // uniqueness, check digit validated by app via barcode crate
pub fn remove_barcode(tx, code: &str) -> Result<()>;
pub fn next_internal_number(tx) -> Result<u64>;
pub fn set_cost(tx, &Id, Money, reason: &str, by: &Id, now) -> Result<()>;
pub fn kit_components(conn, kit: &Id) -> Result<Vec<(Id, Qty)>>;
pub fn set_kit_components(tx, kit: &Id, &[(Id, Qty)]) -> Result<()>;
pub fn generate_variants(tx, parent: &Id, attrs: &[(String, Vec<String>)], now) -> Result<Vec<Id>>;
pub fn set_image(tx, &Id, source: &Path, images_dir: &Path) -> Result<()>;
// units, categories, brands: create/update/archive/list
pub fn migrations() -> ModuleMigrations;
```

## Tests
- CAT-04 duplicate barcode message; SKU auto-generation; archive hides from search and `find_by_code`.
- Search: prefix search, accented letters, exact match ranked first; the search row is updated on barcode/category rename.
- Variant generation creates the cartesian product; parent not sellable.
- Kit cycles refused (a kit can't contain itself or another kit).
- Price change rows written only when prices change.
- Benchmark: `search` ≤ 150 ms and `find_by_code` ≤ 2 ms over 50k products.

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
- Not started (see [[Task Index]]).
