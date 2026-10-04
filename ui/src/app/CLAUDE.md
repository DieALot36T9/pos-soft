# Module: app

You are working on the **app** module. You can do this without knowing the rest of the app.

1. Read `docs/05 Modules/Module - app.md`, especially "Working on this module".
   It lists the requirements, the commands, who uses it, and the traps.
2. Stay inside `crates/app/`, `ui/src/app/` and `ui/src/shared/`. Don't edit other modules.
   If you need something new from a module, change that module first (with its contract test).
3. Test on its own: `cargo nextest run -p app` (and `pnpm test src/app` for screens).
4. Before finishing: tests first and green, plain-English comments, and the module page
   updated to match the code (docs/08 Testing/Definition of Done.md).
