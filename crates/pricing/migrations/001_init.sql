-- Pricing: price lists, tiers and promotions (Module - pricing).
-- Product, category and brand ids belong to catalog, so they have no foreign keys (ADR-005).

CREATE TABLE pricing_price_lists (
  id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, active INTEGER NOT NULL DEFAULT 1, created_at INTEGER NOT NULL
);
CREATE TABLE pricing_price_list_items (
  price_list_id TEXT NOT NULL REFERENCES pricing_price_lists(id),
  product_id    TEXT NOT NULL,                       -- catalog (no FK)
  price_minor   INTEGER NOT NULL CHECK (price_minor >= 0),
  PRIMARY KEY (price_list_id, product_id)
);
CREATE TABLE pricing_tiers (
  id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE,
  price_list_id TEXT REFERENCES pricing_price_lists(id),
  discount_bps  INTEGER NOT NULL DEFAULT 0, sort INTEGER NOT NULL DEFAULT 0, archived_at INTEGER
);
CREATE TABLE pricing_promotions (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,
  kind          TEXT NOT NULL CHECK (kind IN ('percent_off','amount_off','fixed_price','buy_x_get_y','multi_buy')),
  percent_bps   INTEGER,          -- percent_off
  amount_minor  INTEGER,          -- amount_off (per unit) / fixed_price (per unit) / multi_buy (bundle price)
  buy_qty       INTEGER,          -- buy_x_get_y: buy X (whole units)
  get_qty       INTEGER,          -- buy_x_get_y: get Y free
  bundle_qty    INTEGER,          -- multi_buy
  starts_at     INTEGER, ends_at INTEGER,
  days_mask     INTEGER NOT NULL DEFAULT 127,   -- bit 0 = Monday
  tier_id       TEXT REFERENCES pricing_tiers(id),
  active        INTEGER NOT NULL DEFAULT 1,
  created_at    INTEGER NOT NULL
);
CREATE TABLE pricing_promotion_targets (
  promotion_id TEXT NOT NULL REFERENCES pricing_promotions(id),
  target_type  TEXT NOT NULL CHECK (target_type IN ('product','category','brand')),
  target_id    TEXT NOT NULL,
  PRIMARY KEY (promotion_id, target_type, target_id)
);
CREATE INDEX pricing_targets_lookup ON pricing_promotion_targets(target_type, target_id);
CREATE VIEW pricing_v_promotions AS SELECT id, name, kind, starts_at, ends_at, active FROM pricing_promotions;
