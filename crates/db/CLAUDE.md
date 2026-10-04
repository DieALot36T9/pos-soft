# Module: db

You are working on the **db** module. You can do this without knowing the rest of the app.

1. Read `docs/05 Modules/Module - db.md`, especially "Working on this module".
   It lists the requirements, the tables, the public API, who uses it, and the traps.
2. Stay inside `crates/db/` and `ui/src/modules/backup/`. Don't edit other modules.
   If you need another module's data, take it as a function argument; `app` passes it in.
3. Test on its own: `cargo nextest run -p db` (and `pnpm test src/modules/backup` for screens).
4. Before finishing: tests first and green, plain-English comments, and the module page
   updated to match the code (docs/08 Testing/Definition of Done.md).
