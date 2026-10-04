# Module: auth

You are working on the **auth** module. You can do this without knowing the rest of the app.

1. Read `docs/05 Modules/Module - auth.md`, especially "Working on this module".
   It lists the requirements, the tables, the public API, who uses it, and the traps.
2. Stay inside `crates/auth/` and `ui/src/modules/auth/` and `ui/src/modules/users/`. Don't edit other modules.
   If you need another module's data, take it as a function argument; `app` passes it in.
3. Test on its own: `cargo nextest run -p auth` (and `pnpm test src/modules/auth` and `pnpm test src/modules/users` for screens).
4. Before finishing: tests first and green, plain-English comments, and the module page
   updated to match the code (docs/08 Testing/Definition of Done.md).
