---
tags: [module]
crate: {{name}}
milestone:
---
# Module: {{name}}

**Purpose:** one or two sentences.

**Requirements:** [[Req - …]]. **Depends on:** `core`, `rusqlite`, (approved crates).

## Tables
```sql
-- every table and view prefixed with {{name}}_
```

## Published views
| View | Columns | Used by |
|---|---|---|

## Public API
```rust
pub fn …
pub fn migrations() -> ModuleMigrations;
```

## Rules and notes

## Tests
- 

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]].*

| | |
|---|---|
| **Rust folder** | `crates/{{name}}/` |
| **Screen folder** | `ui/src/modules/{{name}}/` |
| **Requirements** | |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | |
| **Used by** (re-run their tests when the public API changes) | |
| **Test it on its own** | `cargo nextest run -p {{name}}` |

**Watch out for**
- 

## Change log (significant API/table changes)
| Date | Change |
|---|---|
