---
tags: [guide, modules]
---
# Adding a Module

1. **Decide it really is a module.** It owns its own tables and rules, and nothing existing owns that data. Otherwise, extend the existing module.
2. Write an ADR if the module changes the architecture.
3. Generate the skeleton:
   ```bash
   cargo xtask new-module loyalty
   ```
   This creates `crates/loyalty/` (`Cargo.toml` depending on `core` only, `src/lib.rs`, `src/error.rs`, `migrations/001_init.sql`, `tests/contract.rs`), `ui/src/modules/loyalty/` (`api.ts`, `text/en.ts`, `text/index.ts`) `docs/05 Modules/Module - loyalty.md` from the template, and a `CLAUDE.md` in both module folders that points to that page.
4. Fill in the module page **first**: purpose, tables, public API, views, tests, and the **"Working on this module"** section (requirements, what it gets from `app`, who uses it, traps).
5. Write contract tests for the API (failing).
6. Implement: migrations → functions → tests green.
7. Wire it into `app`: add `loyalty::migrations()` to `app/src/migrations.rs`, commands, workflows, permissions (new permissions default to Admin-only).
8. Add it to [[Module Map]], [[Database Overview]] and `deny.toml` (ban the new crate with `wrappers = ["app"]`).
9. Complete the [[Definition of Done]].
