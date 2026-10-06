# Module: customers (screens)

You are working on the **customer** screens in the back office: the customer list and
search (CUS-02), records (CUS-01), the customer page with purchases (CUS-03), points
corrections (CUS-04), debt payments and the credit statement (CUS-05). Customer lookup
at the till lives in `modules/checkout/`.

1. Read `docs/05 Modules/Module - customers.md`, especially "Working on this module".
2. Stay inside `ui/src/modules/customers/`. Import only `shared/`, `generated/` and this folder.
3. All screen text is in `text/en.ts`. Every button and field has a tooltip.
   Balances come from Rust: never add up points or money here.
4. Test on its own: `pnpm test src/modules/customers` and `pnpm e2e`.
