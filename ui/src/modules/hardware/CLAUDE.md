# Module: hardware (screens)

You are working on the **hardware** screens: test print, the Preview printer, and the
dev-only scan simulator and fake scale panel.

1. Read `docs/05 Modules/Module - hardware.md`, especially "Working on this module".
2. Stay inside `ui/src/modules/hardware/`. Import only `shared/`, `generated/` and this folder.
3. All screen text is in `text/en.ts`. Every button and field has a tooltip.
4. Test on its own: `pnpm test src/modules/hardware` and `pnpm e2e`.
