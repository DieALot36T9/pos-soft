-- Inventory: batches, the stock ledger, cached levels, reorder rules, adjustment
-- reasons and stock counts. Page: docs/05 Modules/Module - inventory.md

-- A batch is one delivery of a product, with its own expiry date and cost.
-- on_hand_milli is a cache of the batch's ledger rows, updated in the same transaction.
CREATE TABLE inventory_batches (
  id              TEXT PRIMARY KEY,
  product_id      TEXT NOT NULL,
  batch_no        TEXT,
  expiry_date     TEXT,
  unit_cost_minor INTEGER NOT NULL DEFAULT 0,
  received_at     INTEGER NOT NULL,
  on_hand_milli   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX inventory_batches_fefo ON inventory_batches(product_id, expiry_date);

-- Every stock change, one signed row each. Never changed or deleted (ADR-010).
CREATE TABLE inventory_ledger (
  id              TEXT PRIMARY KEY,
  product_id      TEXT NOT NULL,
  batch_id        TEXT REFERENCES inventory_batches(id),
  qty_delta_milli INTEGER NOT NULL CHECK (qty_delta_milli <> 0),
  kind            TEXT NOT NULL CHECK (kind IN ('sale','sale_void','return','receive','supplier_return','adjust','count','opening')),
  reason_code     TEXT,
  note            TEXT,
  ref_type        TEXT,
  ref_id          TEXT,
  unit_cost_minor INTEGER NOT NULL DEFAULT 0,
  at              INTEGER NOT NULL,
  business_date   TEXT NOT NULL,
  till_id         TEXT NOT NULL,
  user_id         TEXT
);
CREATE INDEX inventory_ledger_product ON inventory_ledger(product_id, at);
CREATE INDEX inventory_ledger_ref ON inventory_ledger(ref_type, ref_id);
CREATE INDEX inventory_ledger_date ON inventory_ledger(business_date);
CREATE INDEX inventory_ledger_batch ON inventory_ledger(batch_id);

CREATE TRIGGER inventory_ledger_no_update BEFORE UPDATE ON inventory_ledger
BEGIN
  SELECT RAISE(ABORT, 'inventory_ledger is append-only');
END;
CREATE TRIGGER inventory_ledger_no_delete BEFORE DELETE ON inventory_ledger
BEGIN
  SELECT RAISE(ABORT, 'inventory_ledger is append-only');
END;

-- Cached stock per product: the sum of its ledger rows.
CREATE TABLE inventory_levels (
  product_id    TEXT PRIMARY KEY,
  on_hand_milli INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL
);

CREATE TABLE inventory_reorder_rules (
  product_id          TEXT PRIMARY KEY,
  reorder_level_milli INTEGER NOT NULL,
  reorder_qty_milli   INTEGER NOT NULL DEFAULT 0
);

-- Why stock was adjusted. Built-in reasons (is_system = 1) can't be switched off.
CREATE TABLE inventory_reasons (
  code      TEXT PRIMARY KEY,
  name      TEXT NOT NULL,
  direction TEXT NOT NULL CHECK (direction IN ('in','out','both')),
  is_system INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
  active    INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1))
);
INSERT INTO inventory_reasons (code, name, direction, is_system) VALUES
  ('damage', 'Damage', 'out', 1),
  ('expiry', 'Expired', 'out', 1),
  ('theft', 'Theft', 'out', 1),
  ('count_error', 'Count error', 'both', 1),
  ('opening', 'Opening stock', 'in', 1),
  ('internal_use', 'Internal use', 'out', 1),
  ('other', 'Other', 'both', 1);

CREATE TABLE inventory_counts (
  id         TEXT PRIMARY KEY,
  number     TEXT NOT NULL UNIQUE,
  name       TEXT NOT NULL,
  scope      TEXT NOT NULL CHECK (scope IN ('full','partial')),
  scope_json TEXT NOT NULL DEFAULT '{}',
  status     TEXT NOT NULL CHECK (status IN ('open','posted','cancelled')),
  created_at INTEGER NOT NULL,
  created_by TEXT NOT NULL,
  posted_at  INTEGER,
  posted_by  TEXT
);

-- One line per product (or per batch) in a count. expected_milli is the stock
-- when the item was first counted, so later sales don't show up as differences.
CREATE TABLE inventory_count_lines (
  count_id       TEXT NOT NULL REFERENCES inventory_counts(id),
  product_id     TEXT NOT NULL,
  batch_id       TEXT,
  counted_milli  INTEGER NOT NULL,
  expected_milli INTEGER NOT NULL,
  counted_at     INTEGER NOT NULL,
  counted_by     TEXT NOT NULL,
  PRIMARY KEY (count_id, product_id, batch_id)
);
-- SQLite lets NULLs repeat in a primary key, so this index keeps one
-- "whole product" line (batch_id NULL) per product too.
CREATE UNIQUE INDEX inventory_count_lines_one_each
  ON inventory_count_lines(count_id, product_id, IFNULL(batch_id, ''));

-- Gapless document numbers, e.g. 'count' → SC-000001.
CREATE TABLE inventory_sequences (
  name TEXT PRIMARY KEY,
  next INTEGER NOT NULL
);

-- Published read views (Module Rules R2). Other modules read only these.
-- A product with a reorder rule but no stock movement yet shows with 0 on hand,
-- so it still appears in low-stock lists.
CREATE VIEW inventory_v_levels AS
  SELECT l.product_id, l.on_hand_milli, r.reorder_level_milli, r.reorder_qty_milli
  FROM inventory_levels l LEFT JOIN inventory_reorder_rules r ON r.product_id = l.product_id
  UNION ALL
  SELECT r.product_id, 0, r.reorder_level_milli, r.reorder_qty_milli
  FROM inventory_reorder_rules r
  WHERE r.product_id NOT IN (SELECT product_id FROM inventory_levels);
CREATE VIEW inventory_v_ledger AS SELECT * FROM inventory_ledger;
CREATE VIEW inventory_v_batches AS SELECT * FROM inventory_batches WHERE on_hand_milli <> 0;
