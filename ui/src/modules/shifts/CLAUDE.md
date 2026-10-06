# Module: shifts (screens)

You are working on the shift dialogs at the till: open a shift, cash in and out, the
no-sale drawer, close with the cash count, and force close (SHF-01 to SHF-07).
The Rust side is the `shifts` crate; `app` adds the audit log, carts and settings.

1. Read `docs/05 Modules/Module - shifts.md`, especially "Working on this module".
2. Stay inside `ui/src/modules/shifts/`. Import only `shared/`, `generated/` and this folder.
3. All screen text is in `text/en.ts`. Every button and field has a tooltip.
   Count totals and expected cash come from Rust (`shift_count_total`, `shift_close`).
4. Test on its own: `pnpm test src/modules/shifts` and `pnpm e2e`.
