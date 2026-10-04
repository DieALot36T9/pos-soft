---
tags: [module]
crate: auth
milestone: M3
---
# Module: auth

**Purpose:** users, roles, permissions, credential checks, lockouts, override verification, recovery code.

**Requirements:** [[Req - Users and Security]], [[Roles and Permissions]]. **Design:** [[Security Model]], [[ADR-015 Permissions enforced in Rust]].
**Depends on:** `core`, `rusqlite`, `argon2`, `rand`.

## Tables
```sql
CREATE TABLE auth_roles (
  id                TEXT PRIMARY KEY,
  name              TEXT NOT NULL UNIQUE,
  is_system         INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0,1)),  -- Admin, Cashier
  max_discount_bps  INTEGER NOT NULL DEFAULT 0,                             -- Admin: 10000
  created_at        INTEGER NOT NULL
);
CREATE TABLE auth_role_permissions (
  role_id     TEXT NOT NULL REFERENCES auth_roles(id),
  permission  TEXT NOT NULL,             -- Permission enum, snake_case
  PRIMARY KEY (role_id, permission)
);
CREATE TABLE auth_users (
  id               TEXT PRIMARY KEY,
  username         TEXT NOT NULL UNIQUE COLLATE NOCASE,
  display_name     TEXT NOT NULL,
  role_id          TEXT NOT NULL REFERENCES auth_roles(id),
  password_hash    TEXT,                 -- argon2id PHC string
  pin_hash         TEXT,
  active           INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  failed_attempts  INTEGER NOT NULL DEFAULT 0,
  locked_until     INTEGER,
  created_at       INTEGER NOT NULL,
  updated_at       INTEGER NOT NULL,
  CHECK (password_hash IS NOT NULL OR pin_hash IS NOT NULL)
);
CREATE TABLE auth_recovery (
  id          INTEGER PRIMARY KEY CHECK (id = 1),
  code_hash   TEXT NOT NULL,
  created_at  INTEGER NOT NULL
);
```
The Admin role's permissions aren't stored. `permissions_of` returns **all** permissions for it.

## Public API
```rust
pub fn create_role(tx, NewRole) -> Result<Id>;
pub fn set_role_permissions(tx, role: &Id, perms: &[Permission], max_discount: Bps) -> Result<()>;
pub fn create_user(tx, NewUser) -> Result<Id>;           // hashes secrets
pub fn update_user(tx, &Id, UserUpdate) -> Result<()>;   // refuses to remove the last active admin
pub fn set_secret(tx, &Id, SecretKind, Secret<String>) -> Result<()>;
pub fn login_tiles(conn) -> Result<Vec<UserTile>>;
pub fn verify_login(tx, user: &Id, secret: Secret<String>, now) -> Result<SessionUser>; // lockout logic
pub fn permissions_of(conn, user: &Id) -> Result<PermissionSet>;
pub fn verify_override(tx, username: &str, secret: Secret<String>, needed: Permission, now) -> Result<Id>;
pub fn create_recovery_code(tx, now) -> Result<Secret<String>>;   // returns plain code once
pub fn reset_with_recovery(tx, code: Secret<String>, admin: &Id, new_password: Secret<String>, now) -> Result<Secret<String>>; // returns the new code
pub fn migrations() -> ModuleMigrations;
```
Override **tokens** (single-use, 60 s) live in `app` memory, not here.

## Tests
- Lockout after 5 failures, unlock after 60 s (fixed clock), counter reset on success.
- The last active admin can't be disabled or demoted.
- Admin has every permission, including a newly added enum variant.
- `verify_override` fails for a user without the needed permission.
- Hash parameters produce a check of ≤ 300 ms (benchmark, not a unit test).

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/auth/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/auth/` |
| **Screen folder** | `ui/src/modules/auth/` (login, lock, admin approval dialog) and `ui/src/modules/users/` |
| **Requirements** | [[Req - Users and Security]] USR-01 to USR-07 and USR-09, [[Roles and Permissions]] |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | The current time (`Clock`) |
| **Used by** (re-run their tests when the public API changes) | Login and lock commands, the permission guard (`app/src/guard.rs`), and the admin-approval flow |
| **Test it on its own** | `cargo nextest run -p auth` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Passwords, PINs and codes are always wrapped in `Secret<…>` and are never logged or returned.
- There must always be at least one active admin.
- A new `Permission` defaults to Admin-only. Add it to [[Roles and Permissions]] and to the permission test in `app`.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.
