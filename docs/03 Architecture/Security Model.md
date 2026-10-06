---
tags: [architecture, security]
---
# Security Model

## What we protect against
| Threat | Example | Control |
|---|---|---|
| Cashier abuse | Voiding sales and keeping the cash, big discounts for friends, no-sale drawer opens | Permissions, overrides, audit log, discounts/overrides report, blind cash count |
| Misuse of the UI | Calling a command the UI hides | Permission check in Rust for every command |
| Lost admin password | Offline shop, no email reset | Recovery code ([[Req - Users and Security]] USR-09) |
| Data loss | Power cut, disk failure | WAL + FULL sync, backups with checks, second backup folder |
| Someone copies the database file | Stolen PC | Out of scope for app-level crypto. Recommend BitLocker in [[Installation]]. Passwords are hashed, no card numbers stored |
| Malicious import file | Formula injection in CSV | On export, cells starting with `= + - @` get a leading `'`. On import, cells are treated as text, never evaluated |

## Authentication
- Passwords and PINs are hashed with **argon2id** (memory 19 MiB, 2 iterations, 1 lane: the **OWASP Password Storage Cheat Sheet** recommendation; checked in M3 so a check takes ≤ 300 ms on the reference PC).
- PINs are 4–8 digits. Passwords are at least 8 characters.
- Lockout: 5 failures → 60 s, per user.
- The session lives only in Rust memory. Restarting the app logs everyone out (an open cart is kept in the database).

**Login (C-M3):** the login screen shows a tile for each active user; the user types their PIN or password (`login` → `auth::verify_login`). Logins, logouts and lockouts are audited. The admin made in the setup wizard is signed in straight away. F12, the Lock button or the idle timer lock the till: the session stays, but every guarded command answers `till_locked` until the same user (or an admin) unlocks it. Before each permission check the user is read again from `auth`, so a disabled user or a changed role counts at once.

## Authorization
- `Permission` is a Rust enum in `core`. Roles map to sets of permissions in `auth`.
- Every Tauri command starts with `session::require(state, Permission::X)?` or `session::require_or_approval(state, Permission::X, approval)?` (the second gives `OverrideRequired` so the screen can ask an admin).
- `commands::COMMANDS` lists **every command** with the permission it requires, so a new command can't be added without deciding its permission. `crates/app/tests/permissions.rs` calls every guarded command for real: signed out, as a cashier (refused exactly where the Cashier role lacks the permission), as an admin, and on a locked till.
- Setup commands need no sign-in (nobody exists yet), but work only until setup is complete. Settings and tax rate commands also work without sign-in during setup, because the wizard uses them; afterwards they need `SettingsManage` / `TaxManage`. The recovery commands work only while the database is damaged.
- `product_save` checks `CatalogManage`, and also `CatalogPriceEdit` when the sale or cost price changes (or a new product has a price), so a stock keeper can edit products without changing prices (C-M4).
- Override tokens: created by `request_override` after `auth::verify_override(admin_username, password, permission)`, random 128-bit, **single use**, tied to one permission and the user who asked, expire after 60 s, kept in memory only and dropped at logout or lock.

## Audit
- `audit_events` is append-only (triggers abort UPDATE/DELETE).
- Events record `user_id` and `approved_by` (for overrides).
- See the list of audited events in [[Req - Users and Security]] USR-08.

## Tauri hardening
- Capabilities: only our own commands, the `dialog` plugin (`dialog:allow-open`: open file and folder pickers) and `fs` limited to the app data folder and paths the user picks. No `shell`, no `http`.
- Strict CSP: `default-src 'self'; img-src 'self' asset: data:; style-src 'self' 'unsafe-inline'` (Tailwind needs inline styles only for Radix positioning).
- No remote content is ever loaded.

## Secrets in logs
- `Secret<String>` wrapper types for passwords, PINs, recovery codes and tokens. Their `Debug` prints `***`.
- Payment references are masked except the last 4 characters.
