---
tags: [adr]
status: accepted
---
# ADR-003 SQLite with rusqlite

## Context
We need an embedded, offline, reliable database that handles millions of rows, survives power cuts, and is easy to back up.

## Decision
- **SQLite**, compiled in (`rusqlite` with the `bundled` feature) so every OS uses the same version with FTS5.
- **WAL** journal, **`synchronous = FULL`**, foreign keys on.
- **Plain SQL** in each module (no ORM, no query builder). Statements are prepared and cached.
- One writer connection + one reader connection ([[Database Strategy]]).

## Consequences
- ✅ No server and no install step; the database is one file; reads take microseconds.
- ✅ Plain SQL is easy to read and review, and maps directly to the schema on each module page.
- ⚠️ SQL strings aren't checked at compile time. Every query is covered by module tests against a real in-memory SQLite.
- ⚠️ Never on a network share. Several tills later need a sync or server design.

## Alternatives
- **sqlx:** compile-time checked queries, but async-first and needs a database at build time.
- **Diesel / SeaORM:** more abstraction than we need; harder to read.
- **DuckDB:** made for analytics, not lots of small transactions.
