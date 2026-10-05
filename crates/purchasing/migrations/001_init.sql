-- Purchasing: suppliers, purchase orders, goods received (GRN), supplier returns
-- and the payables ledger. Page: docs/05 Modules/Module - purchasing.md.

CREATE TABLE purchasing_suppliers (
  id TEXT PRIMARY KEY, name TEXT NOT NULL, contact_person TEXT, phone TEXT, email TEXT, address TEXT,
  tax_number TEXT, payment_terms_days INTEGER NOT NULL DEFAULT 0, notes TEXT,
  archived_at INTEGER, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);

CREATE TABLE purchasing_orders (
  id TEXT PRIMARY KEY, number TEXT NOT NULL UNIQUE, supplier_id TEXT NOT NULL REFERENCES purchasing_suppliers(id),
  status TEXT NOT NULL CHECK (status IN ('draft','sent','partial','received','cancelled')),
  expected_date TEXT, notes TEXT, created_at INTEGER NOT NULL, created_by TEXT NOT NULL, sent_at INTEGER
);

CREATE TABLE purchasing_order_lines (
  id TEXT PRIMARY KEY, order_id TEXT NOT NULL REFERENCES purchasing_orders(id),
  product_id TEXT NOT NULL, product_name TEXT NOT NULL, sku TEXT NOT NULL,
  qty_milli INTEGER NOT NULL, unit_cost_minor INTEGER NOT NULL, received_milli INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX purchasing_order_lines_order ON purchasing_order_lines(order_id);

CREATE TABLE purchasing_grns (
  id TEXT PRIMARY KEY, number TEXT NOT NULL UNIQUE, supplier_id TEXT NOT NULL REFERENCES purchasing_suppliers(id),
  order_id TEXT REFERENCES purchasing_orders(id), supplier_invoice_no TEXT,
  invoice_total_minor INTEGER NOT NULL, lines_total_minor INTEGER NOT NULL,
  received_at INTEGER NOT NULL, business_date TEXT NOT NULL, user_id TEXT NOT NULL, notes TEXT
);

CREATE TABLE purchasing_grn_lines (
  id TEXT PRIMARY KEY, grn_id TEXT NOT NULL REFERENCES purchasing_grns(id), order_line_id TEXT REFERENCES purchasing_order_lines(id),
  product_id TEXT NOT NULL, product_name TEXT NOT NULL, qty_milli INTEGER NOT NULL, unit_cost_minor INTEGER NOT NULL,
  batch_no TEXT, expiry_date TEXT, inventory_batch_id TEXT
);
CREATE INDEX purchasing_grn_lines_grn ON purchasing_grn_lines(grn_id);

CREATE TABLE purchasing_returns (
  id TEXT PRIMARY KEY, number TEXT NOT NULL UNIQUE, supplier_id TEXT NOT NULL REFERENCES purchasing_suppliers(id),
  grn_id TEXT REFERENCES purchasing_grns(id), total_minor INTEGER NOT NULL, reason TEXT,
  at INTEGER NOT NULL, business_date TEXT NOT NULL, user_id TEXT NOT NULL
);

CREATE TABLE purchasing_return_lines (
  id TEXT PRIMARY KEY, return_id TEXT NOT NULL REFERENCES purchasing_returns(id),
  product_id TEXT NOT NULL, product_name TEXT NOT NULL, qty_milli INTEGER NOT NULL, unit_cost_minor INTEGER NOT NULL, inventory_batch_id TEXT
);

-- Append-only (ADR-010). A positive amount means the shop owes the supplier more.
CREATE TABLE purchasing_ledger (
  id TEXT PRIMARY KEY, supplier_id TEXT NOT NULL REFERENCES purchasing_suppliers(id),
  amount_minor INTEGER NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('invoice','payment','return_credit','adjust')),
  method_kind TEXT, reference TEXT, ref_type TEXT, ref_id TEXT,
  due_date TEXT, at INTEGER NOT NULL, business_date TEXT NOT NULL, user_id TEXT NOT NULL
);
CREATE INDEX purchasing_ledger_supplier ON purchasing_ledger(supplier_id, at);

CREATE TRIGGER purchasing_ledger_no_update BEFORE UPDATE ON purchasing_ledger
BEGIN SELECT RAISE(ABORT, 'purchasing_ledger is append-only'); END;
CREATE TRIGGER purchasing_ledger_no_delete BEFORE DELETE ON purchasing_ledger
BEGIN SELECT RAISE(ABORT, 'purchasing_ledger is append-only'); END;

-- Cache: the sum of each supplier's ledger rows, updated in the same transaction.
CREATE TABLE purchasing_balances (supplier_id TEXT PRIMARY KEY, balance_minor INTEGER NOT NULL);

-- Gapless document numbers: 'po', 'grn', 'sr'.
CREATE TABLE purchasing_sequences (doc TEXT PRIMARY KEY, next INTEGER NOT NULL);

CREATE VIEW purchasing_v_payables AS
  SELECT s.id AS supplier_id, s.name, s.payment_terms_days, COALESCE(b.balance_minor, 0) AS balance_minor
  FROM purchasing_suppliers s LEFT JOIN purchasing_balances b ON b.supplier_id = s.id;
CREATE VIEW purchasing_v_ledger AS SELECT * FROM purchasing_ledger;
CREATE VIEW purchasing_v_grn_lines AS
  SELECT gl.*, g.number, g.supplier_id, g.received_at, g.business_date
  FROM purchasing_grn_lines gl JOIN purchasing_grns g ON g.id = gl.grn_id;
