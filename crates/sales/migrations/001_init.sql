-- Sales module tables: payment methods, invoice numbers, carts, completed sales,
-- returns and payments. Page: docs/05 Modules/Module - sales.md

CREATE TABLE sales_payment_methods (
  id TEXT PRIMARY KEY, name TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('cash','card','wallet','other','credit','points','exchange_credit')),
  requires_reference INTEGER NOT NULL DEFAULT 0 CHECK (requires_reference IN (0,1)),
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  sort INTEGER NOT NULL DEFAULT 0, is_system INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0,1))
);
-- Each system kind has exactly one method, so the app can find it by kind.
CREATE UNIQUE INDEX sales_payment_methods_system_kind ON sales_payment_methods(kind) WHERE is_system = 1;

-- The methods every shop has. Their ids are fixed so every database agrees.
-- The shop adds its own card, wallet and other methods in Settings (SET-15).
INSERT INTO sales_payment_methods (id, name, kind, requires_reference, active, sort, is_system) VALUES
  ('00000000000000000000000001', 'Cash',            'cash',            0, 1, 1,   1),
  ('00000000000000000000000002', 'Customer credit', 'credit',          0, 1, 900, 1),
  ('00000000000000000000000003', 'Loyalty points',  'points',          0, 1, 910, 1),
  ('00000000000000000000000004', 'Exchange credit', 'exchange_credit', 0, 1, 920, 1);

-- The next invoice or return number for each till. Rows are made on first use.
CREATE TABLE sales_sequences (
  till_id TEXT NOT NULL, doc TEXT NOT NULL CHECK (doc IN ('sale','return')),
  next INTEGER NOT NULL CHECK (next >= 1),
  PRIMARY KEY (till_id, doc)
);

-- Carts: saved on every change
CREATE TABLE sales_carts (
  id TEXT PRIMARY KEY, till_id TEXT NOT NULL, shift_id TEXT NOT NULL, user_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('open','held')),
  customer_id TEXT, price_list_id TEXT, note TEXT,
  invoice_discount_kind TEXT CHECK (invoice_discount_kind IN ('percent','amount')), invoice_discount_value INTEGER,
  exchange_return_id TEXT,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
-- A cashier has at most one open cart on a till.
CREATE UNIQUE INDEX sales_carts_one_open ON sales_carts(till_id, user_id) WHERE status = 'open';
CREATE INDEX sales_carts_till ON sales_carts(till_id, status);
CREATE INDEX sales_carts_shift ON sales_carts(shift_id, status);
CREATE TABLE sales_cart_lines (
  id TEXT PRIMARY KEY, cart_id TEXT NOT NULL REFERENCES sales_carts(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL, product_id TEXT NOT NULL, qty_milli INTEGER NOT NULL CHECK (qty_milli > 0),
  source TEXT NOT NULL CHECK (source IN ('scan','search','scale_label','manual')),
  embedded_price_minor INTEGER, price_override_minor INTEGER, override_by TEXT,
  manual_discount_kind TEXT CHECK (manual_discount_kind IN ('percent','amount')), manual_discount_value INTEGER,
  added_at INTEGER NOT NULL
);
CREATE INDEX sales_cart_lines_cart ON sales_cart_lines(cart_id, seq);

-- Completed sales: never updated except void fields
CREATE TABLE sales_sales (
  id TEXT PRIMARY KEY, invoice_no TEXT NOT NULL UNIQUE,
  till_id TEXT NOT NULL, shift_id TEXT NOT NULL, user_id TEXT NOT NULL, business_date TEXT NOT NULL, completed_at INTEGER NOT NULL,
  customer_id TEXT, customer_name TEXT, customer_phone TEXT, price_list_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('completed','voided')),
  base_minor INTEGER NOT NULL, promo_discount_minor INTEGER NOT NULL, tier_discount_minor INTEGER NOT NULL,
  manual_discount_minor INTEGER NOT NULL, invoice_discount_minor INTEGER NOT NULL,
  net_minor INTEGER NOT NULL, tax_minor INTEGER NOT NULL, gross_minor INTEGER NOT NULL,
  rounding_minor INTEGER NOT NULL, total_minor INTEGER NOT NULL, change_minor INTEGER NOT NULL,
  cost_minor INTEGER NOT NULL,
  tax_mode TEXT NOT NULL CHECK (tax_mode IN ('inclusive','exclusive')), exchange_return_id TEXT,
  voided_at INTEGER, voided_by TEXT, void_approved_by TEXT, void_reason TEXT
);
CREATE INDEX sales_sales_date ON sales_sales(business_date);
CREATE INDEX sales_sales_shift ON sales_sales(shift_id);
CREATE INDEX sales_sales_customer ON sales_sales(customer_id, completed_at);
CREATE INDEX sales_sales_till ON sales_sales(till_id, completed_at);
CREATE TABLE sales_lines (
  id TEXT PRIMARY KEY, sale_id TEXT NOT NULL REFERENCES sales_sales(id), seq INTEGER NOT NULL,
  product_id TEXT NOT NULL, name TEXT NOT NULL, sku TEXT NOT NULL, unit_symbol TEXT NOT NULL,
  category_id TEXT NOT NULL, brand_id TEXT,
  qty_milli INTEGER NOT NULL, unit_price_minor INTEGER NOT NULL, retail_price_minor INTEGER NOT NULL,
  base_minor INTEGER NOT NULL, promotion_id TEXT, promotion_name TEXT, promo_discount_minor INTEGER NOT NULL,
  tier_discount_minor INTEGER NOT NULL, manual_discount_minor INTEGER NOT NULL, invoice_discount_minor INTEGER NOT NULL,
  net_minor INTEGER NOT NULL, tax_minor INTEGER NOT NULL, gross_minor INTEGER NOT NULL,
  tax_rate_id TEXT NOT NULL, tax_name TEXT NOT NULL, tax_bps INTEGER NOT NULL, tax_components_json TEXT NOT NULL,
  unit_cost_minor INTEGER NOT NULL, cost_minor INTEGER NOT NULL, price_overridden INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX sales_lines_sale ON sales_lines(sale_id);
CREATE INDEX sales_lines_product ON sales_lines(product_id);

CREATE TABLE sales_returns (
  id TEXT PRIMARY KEY, return_no TEXT NOT NULL UNIQUE, sale_id TEXT REFERENCES sales_sales(id),
  till_id TEXT NOT NULL, shift_id TEXT NOT NULL, user_id TEXT NOT NULL, approved_by TEXT,
  business_date TEXT NOT NULL, at INTEGER NOT NULL, customer_id TEXT, reason TEXT,
  net_minor INTEGER NOT NULL, tax_minor INTEGER NOT NULL, gross_minor INTEGER NOT NULL,
  rounding_minor INTEGER NOT NULL, total_minor INTEGER NOT NULL, cost_minor INTEGER NOT NULL
);
CREATE INDEX sales_returns_sale ON sales_returns(sale_id);
CREATE TABLE sales_return_lines (
  id TEXT PRIMARY KEY, return_id TEXT NOT NULL REFERENCES sales_returns(id), sale_line_id TEXT REFERENCES sales_lines(id),
  product_id TEXT NOT NULL, name TEXT NOT NULL, qty_milli INTEGER NOT NULL,
  net_minor INTEGER NOT NULL, tax_minor INTEGER NOT NULL, gross_minor INTEGER NOT NULL,
  tax_rate_id TEXT NOT NULL, tax_bps INTEGER NOT NULL, unit_cost_minor INTEGER NOT NULL, cost_minor INTEGER NOT NULL,
  restock INTEGER NOT NULL CHECK (restock IN (0,1))
);
CREATE INDEX sales_return_lines_return ON sales_return_lines(return_id);
CREATE INDEX sales_return_lines_sale_line ON sales_return_lines(sale_line_id);

CREATE TABLE sales_payments (
  id TEXT PRIMARY KEY, doc_type TEXT NOT NULL CHECK (doc_type IN ('sale','return')), doc_id TEXT NOT NULL,
  method_id TEXT NOT NULL REFERENCES sales_payment_methods(id), kind TEXT NOT NULL,
  amount_minor INTEGER NOT NULL, tendered_minor INTEGER, reference TEXT, at INTEGER NOT NULL
);
CREATE INDEX sales_payments_doc ON sales_payments(doc_type, doc_id);

-- A completed sale is never changed, except that it can be voided once.
CREATE TRIGGER sales_sales_keep_contents BEFORE UPDATE OF
  id, invoice_no, till_id, shift_id, user_id, business_date, completed_at,
  customer_id, customer_name, customer_phone, price_list_id,
  base_minor, promo_discount_minor, tier_discount_minor, manual_discount_minor, invoice_discount_minor,
  net_minor, tax_minor, gross_minor, rounding_minor, total_minor, change_minor, cost_minor,
  tax_mode, exchange_return_id
ON sales_sales BEGIN SELECT RAISE(ABORT, 'a completed sale is never changed'); END;
CREATE TRIGGER sales_sales_void_once BEFORE UPDATE ON sales_sales
  WHEN OLD.status <> 'completed' OR NEW.status <> 'voided'
BEGIN SELECT RAISE(ABORT, 'only a completed sale can be voided, and only once'); END;
CREATE TRIGGER sales_sales_no_delete BEFORE DELETE ON sales_sales
BEGIN SELECT RAISE(ABORT, 'a completed sale is never deleted'); END;
CREATE TRIGGER sales_lines_no_update BEFORE UPDATE ON sales_lines
BEGIN SELECT RAISE(ABORT, 'sale lines are never changed'); END;
CREATE TRIGGER sales_lines_no_delete BEFORE DELETE ON sales_lines
BEGIN SELECT RAISE(ABORT, 'sale lines are never deleted'); END;
CREATE TRIGGER sales_returns_no_update BEFORE UPDATE ON sales_returns
BEGIN SELECT RAISE(ABORT, 'returns are never changed'); END;
CREATE TRIGGER sales_returns_no_delete BEFORE DELETE ON sales_returns
BEGIN SELECT RAISE(ABORT, 'returns are never deleted'); END;
CREATE TRIGGER sales_return_lines_no_update BEFORE UPDATE ON sales_return_lines
BEGIN SELECT RAISE(ABORT, 'return lines are never changed'); END;
CREATE TRIGGER sales_return_lines_no_delete BEFORE DELETE ON sales_return_lines
BEGIN SELECT RAISE(ABORT, 'return lines are never deleted'); END;
CREATE TRIGGER sales_payments_no_update BEFORE UPDATE ON sales_payments
BEGIN SELECT RAISE(ABORT, 'payments are never changed'); END;
CREATE TRIGGER sales_payments_no_delete BEFORE DELETE ON sales_payments
BEGIN SELECT RAISE(ABORT, 'payments are never deleted'); END;

-- Published views (the module's public contract for reports)
CREATE VIEW sales_v_lines AS            -- for reports: sale lines with header fields
  SELECT l.*, s.invoice_no, s.business_date, s.completed_at, s.till_id, s.shift_id, s.user_id, s.customer_id, s.status
  FROM sales_lines l JOIN sales_sales s ON s.id = l.sale_id;
CREATE VIEW sales_v_sales AS SELECT * FROM sales_sales;
CREATE VIEW sales_v_return_lines AS
  SELECT rl.*, r.return_no, r.business_date, r.at, r.till_id, r.shift_id, r.user_id, r.customer_id, r.sale_id
  FROM sales_return_lines rl JOIN sales_returns r ON r.id = rl.return_id;
CREATE VIEW sales_v_payments AS
  SELECT p.*, COALESCE(s.business_date, r.business_date) AS business_date, COALESCE(s.shift_id, r.shift_id) AS shift_id,
         COALESCE(s.status, 'completed') AS doc_status
  FROM sales_payments p
  LEFT JOIN sales_sales s ON p.doc_type = 'sale' AND s.id = p.doc_id
  LEFT JOIN sales_returns r ON p.doc_type = 'return' AND r.id = p.doc_id;
