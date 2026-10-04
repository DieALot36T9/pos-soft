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
- Passwords and PINs are hashed with **argon2id** (memory 19 MiB, 2 iterations, 1 lane; tuned in M3 so a check takes ≤ 300 ms on the reference PC).
- PINs are 4–8 digits. Passwords are at least 8 characters.
- Lockout: 5 failures → 60 s, per user.
- The session lives only in Rust memory. Restarting the app logs everyone out (an open cart is kept in the database).

## Authorization
- `Permission` is a Rust enum in `core`. Roles map to sets of permissions in `auth`.
- Every Tauri command starts with `ctx.require(Permission::X)?` or `ctx.require_or_override(Permission::X, override_token)?`.
- A unit test lists **every command** and asserts the permission it requires, so a new command can't be added without deciding its permission.
- Override tokens: created by `auth::verify_override(admin_username, secret, permission)`, random 128-bit, **single use**, tied to one permission, expire after 60 s, kept in memory only.

## Audit
- `audit_events` is append-only (triggers abort UPDATE/DELETE).
- Events record `user_id` and `approved_by` (for overrides).
- See the list of audited events in [[Req - Users and Security]] USR-08.

## Tauri hardening
- Capabilities: only our own commands, the `dialog` plugin (open/save file pickers) and `fs` limited to the app data folder and paths the user picks. No `shell`, no `http`.
- Strict CSP: `default-src 'self'; img-src 'self' asset: data:; style-src 'self' 'unsafe-inline'` (Tailwind needs inline styles only for Radix positioning).
- No remote content is ever loaded.

## Secrets in logs
- `Secret<String>` wrapper types for passwords, PINs, recovery codes and tokens. Their `Debug` prints `***`.
- Payment references are masked except the last 4 characters.
