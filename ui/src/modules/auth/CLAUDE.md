# Module: auth (screens)

You are working on the **login, lock and own PIN/password** screens (USR-01 to USR-03, USR-07, USR-09).

1. Read `docs/05 Modules/Module - auth.md`, especially "Working on this module".
2. Stay inside `ui/src/modules/auth/`. Import only `shared/`, `generated/` and this folder.
   The admin approval dialog is in `shared/approval/`, because every module uses it.
3. All screen text is in `text/en.ts`. Every button and field has a tooltip. Rust checks
   every PIN and password; the screens only show its answer.
4. Test on its own: `pnpm test src/modules/auth` and `pnpm e2e`.
