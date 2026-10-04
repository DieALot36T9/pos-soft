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
   This creates `crates/loyalty/` (`Cargo.toml` depending on `core` only, `src/lib.rs`, `src/error.rs`, `migrations/001_init.sql`, `tests/contract.rs`), `ui/src/modules/loyalty/` (`api.ts`, `text/en.ts`), `docs/05 Modules/Module - loyalty.md` from the template, and a `CLAUDE.md` in both module folders that points to that page. It also adds `loyalty` to the ban list in `deny.toml`, so only `app` may depend on it.
4. Fill in the module page **first**: purpose, tables, public API, views, tests, and the **"Working on this module"** section (requirements, what it gets from `app`, who uses it, traps).
5. Write contract tests for the API (failing).
6. Implement: migrations → functions → tests green.
7. Wire it into `app`: add `loyalty::migrations()` to `app/src/migrations.rs`, commands, workflows, permissions (new permissions default to Admin-only). Register its screen text in `ui/src/app/i18n.ts` (`loyalty: loyaltyEn` in `resources.en`), so `useTranslation("loyalty")` works and its keys are typed.
8. Add it to [[Module Map]] and [[Database Overview]]. Check that `deny.toml` lists it (step 3 does this).
9. Complete the [[Definition of Done]].
