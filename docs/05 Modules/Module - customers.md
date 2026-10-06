---
tags: [module]
crate: customers
milestone: M8
---
# Module: customers

**Purpose:** customer records, loyalty points ledger, credit (baki) and store-credit ledger, balances.

**Requirements:** [[Req - Customers and Loyalty]], [[Req - Payments]] PAY-04, PAY-05. **Depends on:** `core`, `rusqlite`, `serde`, `thiserror`, and **`phonenumber`** (approved here: E.164 cleaning with libphonenumber's rules, [[Reuse Map]]).

## Tables
Migration `migrations/001_init.sql` (built in B-customers):
```sql
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
```
Changes from the first plan, and why:
- **Append-only triggers** on both ledgers, as [[Data Conventions]] rule 6 asks.
- **`customers_search` holds only the name.** Phone search reads `customers_customers.phone` with `LIKE '%digits%'`, so the cashier can type any part of the number (start, middle or the last digits) and a local `017…` still finds `+88017…`. A full-text prefix search can't match the middle or end of a number. The tokenizer has `remove_diacritics 2` ([[Data Conventions]] rule 8).
- **Indexes on `(ref_type, ref_id)`** for voids, and on the points ledger by customer.
- **`customers_v_customers` also has `oldest_unpaid_at`** (for the aging report), and there is a **`customers_v_points_ledger`** view for reports.

## Public API
All functions return `Result<_, CustomersError>`. `CustomersError` turns into `AppError` with `?` (codes listed in `src/error.rs`, e.g. `phone_invalid`, `phone_in_use`, `points_not_enough`).
```rust
// records (CUS-01, CUS-02)
pub fn normalize_phone(raw: &str, default_country: &str) -> Result<String>;   // E.164 via `phonenumber`. "01711-123456", "BD" → "+8801711123456"
pub fn create(tx, details: CustomerDetails, default_country: &str, now: Timestamp) -> Result<Id>;
pub fn update(tx, &Id, details: CustomerDetails, default_country: &str, now: Timestamp) -> Result<()>;  // replaces all fields
pub fn set_archived(tx, &Id, archived: bool, now: Timestamp) -> Result<()>;   // archive, never delete
pub fn get(conn, &Id) -> Result<CustomerWithBalances>;
pub fn lookup(conn, typed: &str, limit: u32) -> Result<Vec<CustomerHit>>;     // part of the phone, or the start of name words; archived left out

// ledgers (CUS-04, CUS-05, CUS-06)
pub fn add_points(tx, &Id, delta: i64, kind: PointsKind, ref_: Option<&LedgerRef>, reason: Option<&str>, ctx: &LedgerCtx) -> Result<i64>;   // new balance
pub fn add_credit(tx, &Id, amount: Money, kind: CreditKind, method: Option<&PaymentMethod>, ref_: Option<&LedgerRef>, ctx: &LedgerCtx) -> Result<Money>;  // new balance
pub fn pay_with_credit(tx, &Id, amount: Money, ref_: Option<&LedgerRef>, ctx: &LedgerCtx) -> Result<CreditSplit>;  // store credit first, then new debt
pub fn reverse_ref(tx, ref_type: &str, ref_id: &Id, ctx: &LedgerCtx) -> Result<()>;   // void: opposite rows, once
pub fn statement(conn, &Id, from: BusinessDate, to: BusinessDate) -> Result<Statement>;  // by business date, both days included
pub fn migrations() -> ModuleMigrations;

// pure
pub fn eligible_for_points(total_due: Money, paid_by_points: Money, paid_by_credit: Money) -> Result<Money>;  // never below 0
pub fn points_earned(eligible: Money, s: &LoyaltySettingsView) -> i64;               // floor(eligible / spend_unit) × points_per_unit; 0 if off
pub fn points_value(points: i64, s: &LoyaltySettingsView) -> Result<Money>;          // points × value_per_point
pub fn max_points_to_redeem(balance: i64, amount_due: Money, s: &LoyaltySettingsView) -> i64;  // PAY-05; 0 under min_redeem
pub fn split_credit_payment(balance: Money, amount: Money) -> Result<CreditSplit>;   // for the credit-limit check in `app`
pub fn oldest_unpaid_at(rows: &[(Timestamp, Money)]) -> Option<Timestamp>;          // first in, first out
```
**Types** (`src/types.rs`): `CustomerDetails` (phone, name, email, address, tier_id, credit_limit, loyalty_opt_out, notes; `CustomerDetails::quick(phone, name)` for quick create), `Customer`, `CustomerWithBalances` (customer, points, credit, oldest_unpaid_at), `CustomerHit` (id, phone, name, tier_id, points, credit), `LedgerCtx` (at, business_date, user_id), `LedgerRef` (ref_type, ref_id), `PaymentMethod` (method_kind, reference), `PointsKind`, `CreditKind`, `LoyaltySettingsView` (enabled, spend_unit, points_per_unit, value_per_point, min_redeem: `app` copies it from `settings`' `LoyaltySettings`), `CreditSplit` (store_credit_used, new_debt), `Statement` (customer_id, from, to, opening, lines, closing), `StatementLine` (… balance_after).

**Ledger rules**
| Kind | Sign | Refused when |
|---|---|---|
| points `earn` | + | the customer opted out of loyalty |
| points `redeem` | − | the balance would go below 0 |
| points `adjust` | ± | no reason given (CUS-04), or the balance would go below 0. `app` checks `LoyaltyAdjust` |
| points `reverse` | ± | never: a void or return may take the balance below 0 |
| credit `credit_sale`, `store_credit_used` | + | `store_credit_used` more than the store credit left |
| credit `payment`, `return`, `store_credit` | − | `payment` more than the debt |
| credit `adjust`, `void_reversal` | ± | – |

A zero amount is always refused. Credit limit checks happen in the workflow (it knows the sale amount and the permission): use `split_credit_payment` to see how much new debt a payment makes.

**Voids:** `reverse_ref` writes a `reverse` points row (reason `"void"`) and a `void_reversal` credit row for every row of the document. A second call for the same document is refused (`already_reversed`). A document with no customer rows changes nothing.

**Oldest unpaid debt** (`customers_balances.oldest_unpaid_at`): payments pay off the oldest debt first. Money beyond the debt is store credit and pays the next debts first. It is worked out again from the customer's ledger on every credit change, in the same transaction.

## Tests
- `tests/contract.rs`: phone normalization (`+880 1711-123456` → `+8801711123456`) and uniqueness (CUS-01); create, update, archive; lookup by part of a phone and by name prefixes, limit, odd input (CUS-02); the published views.
- `tests/ledgers.rs`: points rows, signs, reasons, opt-out (CUS-04); credit sales, payments, statement with opening and closing balances, oldest unpaid debt (CUS-05); **store credit used before new debt** (CUS-06); voids; ledgers are append-only (ADR-010).
- `tests/properties.rs` (proptest): **balances equal ledger sums after random operations**, and something is owed exactly when there is an oldest unpaid debt.
- Unit tests in `src/points.rs`: **CUS-04 example / E16** (1 point per 100.00 on 1,250.00 cash → 12 points); **earned points exclude points/credit-paid amounts**; PAY-05 (1,200 points at 0.10 → up to 120.00, whole points, minimum to redeem).
- Unit tests in `src/credit.rs`, `src/lookup.rs`, `src/phone.rs`, `src/error.rs`: store credit split, first-in-first-out aging, search query building, phone edge cases, error JSON.
- Every test uses `tests/support::test_db()`: an in-memory database built only from this module's migrations.

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
- Ledgers are append-only (triggers refuse UPDATE and DELETE).
- Points and credit functions take `&Transaction` and never commit. If one returns an error after writing, `app` must roll the transaction back (all checks run before the first write, so refused calls write nothing).
- The phone search treats typed digits without their leading zeros, so `01711` finds `+8801711…`.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Built in task B-customers: tables, public API and tests above.
- **Connected in C-M8:** `app` attaches customers at the till, records points and credit when a sale completes, takes debt payments, corrects points, and shows the customer screens. The types the screens use derive `ts_rs::TS`.
- **Not here by design:** purchase history, total spend and visit count (CUS-03) come from `sales`' views, built by `app`/`reports`. The receivables aging report (CUS-05) and receipt fields (CUS-08, masked phone) belong to `reports` and `printing`; aging can use `customers_v_customers.oldest_unpaid_at` or walk `customers_v_credit_ledger`.
- **Decided in C-M8:** a back-office payment larger than the debt stays refused (`payment_more_than_debt`); advances can be added later as store credit if shops ask. A `return` row may take the balance below zero; the workflow should normally use `store_credit` for that (C-M9 does: `return` only up to the debt, the rest `store_credit`).
- **Used by returns (C-M9):** a return writes `return` and `store_credit` credit rows and `reverse` points rows (reason `"return"`: points paid given back, points earned taken back) pointing at `("return", id)`. `reverse_ref`'s "already reversed" check looks only at reason `"void"`, so a return never blocks a later void of another document. `app` reads `customers_v_points_ledger` to find the points earned on a sale.
- **Decided in C-M8:** archived customers can't be attached at the till (`app` refuses with `customer_archived`), so they get no new credit sales or points; their ledgers keep working.
- `lookup` has no "include archived" option; the back-office list (C-M8) reads `customers_v_customers` in `app` instead.
