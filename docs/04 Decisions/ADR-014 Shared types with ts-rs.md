---
tags: [adr]
status: accepted
---
# ADR-014 Shared types with ts-rs

## Context
Rust structs cross into TypeScript through Tauri commands. Hand-written TS copies drift and cause runtime bugs.

## Decision
- Every struct or enum that crosses the bridge derives `ts_rs::TS` and is exported to `ui/src/generated/`.
- `cargo xtask gen-types` regenerates them. CI fails if the generated files differ from what's committed.
- Command wrappers in each `api.ts` are written by hand (one line each) using the generated types.

## Consequences
- ✅ Changing a Rust type breaks the TS build exactly where it matters.
- ⚠️ Command names and argument names are still strings. A test in `app` lists all registered commands, and `api.ts` names are checked against that list by a small Vitest test.

## Alternatives
- **tauri-specta:** generates the command wrappers too, but its Tauri 2 version was still pre-release when this was decided. Can be revisited later.
