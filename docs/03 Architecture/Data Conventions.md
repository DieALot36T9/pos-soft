---
tags: [architecture, database, conventions]
---
# Data Conventions

## Column types and names
| Kind | SQLite type | Rust type | Column name pattern | Example |
|---|---|---|---|---|
| Identifier | `TEXT` (26-char ULID) | `core::Id` | `id`, `<thing>_id` | `product_id` |
| Money | `INTEGER` (minor units) | `core::Money` (i64) | `*_minor` | `sale_price_minor` |
| Quantity | `INTEGER` (thousandths) | `core::Qty` (i64) | `*_milli` | `qty_milli` |
| Rate / percent | `INTEGER` (basis points) | `core::Bps` (u32) | `*_bps` | `tax_bps` |
| Points | `INTEGER` | `i64` | `points`, `points_delta` | |
| Timestamp | `INTEGER` (Unix ms, UTC) | `core::Timestamp` | `*_at` | `completed_at` |
| Business date | `TEXT` `YYYY-MM-DD` | `core::BusinessDate` | `business_date` | |
| Calendar date | `TEXT` `YYYY-MM-DD` | `chrono::NaiveDate` | `*_date` | `expiry_date` |
| Boolean | `INTEGER` 0/1 with `CHECK` | `bool` | `is_*`, `has_*`, or adjective | `is_weighed`, `active` |
| Enum | `TEXT` with `CHECK (x IN (...))` | Rust enum | singular noun | `kind`, `status` |
| JSON details | `TEXT` | `serde_json::Value` | `*_json` | `details_json` |
| Archive marker | `INTEGER NULL` (timestamp) | `Option<Timestamp>` | `archived_at` | |

## Rules
1. **Every table has `id TEXT PRIMARY KEY`** (ULID), except pure link tables and caches keyed by another id.
2. **Money columns are `NOT NULL`** (default 0). Nullable money is never used to mean "zero".
3. **Signs:** ledger deltas are signed. Document amounts (sale totals) are positive. Direction comes from the document type.
4. **Who and when:** tables that record actions have `created_at` and `created_by` (user id). Ledgers have `at`, `user_id`, `business_date`.
5. **Archive, don't delete,** for anything history refers to (products, customers, suppliers, users, tax rates, categories).
6. **Ledgers are append-only:** every `*_ledger` table and `audit_events` gets triggers that `RAISE(ABORT)` on UPDATE or DELETE.
7. **Snapshots:** documents (sales, returns, GRNs) copy the names, prices, tax rates and costs they used ([[ADR-011 Snapshots on sale lines]]).
8. **Text** is stored as given (UTF-8). Searching uses FTS5 with the `unicode61` tokenizer and `remove_diacritics 2`.
9. **Phone numbers** are stored in **E.164** form (e.g. `+8801711123456`), cleaned with the `phonenumber` crate using the shop's country as the default ([[Reuse Map]]).
10. **Enum values** are lower snake case (`buy_x_get_y`). Rust enums map them with `serde(rename_all = "snake_case")`.

## Document numbers
| Document | Format | Sequence scope |
|---|---|---|
| Sale | `{till}-{n:06}` e.g. `T1-000123` | per till |
| Return | `{till}-R{n:06}` | per till |
| Purchase order | `PO-{n:06}` | per database |
| GRN | `GRN-{n:06}` | per database |
| Supplier return | `SR-{n:06}` | per database |
| Stock count | `SC-{n:06}` | per database |

Numbers come from a sequence table owned by the module, incremented **inside the same transaction** that creates the document, so they're gapless.

## Time
- All timestamps are stored in UTC. The UI and reports show local time using the OS time zone.
- **Business date** = local date of (`now` − cutoff hours). Calculated in exactly one function: `core::business_date(now, offset, cutoff_hour)`. [[ADR-016 Business day cutoff]]
- A `Clock` trait in `core` provides "now", so tests can control time.
