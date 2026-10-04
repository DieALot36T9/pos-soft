---
tags: [task]
id: A1
model: claude-opus-5-5
effort: medium
phase: A
depends_on: [A0]
status: done
---
# Task A1: Swap hand-written parts for standard tools

**Model: Opus 5.5 · Effort: medium**

**Start in a fresh session.** A0 is merged. Run this task **before the B tasks**, because it changes the checks they run.

## Read first
- [[Reuse Map]] and [[ADR-023 Reuse before building]]
- [[Module Rules]], [[Frontend Architecture]], [[Coding Standards]], [[Writing Style]]

## Build
1. **cargo-deny instead of `xtask check-deps`**
   - Add `deny.toml`. Under `[bans]`, deny every module crate (`settings`, `auth`, `audit`, `tax`, `catalog`, `pricing`, `barcode`, `inventory`, `sales`, `shifts`, `customers`, `purchasing`, `reports`, `io`, `printing`, `hardware`, `db`) with `wrappers = ["app"]`. Exclude dev-dependencies from the graph, so `reports` may use modules in its tests only.
   - Under `[licenses]`, allow MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, Unicode-3.0, Unicode-DFS-2016, OFL-1.1 (add others only if a dependency truly needs them, and record why in the Reuse Map).
   - CI runs `cargo deny check` (via `EmbarkStudios/cargo-deny-action`). Delete `xtask/src/check_deps.rs` and its command, and move its "a module must not depend on another module" test into a CI step that proves `cargo deny check bans` fails on a deliberately bad `Cargo.toml` (a small script is fine).
2. **eslint-plugin-boundaries instead of A0's custom ESLint rule**
   - Elements: `app`, `shared`, `generated`, and `modules/*` (each module folder its own element). Allowed: modules → `shared`, `generated`, themselves. `app` → everything. Delete the custom `module-boundaries` rule.
3. **i18next instead of `useText`**
   - Add `i18next` + `react-i18next`. Each module's `text/en.ts` becomes an i18next **namespace** (resources stay plain TypeScript objects). Enable typed keys (i18next's TypeScript resource typing), so a wrong key is a build error.
   - Components use `useTranslation('<namespace>')` and `t('holdButton.tip')`. Keep `react/jsx-no-literals`.
   - Remove `useText` and its test; add a test proving a missing key fails the type check (or a runtime test that every key used exists).
4. **num-format for digit grouping in `core`**
   - Replace the hand-written `group_digits` in `crates/core/src/currency.rs` with `num-format` (`Grouping::Standard` / `Grouping::Indian`). **Keep all existing tests and `money_vectors.json` unchanged**: they prove the output is identical.
5. **tauri-plugin-single-instance**: only one copy of the app can run (a second launch focuses the first window). Register it first among plugins.
6. **Clear the "Node.js 20 is deprecated" CI warning:** move GitHub Actions to their current major versions: `actions/checkout@v7`, `actions/setup-node@v7`, `pnpm/action-setup@v6` (check each action's releases page for anything newer at the time).
7. Update [[Module - core]], [[Module Rules]], [[Frontend Architecture]], [[Tech Stack]], [[Dev Environment Setup]], [[Repository Layout]] and the [[Reuse Map]] (mark the A1 rows "in use") to match.

## Done when
- `cargo deny check` passes in CI, and the bad-dependency test proves it catches a module depending on another module.
- `pnpm lint` fails if a module folder imports another module folder (proved by a test fixture or a documented manual check).
- All existing tests still pass, including every `core` money-format test, unchanged.
- No references to `check-deps` or `useText` remain in code or docs (except history: the A0 brief and old CHANGELOG entries stay as written).
- Pull request from `task/A1`.
- Anything not finished is listed under **"Open items"** on [[Module - core]].
