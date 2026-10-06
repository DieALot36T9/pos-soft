# Module: users (screens)

You are working on the **Users** and **Roles** screens (USR-04, USR-06). The Rust side is the `auth` crate.

1. Read `docs/05 Modules/Module - auth.md`, especially "Working on this module".
2. Stay inside `ui/src/modules/users/`. Import only `shared/`, `generated/` and this folder.
3. All screen text is in `text/en.ts`; permission names are in `shared/text/en/permissions.ts`.
   Every button and field has a tooltip. Rust checks every rule (last admin, PIN format, …).
4. Test on its own: `pnpm test src/modules/users` and `pnpm e2e`.
