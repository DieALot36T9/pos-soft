# Contributing

This page is short on purpose. The rules live in the design vault in `docs/`.

## Start here
1. Read `CLAUDE.md` (the project rules).
2. Pick a task in `docs/12 Tasks/Task Index.md`, or a module page in `docs/05 Modules/`.
   Read its **"Working on this module"** section and the module folder's `CLAUDE.md`.
3. Read `docs/09 Guides/Coding Standards.md` and `docs/09 Guides/Writing Style.md`.

## Set up a computer
- Linux or a cloud session: `bash scripts/cloud-setup.sh` installs everything.
- Windows and macOS: `docs/09 Guides/Dev Environment Setup.md`.

## Everyday commands
| Goal | Command |
|---|---|
| Run the app | `pnpm tauri dev` |
| Screens only, in a browser, with fake data | `pnpm dev:mock` |
| Test one module | `cargo nextest run -p catalog` (core is `-p pos-core`) |
| Test everything | `cargo nextest run --workspace` and `pnpm test` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` and `pnpm lint` |
| Module boundaries | `cargo xtask check-deps` |
| TypeScript types from Rust | `cargo xtask gen-types` |
| New module | `cargo xtask new-module <name>`, then `docs/09 Guides/Adding a Module.md` |

## Before you open a pull request
- Tests first (`docs/09 Guides/TDD Workflow.md`), and every check above passes.
- The module page in `docs/05 Modules/` and `docs/CHANGELOG.md` are updated in the same change.
- The full list: `docs/08 Testing/Definition of Done.md`.
