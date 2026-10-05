---
tags: [module]
crate: audit
milestone: M3
---
# Module: audit

**Purpose:** an append-only record of sensitive actions.

**Requirements:** [[Req - Users and Security]] USR-08. **Depends on:** `core`, `rusqlite`, `serde`, `serde_json`, `thiserror`, `ts-rs` (types for the audit screen).

## Tables
```sql
CREATE TABLE audit_events (
  id             TEXT PRIMARY KEY,
  at             INTEGER NOT NULL,
  business_date  TEXT NOT NULL,
  till_id        TEXT NOT NULL,
  user_id        TEXT,                 -- NULL for system events (backup timer)
  approved_by    TEXT,                 -- override approver
  action         TEXT NOT NULL,        -- 'sale.void', 'price.change', 'drawer.no_sale', ...
  entity_type    TEXT,
  entity_id      TEXT,
  details_json   TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX audit_events_at ON audit_events(at);
CREATE INDEX audit_events_action ON audit_events(action, at);
CREATE INDEX audit_events_user ON audit_events(user_id, at);
CREATE TRIGGER audit_events_no_update BEFORE UPDATE ON audit_events BEGIN SELECT RAISE(ABORT, 'audit is append-only'); END;
CREATE TRIGGER audit_events_no_delete BEFORE DELETE ON audit_events BEGIN SELECT RAISE(ABORT, 'audit is append-only'); END;
CREATE VIEW audit_v_events AS SELECT * FROM audit_events;
```
File: `crates/audit/migrations/001_init.sql`.

## Public API
Every function returns `Result<_, pos_core::AppError>`.
```rust
pub struct AuditEvent { pub action: &'static str, pub entity_type: Option<&'static str>, pub entity_id: Option<Id>,
                        pub details: serde_json::Value, pub user_id: Option<Id>, pub approved_by: Option<Id> }
pub struct AuditCtx { pub at: Timestamp, pub business_date: BusinessDate, pub till_id: Id }
pub fn record(tx: &Transaction, ev: AuditEvent, ctx: &AuditCtx) -> Result<Id, AppError>;

pub struct AuditFilter { pub from: Option<BusinessDate>, pub to: Option<BusinessDate>,   // business dates, both ends included
                         pub user_id: Option<Id>, pub action: Option<String> }           // Default = no filter
pub struct AuditPage { pub limit: u32, pub offset: u32 }   // AuditPage::MAX_LIMIT = 500
pub struct AuditRow { pub id: Id, pub at: Timestamp, pub business_date: BusinessDate, pub till_id: Id,
                      pub user_id: Option<Id>, pub approved_by: Option<Id>, pub action: String,
                      pub entity_type: Option<String>, pub entity_id: Option<Id>, pub details: serde_json::Value }
pub fn query(conn: &Connection, f: &AuditFilter, page: AuditPage) -> Result<Vec<AuditRow>, AppError>;
pub fn migrations() -> ModuleMigrations;
pub enum AuditError { Sqlite, BadAction, DetailsNotObject, SecretInDetails, BadPageLimit, BadStoredRow }  // → AppError
```
- `record` writes inside the caller's transaction. If the caller rolls back, the event is gone too.
- `record` refuses (as `AppError::Internal`, because it's a bug in the caller):
  - an action that isn't lower-case words joined by dots (`sale.void`, `drawer.no_sale`);
  - `details` that aren't a JSON object;
  - `details` with a secret-sounding key at any depth: `password`, `pin`, `recovery_code`, `override_token`, `token`, `secret`. The error names the key, never the value.
- `query` returns the newest first (`at`, then the order they were recorded). A page `limit` of 0 or over 500 is `AppError::validation("page_limit_invalid")`.
- `AuditFilter`, `AuditPage` and `AuditRow` derive `ts_rs::TS` (in `ui/src/generated/`). `details` is typed `Record<string, unknown>` there. The page type is `AuditPage`, not `Page`, so it can't clash with another module's generated `Page.ts`.

Action names are constants in `app::audit_actions` so they're listed in one place.

## Tests
All in `crates/audit/`. Every test name starts with `usr_08_` (requirement USR-08).
- `tests/contract.rs`:
  - UPDATE and DELETE abort (`usr_08_nobody_can_change_an_event`, `usr_08_nobody_can_delete_an_event`).
  - `record` then `query` gives back every field; system events have no user.
  - Filtering by business date range (both ends included), user, action, and all of them together.
  - Paging walks through every event once; newest first; events at the same moment keep a fixed order.
  - Page limit checked; a rolled-back transaction leaves no event; bad action names, non-object details and secrets in details are refused.
  - `audit_v_events` publishes every column; dates are stored as text and details as JSON text.
  - `AuditRow` is sent to the UI in snake case.
- Unit tests in `src/record.rs`: action name check, secret keys found inside lists, ordinary details pass.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/audit/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/audit/` |
| **Screen folder** | `ui/src/modules/audit/` |
| **Requirements** | [[Req - Users and Security]] USR-08 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | `AuditCtx` (time, business date, till, user) from `app` |
| **Used by** (re-run their tests when the public API changes) | Nearly every workflow |
| **Test it on its own** | `cargo nextest run -p audit` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- The table is append-only. Database triggers block UPDATE and DELETE; never work around them.
- Action names live in `app/src/audit_actions.rs`, not here.
- Never put secrets in `details`.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Built in task B-audit (Rust only). **No `app` wiring and no screens yet:** C-M3 adds `app/src/audit_actions.rs`, the `audit_query` command, the audit screen and the Excel export.
- Filters use `(:x IS NULL OR column = :x)`, which is simple but can't always use the indexes. Measure with a big seeded log (`xtask seed`) before adding a `business_date` index or building the SQL per filter.
- If another module needs the same paging shape (catalog and inventory pages say `Page`), consider one shared `Page` in `core` and switch `AuditPage` to it (rule R5).
- The secret-key check is a safety net, not a guarantee. Callers still must not put secrets in `details`.
