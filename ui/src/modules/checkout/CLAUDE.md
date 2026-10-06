# Module: checkout (screens)

You are working on the **Checkout** screen (till mode): scan box, cart, totals, weight,
quantity, hold/resume/cancel, the payment dialog (CHK-01 to CHK-22, PAY-01 to PAY-08) and
returns, exchanges and voids (RET-01 to RET-07, `ReturnDialog`).
The Rust side is the `sales` crate; `app` connects it to catalog, barcode, pricing, tax,
inventory and shifts.

1. Read `docs/05 Modules/Module - sales.md` and `docs/07 UI Design/Checkout Screen.md`.
2. Stay inside `ui/src/modules/checkout/`. Import only `shared/`, `generated/` and this folder.
   The shift dialogs live in `modules/shifts/`; `app/TillFrame.tsx` joins the two.
3. All screen text is in `text/en.ts`. Every button and field has a tooltip.
   Every amount comes from Rust (`CartView`, `PaymentPreview`): never add up money here.
4. Test on its own: `pnpm test src/modules/checkout` and `pnpm e2e`.
