---
tags: [architecture, stack]
---
# Tech Stack

The stack is chosen to meet two goals together: **(1) Claude can build and change it quickly without breaking things, and (2) the finished app feels instant.**

## The stack
Development and demo testing happen on **macOS** for now. Shops run **Windows** ([[ADR-017 Windows first all three OS built]]).

| Layer | Choice | Why |
|---|---|---|
| Desktop shell | **Tauri 2** | Native webview instead of a bundled Chromium: about 10 MB installers, about 100 MB RAM, cold start under 1 s. [[ADR-001 Tauri 2 as desktop shell]] |
| Business logic | **Rust (stable, 2024 edition)** | The compiler catches most mistakes before tests run, which matters when an AI writes most of the code. It's also fast with no GC pauses |
| Database | **SQLite (bundled) via rusqlite**, WAL mode | In-process, so queries take microseconds with no network hop. One file, fully offline. [[ADR-003 SQLite with rusqlite]] |
| UI framework | **React 19 + TypeScript (strict)** | The UI stack I write most reliably, with the largest set of examples. React Compiler memoizes automatically. [[ADR-002 React TypeScript Tailwind for UI]] |
| Build tool | **Vite** | Instant hot reload while working on the UI |
| Styling | **Tailwind CSS v4** | Styles sit next to the markup, so one file shows everything. No runtime cost |
| Components | **shadcn/ui** (Radix primitives, copied into the repo), **restyled flat** | Accessible dialogs, menus, comboboxes and **floating tooltips** that we own and can edit. No shadows ([[Design System]]) |
| Server state in UI | **TanStack Query** | Caching and invalidation of command results with almost no code |
| Big tables | **TanStack Table + TanStack Virtual** | Smooth scrolling through 50k rows |
| Forms | **react-hook-form** | Simple, fast forms. Validation happens in Rust |
| Routing | **React Router** (in-memory router) | Familiar; no URLs needed in a desktop app |
| Screen text | **i18next + react-i18next**, one typed namespace per module (`text/en.ts`) | The standard; English now, more languages later without code changes ([[ADR-022 Ready for more languages later]]) |
| Icons | **lucide-react** | Consistent, tree-shaken |
| Rust ↔ TS types | **ts-rs** | Generates TS types from Rust structs, so they can't drift. [[ADR-014 Shared types with ts-rs]] |
| IDs | **ulid** | Sortable, unique across tills. [[ADR-009 ULID ids and till identity]] |
| Passwords | **argon2** (argon2id) | Current best practice |
| Errors | **thiserror** | Simple error enums |
| Logging | **tracing**, **tracing-subscriber** (reload layer), **tracing-appender** | Detailed logging switched on and off at runtime, daily rotating files |
| CSV / Excel | **csv**, **calamine** (read), **rust_xlsxwriter** (write) | Pure Rust, the same on every OS |
| Receipt drawing | **cosmic-text** (text layout) + **tiny-skia** (drawing) + one bundled font (**Noto Sans**) | Receipts print as pictures, so the logo and any currency symbol work on every printer. [[ADR-012 Receipts printed as raster images]] |
| Barcodes (SVG/bitmap) | **barcoders** | EAN-13/8, UPC-A, Code128 |
| QR | **qrcode** | Invoice QR on receipts |
| Images | **image** | Resizing product images and the logo |
| Serial | **serialport** | Scales and serial printers |
| Windows printing | **windows** crate (winspool `WritePrinter`, RAW) | Raw bytes to installed printers |
| Rust tests | `cargo test` (run with **cargo-nextest**), **insta** (snapshots), **proptest** (money/tax properties) | Fast, isolated, in-memory SQLite |
| UI tests | **Vitest** + **Testing Library**, **Playwright** against the UI with a mocked bridge | UI logic and full screen flows without Rust |
| App smoke tests | **tauri-driver** (WebDriver) on Windows and Linux in CI | Proves the packaged app starts and sells |
| Package manager | **pnpm** | Fast, strict |
| CI / releases | GitHub Actions + **tauri-action** | Builds for 3 OSes from one workflow |
| Rust tooling | `cargo-nextest`, `clippy`, `rustfmt`, an `xtask` crate for scripts | Plain Rust scripts instead of shell scripts |

Full list of standards and libraries, and what we still write ourselves: **[[Reuse Map]]**.

## Why not StyleX?
StyleX (Meta's compile-time CSS-in-JS) is a good library, but it's **a worse choice when an AI is writing the code**:
- **Fewer examples to learn from.** Tailwind and shadcn appear in a huge number of projects; StyleX in very few. AI-written StyleX tends to contain more invalid patterns.
- **Strict authoring rules.** Styles must be statically analysable and defined with `stylex.create` at module level, with limits on dynamic values. These are easy to break and only fail at build time.
- **Build plugin setup** (Babel/Vite plugin) adds a moving part. Tailwind v4 is a single Vite plugin.
- **No component ecosystem** like shadcn/ui on top of it.
- Its main benefit, scaling CSS across huge teams and codebases, doesn't matter for a single-developer POS.

**Decision: Tailwind.** Same zero-runtime result, more reliable to write.

## Iteration speed vs. a snappy final app
**Fastest to build and iterate** (what I'm doing to keep the loop short):
1. **Most logic is tested without the UI.** Business rules are plain Rust functions tested against in-memory SQLite, and `cargo nextest -p <module>` runs in seconds. Most work never needs the app window open.
2. **Small crates mean small rebuilds.** Changing `pricing` recompiles `pricing` and `app` only.
3. **The UI can run in a plain browser.** `pnpm dev:mock` runs the React app with a mocked command bridge, hot-reloading in milliseconds with no Rust rebuild. Screens are built and Playwright-tested this way.
4. **Fast dev build settings.** Dependencies are compiled with `opt-level = 1` in dev (so the app isn't sluggish), our own code with `opt-level = 0` (fast compiles), plus a fast linker (`lld` on Windows/Linux).
5. **Generated types.** ts-rs means changing a Rust struct immediately breaks the TS build where it matters. No hunting for mismatches.

**Snappy in production** (what makes it feel instant):
1. **No network, no server.** UI → Tauri IPC → Rust → SQLite in-process. A scan round-trip is about 1–5 ms of work.
2. **Rust release build** with `lto = "thin"`, `codegen-units = 1`, `opt-level = 3`.
3. **Prepared statement cache** (`prepare_cached`) for every query on the checkout path. FTS5 for search. Indexes listed per table.
4. **The checkout screen is preloaded** at startup. Back-office screens are lazy-loaded so they don't slow startup.
5. **Virtualized lists** everywhere a list can be large.
6. **React Compiler** for automatic memoization. The cart re-renders only changed lines.
7. **No animations** at all, except a 0.3 s flash on a new cart line ([[Design System]]).
8. **The writer connection is never blocked by reports.** Reports use a separate read connection (WAL allows reads during writes).

**Alternatives considered:** an Electron + Node stack would be slightly faster for me to iterate (one language), but costs about 150 MB installers, 300+ MB RAM and slower startup. Pure Rust UI frameworks (egui, Slint, Iced) are fast but much slower to build polished forms, tables and Bangla text input with. Tauri + Rust + React gives the best balance.
