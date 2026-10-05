# Module: printing

You are working on the **printing** module. You can do this without knowing the rest of the app.

1. Read `docs/05 Modules/Module - printing.md`, especially "Working on this module".
   It lists the requirements, the tables, the public API, who uses it, and the traps.
2. Stay inside `crates/printing/`. This module has no screens. Don't edit other modules.
   If you need another module's data, take it as a function argument; `app` passes it in.
3. Test on its own: `cargo nextest run -p printing`.
   Golden images (`tests/golden/`) change only with the product owner's approval:
   `UPDATE_GOLDEN=1 cargo nextest run -p printing`. Byte snapshots: `INSTA_UPDATE=always`.
4. Before finishing: tests first and green, plain-English comments, and the module page
   updated to match the code (docs/08 Testing/Definition of Done.md).
