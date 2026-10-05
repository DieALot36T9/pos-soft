-- Customers, the points ledger, the credit ledger and the balances cache.
-- Page: docs/05 Modules/Module - customers.md

CREATE TABLE customers_customers (
  id TEXT PRIMARY KEY,
  phone TEXT NOT NULL UNIQUE,                    -- E.164, from normalize_phone
  name TEXT NOT NULL,
  email TEXT,
  address TEXT,
  tier_id TEXT,                                  -- pricing tier (no FK across modules)
  credit_limit_minor INTEGER NOT NULL DEFAULT 0,
  loyalty_opt_out INTEGER NOT NULL DEFAULT 0 CHECK (loyalty_opt_out IN (0, 1)),
  notes TEXT,
  archived_at INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX customers_name ON customers_customers(name);

-- Append-only.
CREATE TABLE customers_points_ledger (
  id TEXT PRIMARY KEY,
  customer_id TEXT NOT NULL REFERENCES customers_customers(id),
  points_delta INTEGER NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('earn', 'redeem', 'reverse', 'adjust')),
  ref_type TEXT,
  ref_id TEXT,
  reason TEXT,
  at INTEGER NOT NULL,
  user_id TEXT
);
CREATE INDEX customers_points_ledger_customer ON customers_points_ledger(customer_id, at);
CREATE INDEX customers_points_ledger_ref ON customers_points_ledger(ref_type, ref_id);

-- Append-only. A positive amount means the customer owes the shop.
-- A negative amount means the shop owes the customer (store credit).
CREATE TABLE customers_credit_ledger (
  id TEXT PRIMARY KEY,
  customer_id TEXT NOT NULL REFERENCES customers_customers(id),
  amount_minor INTEGER NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('credit_sale', 'payment', 'return', 'store_credit',
                                     'store_credit_used', 'adjust', 'void_reversal')),
  method_kind TEXT,
  reference TEXT,
  ref_type TEXT,
  ref_id TEXT,
  at INTEGER NOT NULL,
  business_date TEXT NOT NULL,
  user_id TEXT
);
CREATE INDEX customers_credit_ledger_customer ON customers_credit_ledger(customer_id, at);
CREATE INDEX customers_credit_ledger_ref ON customers_credit_ledger(ref_type, ref_id);

-- Ledger rows can never be changed or removed (Data Conventions rule 6).
CREATE TRIGGER customers_points_ledger_no_update BEFORE UPDATE ON customers_points_ledger
BEGIN SELECT RAISE(ABORT, 'customers_points_ledger is append-only'); END;
CREATE TRIGGER customers_points_ledger_no_delete BEFORE DELETE ON customers_points_ledger
BEGIN SELECT RAISE(ABORT, 'customers_points_ledger is append-only'); END;
CREATE TRIGGER customers_credit_ledger_no_update BEFORE UPDATE ON customers_credit_ledger
BEGIN SELECT RAISE(ABORT, 'customers_credit_ledger is append-only'); END;
CREATE TRIGGER customers_credit_ledger_no_delete BEFORE DELETE ON customers_credit_ledger
BEGIN SELECT RAISE(ABORT, 'customers_credit_ledger is append-only'); END;

-- A cache of the two ledgers, updated in the same transaction as each ledger row.
CREATE TABLE customers_balances (
  customer_id TEXT PRIMARY KEY REFERENCES customers_customers(id),
  points INTEGER NOT NULL DEFAULT 0,
  credit_minor INTEGER NOT NULL DEFAULT 0,
  oldest_unpaid_at INTEGER                       -- when the oldest unpaid debt started, for aging
);

-- Name search at the till. Phone search uses customers_customers.phone directly.
CREATE VIRTUAL TABLE customers_search USING fts5(
  customer_id UNINDEXED, name, tokenize = 'unicode61 remove_diacritics 2'
);

CREATE VIEW customers_v_customers AS
  SELECT c.id, c.phone, c.name, c.tier_id, c.credit_limit_minor,
         b.points, b.credit_minor, b.oldest_unpaid_at, c.archived_at
  FROM customers_customers c LEFT JOIN customers_balances b ON b.customer_id = c.id;

CREATE VIEW customers_v_credit_ledger AS SELECT * FROM customers_credit_ledger;
CREATE VIEW customers_v_points_ledger AS SELECT * FROM customers_points_ledger;
