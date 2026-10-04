---
tags: [module]
crate: audit
milestone: M3
---
# Module: audit

**Purpose:** an append-only record of sensitive actions.

**Requirements:** [[Req - Users and Security]] USR-08. **Depends on:** `core`, `rusqlite`, `serde_json`.

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

## Public API
```rust
pub struct AuditEvent { pub action: &'static str, pub entity_type: Option<&'static str>, pub entity_id: Option<Id>, pub details: serde_json::Value, pub user_id: Option<Id>, pub approved_by: Option<Id> }
pub fn record(tx: &Transaction, ev: AuditEvent, ctx: &AuditCtx /* at, business_date, till_id */) -> Result<Id>;
pub fn query(conn: &Connection, f: &AuditFilter, page: Page) -> Result<Vec<AuditRow>>;
pub fn migrations() -> ModuleMigrations;
```
Action names are constants in `app::audit_actions` so they're listed in one place.

## Tests
- UPDATE/DELETE abort. Filtering by date/user/action, paging.

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
