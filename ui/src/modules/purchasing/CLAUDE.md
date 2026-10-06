# Module: purchasing (screens)

You are working on the **Purchasing** screens in the back office: suppliers (PUR-01),
purchase orders with suggestions from low stock (PUR-02), receiving goods (PUR-03),
supplier returns (PUR-04), payments and what the shop owes (PUR-05), and statements (PUR-06).
The Rust side is the `purchasing` crate; `app` connects it to catalog, inventory and shifts.

1. Read `docs/05 Modules/Module - purchasing.md`, especially "Working on this module".
2. Stay inside `ui/src/modules/purchasing/`. Import only `shared/`, `generated/` and this folder.
3. All screen text is in `text/en.ts`. Every button and field has a tooltip.
   Every value, total, cost change and balance comes from Rust: never add up money here.
4. Test on its own: `pnpm test src/modules/purchasing` and `pnpm e2e`.
