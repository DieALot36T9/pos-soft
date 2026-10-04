# Module: io

You are working on the **io** module. You can do this without knowing the rest of the app.

1. Read `docs/05 Modules/Module - io.md`, especially "Working on this module".
   It lists the requirements, the tables, the public API, who uses it, and the traps.
2. Stay inside `crates/io/` and `ui/src/modules/io/`. Don't edit other modules.
   If you need another module's data, take it as a function argument; `app` passes it in.
3. Test on its own: `cargo nextest run -p io` (and `pnpm test src/modules/io` for screens).
4. Before finishing: tests first and green, plain-English comments, and the module page
   updated to match the code (docs/08 Testing/Definition of Done.md).
