-- Catalog tables: what the shop sells (Module - catalog, Req - Catalog).

CREATE TABLE catalog_units (
  id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, symbol TEXT NOT NULL,
  unit_code TEXT,                       -- optional UN/ECE Rec 20 code: H87 piece, KGM, GRM, LTR, MLT
  decimals INTEGER NOT NULL CHECK (decimals BETWEEN 0 AND 3),
  is_system INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
  archived_at INTEGER
);

CREATE TABLE catalog_categories (
  id TEXT PRIMARY KEY, parent_id TEXT REFERENCES catalog_categories(id),
  name TEXT NOT NULL, sort INTEGER NOT NULL DEFAULT 0, archived_at INTEGER
);
CREATE INDEX catalog_categories_parent ON catalog_categories(parent_id);

CREATE TABLE catalog_brands (id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, archived_at INTEGER);

CREATE TABLE catalog_products (
  id                    TEXT PRIMARY KEY,
  kind                  TEXT NOT NULL CHECK (kind IN ('standard','variant_parent','variant','kit')),
  parent_id             TEXT REFERENCES catalog_products(id),     -- for variants
  variant_label         TEXT,                                     -- "200 ml / Mango"
  name                  TEXT NOT NULL,
  sku                   TEXT NOT NULL UNIQUE,
  category_id           TEXT NOT NULL REFERENCES catalog_categories(id),
  brand_id              TEXT REFERENCES catalog_brands(id),
  unit_id               TEXT NOT NULL REFERENCES catalog_units(id),
  sale_price_minor      INTEGER NOT NULL DEFAULT 0 CHECK (sale_price_minor >= 0),
  cost_price_minor      INTEGER NOT NULL DEFAULT 0 CHECK (cost_price_minor >= 0),
  tax_rate_id           TEXT NOT NULL,                            -- tax module (no FK)
  is_weighed            INTEGER NOT NULL DEFAULT 0 CHECK (is_weighed IN (0, 1)),
  scale_item_code       TEXT UNIQUE,
  track_stock           INTEGER NOT NULL DEFAULT 1 CHECK (track_stock IN (0, 1)),
  track_expiry          INTEGER NOT NULL DEFAULT 0 CHECK (track_expiry IN (0, 1)),
  preferred_supplier_id TEXT,                                     -- purchasing (no FK)
  image_path            TEXT,                                     -- file name inside the images folder
  description           TEXT,
  archived_at           INTEGER,
  created_at            INTEGER NOT NULL,
  updated_at            INTEGER NOT NULL
);
CREATE INDEX catalog_products_category ON catalog_products(category_id);
CREATE INDEX catalog_products_parent ON catalog_products(parent_id);
CREATE INDEX catalog_products_brand ON catalog_products(brand_id);
CREATE INDEX catalog_products_name ON catalog_products(name);

CREATE TABLE catalog_product_attributes (       -- variant attributes: Size=200 ml, Flavour=Mango
  product_id TEXT NOT NULL REFERENCES catalog_products(id),
  name TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY (product_id, name)
);

CREATE TABLE catalog_barcodes (
  code TEXT PRIMARY KEY,
  product_id TEXT NOT NULL REFERENCES catalog_products(id),
  is_primary INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0, 1)),
  created_at INTEGER NOT NULL
);
CREATE INDEX catalog_barcodes_product ON catalog_barcodes(product_id);

CREATE TABLE catalog_kit_components (
  kit_id TEXT NOT NULL REFERENCES catalog_products(id),
  component_id TEXT NOT NULL REFERENCES catalog_products(id),
  qty_milli INTEGER NOT NULL CHECK (qty_milli > 0),
  PRIMARY KEY (kit_id, component_id)
);
CREATE INDEX catalog_kit_components_component ON catalog_kit_components(component_id);

CREATE TABLE catalog_price_changes (
  id TEXT PRIMARY KEY, product_id TEXT NOT NULL REFERENCES catalog_products(id),
  field TEXT NOT NULL CHECK (field IN ('sale_price','cost_price')),
  old_minor INTEGER NOT NULL, new_minor INTEGER NOT NULL,
  changed_at INTEGER NOT NULL, changed_by TEXT,
  reason TEXT                           -- 'manual', 'grn', 'import'
);
CREATE INDEX catalog_price_changes_product ON catalog_price_changes(product_id, changed_at);

CREATE TABLE catalog_sequences (name TEXT PRIMARY KEY, next INTEGER NOT NULL); -- 'sku', 'internal_barcode'

-- Full-text search over name, SKU, barcodes, category and brand (rule CAT-05).
CREATE VIRTUAL TABLE catalog_search USING fts5(
  product_id UNINDEXED, name, sku, barcodes, category, brand,
  tokenize = 'unicode61 remove_diacritics 2', prefix = '2 3 4'
);

-- Which search row belongs to which product. The search table can't be looked up
-- by product_id quickly, and the products table has no stable whole-number id
-- (VACUUM may renumber its hidden rowid), so this table links the two.
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

-- Built-in units (rule CAT-02). Their ids never change, so code can name them.
INSERT INTO catalog_units (id, name, symbol, unit_code, decimals, is_system) VALUES
  ('00000000000000000000000001', 'piece', 'pc', 'H87', 0, 1),
  ('00000000000000000000000002', 'kg', 'kg', 'KGM', 3, 1),
  ('00000000000000000000000003', 'g', 'g', 'GRM', 0, 1),
  ('00000000000000000000000004', 'litre', 'L', 'LTR', 3, 1),
  ('00000000000000000000000005', 'ml', 'ml', 'MLT', 0, 1),
  ('00000000000000000000000006', 'pack', 'pack', NULL, 0, 1);

-- The default category (rule CAT-01).
INSERT INTO catalog_categories (id, parent_id, name, sort) VALUES
  ('00000000000000000000000010', NULL, 'Uncategorised', 0);

INSERT INTO catalog_sequences (name, next) VALUES ('sku', 1), ('internal_barcode', 1);
