# Module: pricing (screens)

You are working on the **pricing** screens in the back office: price lists and their
prices (PRC-02), tiers (PRC-03) and promotions (PRC-04). Discounts at the till live in
`modules/checkout/`.

1. Read `docs/05 Modules/Module - pricing.md`, especially "Working on this module".
2. Stay inside `ui/src/modules/pricing/`. Import only `shared/`, `generated/` and this folder.
3. All screen text is in `text/en.ts`. Every button and field has a tooltip.
   No price maths here: Rust prices every cart.
4. Test on its own: `pnpm test src/modules/pricing` and `pnpm e2e`.
