# Module: catalog (screens)

You are working on the **Products**, **Categories**, **Brands** and **Units** screens (CAT-01 to CAT-11).
The Rust side is the `catalog` crate; `app` saves products with their reorder rule and audit.

1. Read `docs/05 Modules/Module - catalog.md`, especially "Working on this module".
2. Stay inside `ui/src/modules/catalog/`. Import only `shared/`, `generated/` and this folder.
3. All screen text is in `text/en.ts`. Every button and field has a tooltip.
   Rust checks every rule (SKU, barcodes, check digits, kits, variants, prices).
4. Test on its own: `pnpm test src/modules/catalog` and `pnpm e2e`.
