---
tags: [adr]
status: accepted
---
# ADR-009 ULID ids and till identity

## Context
v1.0 is one till per database, but shops grow. Auto-increment ids would collide if tills are merged or synced later.

## Decision
- Every row id is a **ULID** (`TEXT`, 26 characters, sortable by time).
- Setup creates a **till id** (ULID) and a **till code** (`T1`). Every sale, return, shift, cart and ledger row stores `till_id`.
- Invoice numbers are sequenced **per till** and include the till code.

## Consequences
- ✅ A later multi-till version (a server till, or LAN sync) needs no id or number migration.
- ⚠️ TEXT keys take slightly more space than integers. That's negligible at our scale.
