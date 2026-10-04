---
tags: [adr]
status: accepted
---
# ADR-015 Permissions enforced in Rust

## Context
The UI can be bypassed (dev tools, bugs). Cashiers must not be able to do admin-only actions.

## Decision
- The session (user, role, permissions) lives in Rust state.
- Every command checks `ctx.require(Permission)` or `ctx.require_or_override(Permission, token)` first.
- Overrides: the admin enters credentials in an override dialog → `auth::verify_override` → a single-use token for one permission, valid 60 s → the UI retries the command with the token. Both users are audited.
- A test enumerates every command with its required permission.

## Consequences
- ✅ Security doesn't depend on the UI.
- ✅ "Cashier stays logged in, admin approves once" is supported everywhere through one mechanism.
