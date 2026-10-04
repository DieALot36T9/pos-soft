---
tags: [module]
crate: customers
milestone: M8
---
# Module: customers

**Purpose:** customer records, loyalty points ledger, credit (baki) and store-credit ledger, balances.

**Requirements:** [[Req - Customers and Loyalty]]. **Depends on:** `core`, `rusqlite`.

## Tables
```sql
CREATE TABLE customers_customers (
  id TEXT PRIMARY KEY, phone TEXT NOT NULL UNIQUE, name TEXT NOT NULL, email TEXT, address TEXT,
  tier_id TEXT,                                  -- pricing (no FK)
  credit_limit_minor INTEGER NOT NULL DEFAULT 0,
  loyalty_opt_out INTEGER NOT NULL DEFAULT 0, notes TEXT,
  archived_at INTEGER, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE INDEX customers_name ON customers_customers(name);
CREATE TABLE customers_points_ledger (           -- append-only
  id TEXT PRIMARY KEY, customer_id TEXT NOT NULL REFERENCES customers_customers(id),
  points_delta INTEGER NOT NULL, kind TEXT NOT NULL CHECK (kind IN ('earn','redeem','reverse','adjust')),
  ref_type TEXT, ref_id TEXT, reason TEXT, at INTEGER NOT NULL, user_id TEXT
);
CREATE TABLE customers_credit_ledger (           -- append-only; + = customer owes shop, − = shop owes customer
  id TEXT PRIMARY KEY, customer_id TEXT NOT NULL REFERENCES customers_customers(id),
  amount_minor INTEGER NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('credit_sale','payment','return','store_credit','store_credit_used','adjust','void_reversal')),
  method_kind TEXT, reference TEXT, ref_type TEXT, ref_id TEXT, at INTEGER NOT NULL, business_date TEXT NOT NULL, user_id TEXT
);
CREATE INDEX customers_credit_ledger_customer ON customers_credit_ledger(customer_id, at);
CREATE TABLE customers_balances (
  customer_id TEXT PRIMARY KEY REFERENCES customers_customers(id),
  points INTEGER NOT NULL DEFAULT 0, credit_minor INTEGER NOT NULL DEFAULT 0,
  oldest_unpaid_at INTEGER                       -- for aging, maintained on change
);
CREATE VIRTUAL TABLE customers_search USING fts5(customer_id UNINDEXED, phone, name, tokenize='unicode61');
CREATE VIEW customers_v_customers AS
  SELECT c.id, c.phone, c.name, c.tier_id, c.credit_limit_minor, b.points, b.credit_minor, c.archived_at
  FROM customers_customers c LEFT JOIN customers_balances b ON b.customer_id = c.id;
CREATE VIEW customers_v_credit_ledger AS SELECT * FROM customers_credit_ledger;
```

## Public API
```rust
pub fn normalize_phone(raw: &str) -> Result<String, AppError>;
pub fn create(tx, NewCustomer, now) -> Result<Id>;
pub fn update(tx, &Id, CustomerUpdate, now) -> Result<()>;
pub fn lookup(conn, q: &str, limit: u32) -> Result<Vec<CustomerHit>>;   // phone prefix or name
pub fn get(conn, &Id) -> Result<CustomerWithBalances>;
pub fn add_points(tx, &Id, delta: i64, kind, ref_, reason, ctx) -> Result<i64 /*new balance*/>;
pub fn add_credit(tx, &Id, delta: Money, kind, method, ref_, ctx) -> Result<Money>;
pub fn reverse_ref(tx, ref_type: &str, ref_id: &Id, ctx) -> Result<()>;  // void
pub fn statement(conn, &Id, from, to) -> Result<Statement>;
pub fn migrations() -> ModuleMigrations;
// pure
pub fn points_earned(eligible: Money, s: &LoyaltySettingsView) -> i64;
pub fn points_value(points: i64, s: &LoyaltySettingsView) -> Money;
```
Credit limit checks happen in the workflow (it knows the sale amount and permission).

## Tests
- Phone normalization (`+880 1711-123456` → `+8801711123456`) and uniqueness.
- CUS-04 example; earned points exclude points/credit-paid amounts.
- Balances equal ledger sums after random operations (proptest).
- Store credit used before new debt.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/customers/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/customers/` |
| **Screen folder** | `ui/src/modules/customers/` |
| **Requirements** | [[Req - Customers and Loyalty]] CUS-01 to CUS-08; [[Req - Payments]] PAY-04, PAY-05 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Loyalty settings and `MoveCtx`-style context from `app`. Credit-limit checks are done by `app` |
| **Used by** (re-run their tests when the public API changes) | Attaching a customer at the till, completing a sale, returns, voids, customer payments, reports |
| **Test it on its own** | `cargo nextest run -p customers` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Phone numbers are cleaned in one function, `normalize_phone`. Always use it.
- Balances (points, credit) are a cache of the ledgers, updated in the same transaction.
- Ledgers are append-only.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Not started (see [[Task Index]]).
