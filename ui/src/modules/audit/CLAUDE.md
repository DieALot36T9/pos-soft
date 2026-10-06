# Module: audit (screens)

You are working on the **audit log** screen (USR-08). It only reads: nobody can change or delete an event.

1. Read `docs/05 Modules/Module - audit.md`, especially "Working on this module".
2. Stay inside `ui/src/modules/audit/`. Import only `shared/`, `generated/` and this folder.
3. All screen text is in `text/en.ts`, including a plain name for each action.
   Every button and field has a tooltip.
4. Test on its own: `pnpm test src/modules/audit` and `pnpm e2e`.
