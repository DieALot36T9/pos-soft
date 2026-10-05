---
tags: [module]
crate: auth
milestone: M3
---
# Module: auth

**Purpose:** users, roles, permissions, credential checks, lockouts, override verification, recovery code.

**Requirements:** [[Req - Users and Security]], [[Roles and Permissions]]. **Design:** [[Security Model]], [[ADR-015 Permissions enforced in Rust]].
**Depends on:** `core`, `rusqlite`, `argon2`, `rand`, `serde`, `thiserror`, `ts-rs`. Dev: `criterion` (benchmark), `serde_json`.

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

Migration `001_init.sql` also adds the two system roles with fixed ids (`ADMIN_ROLE_ID`, `CASHIER_ROLE_ID`) and the Cashier's default "C" permissions. There is no first user: the setup wizard (C-M2) creates the first admin with `create_user`.

## Public API
Every function returns `Result<_, AuthError>`. `AuthError` turns into `AppError` with the codes listed below. Functions that change data take `&Transaction`; `tx` below means that.
```rust
pub const ADMIN_ROLE_ID: &str   = "00000000000000000000000001";  // system role Admin (seeded by the migration)
pub const CASHIER_ROLE_ID: &str = "00000000000000000000000002";  // system role Cashier, with the "C" permissions

pub fn create_role(tx, NewRole, now: Timestamp) -> Result<Id>;   // NewRole { name, permissions, max_discount: Bps }
pub fn set_role_permissions(tx, role: &Id, perms: &[Permission], max_discount: Bps) -> Result<()>; // refuses Admin
pub fn permissions_of(conn, user: &Id) -> Result<PermissionSet>; // Admin: all; disabled user: none

pub fn create_user(tx, NewUser, now) -> Result<Id>;          // NewUser { username, display_name, role_id, password, pin }; hashes secrets
pub fn update_user(tx, &Id, UserUpdate, now) -> Result<()>;  // UserUpdate { display_name, role_id, active } (all optional);
                                                             // refuses to disable or demote the last active admin
pub fn set_secret(tx, &Id, SecretKind, Secret<String>, now) -> Result<()>; // also ends a lockout
pub fn login_tiles(conn) -> Result<Vec<UserTile>>;           // active users, by display name

pub fn verify_login(tx, user: &Id, secret: Secret<String>, now) -> Result<LoginOutcome>;
pub fn verify_override(tx, username: &str, secret: Secret<String>, needed: Permission, now) -> Result<OverrideOutcome>;

pub fn create_recovery_code(tx, now) -> Result<Secret<String>>;   // returns the plain code once
pub fn reset_with_recovery(tx, code: Secret<String>, admin: &Id, new_password: Secret<String>, now) -> Result<Secret<String>>; // returns the new code
pub fn migrations() -> ModuleMigrations;

pub enum LoginOutcome { LoggedIn(SessionUser), WrongSecret { attempts_left }, LockedNow { until }, StillLocked { until } }
pub enum OverrideOutcome { Approved { approved_by: Id }, NotAllowed, WrongSecret { attempts_left }, LockedNow { until }, StillLocked { until } }
pub struct SessionUser { id, username, display_name, role_id, role_name, is_admin, max_discount: Bps, permissions: PermissionSet }
pub struct UserTile { id, username, display_name, has_pin, has_password }
pub struct PermissionSet;   // contains(p), iter(), len(), is_empty(), all(), from_list(&[..]); JSON: ["checkout_sell", …]
pub enum SecretKind { Password, Pin }
```
**Why a wrong secret is an outcome, not an error:** the failure count must be saved. `app` rolls back a transaction when a call returns an error, which would undo the count and break the lockout. So `verify_login` and `verify_override` return `Ok(WrongSecret / LockedNow / StillLocked)`, and `app` commits the transaction for every outcome. `LockedNow` tells `app` to audit the lockout (USR-02).

**How the rules work**
- **Login (USR-01):** the user's PIN *or* password is accepted. A disabled user gets `user_inactive`.
- **Lockout (USR-02):** the 5th wrong try in a row locks the user until `now + 60 s`, and the count starts again, so after the lock there are 5 new tries. A correct secret, or a new secret from `set_secret`, clears the count and the lock.
- **Admin approval (USR-05):** checks the approver's **password only** (not a PIN), then the needed permission. Wrong passwords count toward the approver's lockout.
- **Recovery (USR-09):** 20 characters from `23456789ABCDEFGHJKMNPQRSTVWXYZ`, shown as `XXXX-XXXX-XXXX-XXXX-XXXX`. Typing ignores dashes, spaces and small letters. The new password is for an **active admin** only.
- **Secrets:** PIN 4–8 digits, password at least 8 characters. Hashes use argon2id, 19 MiB, 2 iterations, 1 lane (OWASP), via the `argon2` crate. Only hashes are stored.

**Error codes** (the UI turns them into sentences in Phase C):

| Code | Kind | When |
|---|---|---|
| `role_name_missing`, `username_missing`, `display_name_missing` | validation | Name left empty |
| `pin_format`, `password_too_short`, `secret_missing` | validation | Bad or no PIN/password |
| `max_discount_too_high` | validation | Discount limit above 100% |
| `override_user_unknown` | validation | No active user with that username |
| `recovery_code_wrong`, `recovery_user_not_admin` | validation | Recovery refused |
| `username_in_use`, `role_name_in_use` | conflict | Name taken (capitals don't matter) |
| `admin_role_fixed` | conflict | Tried to change the Admin role |
| `last_active_admin` | conflict | Would leave no active admin |
| `user_inactive` | conflict | Disabled user tried to log in |
| `not_found` (entity `user` / `role`) | not found | Unknown id |

Override **tokens** (single-use, 60 s) live in `app` memory, not here.

## Tests
| Test | File | Requirement |
|---|---|---|
| Lockout after 5 failures, unlock after 60 s (`FixedClock`), counter reset on success, other users unaffected | `tests/login.rs` `usr_02_*`, `src/rules.rs` | USR-02 |
| PIN and password login, tiles, disabled users | `tests/login.rs` `usr_01_*` | USR-01 |
| The last active admin can't be disabled or demoted | `tests/users.rs` `usr_06_the_last_active_admin_*` | USR-06 |
| Creating users, secret rules, hashes only, password reset, own PIN/password | `tests/users.rs` `usr_06_*`, `usr_07_*` | USR-06, USR-07 |
| Admin has every permission, including a newly added enum variant (checks `Permission::ALL`; nothing stored for Admin) | `tests/roles.rs` `usr_04_admin_has_every_permission_including_new_ones` | USR-04 |
| Cashier defaults, custom roles, discount limit, Admin can't be edited | `tests/roles.rs` `usr_04_*` | USR-04 |
| `verify_override` fails for a user without the needed permission; password not PIN; lockout | `tests/override.rs` `usr_05_*` | USR-05 |
| Recovery code: shown once, hashed, replaced after use | `tests/recovery.rs` `usr_09_*` | USR-09 |
| Tables, seeded roles, JSON shapes, unknown stored permissions ignored | `tests/contract.rs` | contract |
| Hash parameters give a check of ≤ 300 ms | `benches/hash_check.rs` (criterion, `cargo bench -p auth`): about 19 ms on the cloud build machine. Still to check on the reference PC in M3 | Security Model |

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

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- Built in task B-auth. The page was changed to match the code: every function gets `now` (rows have `created_at`/`updated_at`), and login/approval return an outcome instead of an error for a wrong secret (see "Why a wrong secret is an outcome").
- For C-M3: `app` must **commit** the transaction after `verify_login` / `verify_override` for every outcome, audit `LockedNow`, and for USR-07 call `verify_login` with the current secret before `set_secret`.
- For C-M3: add the error codes above to `ui/src/shared/text/en/errors.ts`, and run `cargo xtask gen-types` after any change to the exported types.
- For C-M2: the setup wizard creates the first admin and calls `create_recovery_code`.
- Run `cargo bench -p auth` on the reference PC in M3 and record the time here (must be ≤ 300 ms).
- No published views yet. If `reports` or `audit` screens need user names, add `auth_v_users` (id, username, display_name, role_name, active) in a new migration.
