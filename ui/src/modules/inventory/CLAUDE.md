# Module: inventory (screens)

You are working on the **Stock** and **Stock counts** screens and the stock alerts on the dashboard (INV-01 to INV-10).
The Rust side is the `inventory` crate; `app` connects it to catalog, settings and the audit log.

1. Read `docs/05 Modules/Module - inventory.md`, especially "Working on this module".
2. Stay inside `ui/src/modules/inventory/`. Import only `shared/`, `generated/` and this folder.
3. All screen text is in `text/en.ts`. Every button and field has a tooltip.
   Rust checks every rule (reasons, batches, negative stock, counts) and works out every value.
4. Test on its own: `pnpm test src/modules/inventory` and `pnpm e2e`.
