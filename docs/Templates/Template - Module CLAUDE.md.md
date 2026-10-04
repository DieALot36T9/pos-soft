# Template for `crates/<name>/CLAUDE.md` and `ui/src/modules/<name>/CLAUDE.md`

`cargo xtask new-module` creates these files from this template. Claude Code reads a folder's `CLAUDE.md` automatically when it works in that folder, so whoever opens the module gets its instructions first.

```markdown
# Module: <name>

You are working on the **<name>** module. You can do this without knowing the rest of the app.

1. Read `docs/05 Modules/Module - <name>.md`, especially "Working on this module".
   It lists the requirements, the tables, the public API, who uses it, and the traps.
2. Stay inside `crates/<name>/` and `ui/src/modules/<folder>/`. Don't edit other modules.
   If you need another module's data, take it as a function argument; `app` passes it in.
3. Test on its own: `cargo nextest run -p <name>` (and `pnpm test src/modules/<folder>` for screens).
4. Before finishing: tests first and green, plain-English comments, and the module page
   updated to match the code (docs/08 Testing/Definition of Done.md).
```
