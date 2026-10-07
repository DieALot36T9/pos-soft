# Module: io (screens)

You are working on the **Import / Export** screen (IO-01 to IO-05): the import wizard
for every entity (template, file, sheet, column mapping, match rule, preview with every
error, import with a progress bar and cancel, summary and error file) and the exports.
The Rust side is the `io` crate; `app` maps rows to modules (`workflows/import_*.rs`,
`workflows/exports.rs`).

1. Read `docs/05 Modules/Module - io.md`, especially "Working on this module".
2. Stay inside `ui/src/modules/io/`. Import only `shared/`, `generated/` and this folder.
3. All screen text is in `text/en.ts`. Every button and field has a tooltip.
   Rust checks every cell and saves everything; the screen only shows what Rust says.
4. Test on its own: `pnpm test src/modules/io` and `pnpm e2e`.
