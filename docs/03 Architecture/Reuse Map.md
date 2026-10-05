---
tags: [architecture, reuse]
---
# Reuse Map

**Rule: don't reinvent.** Before writing something, check this page. Use an established standard, library or open-source pattern if one fits. We only write our own code where nothing fits, and the reason is recorded here. Decision: [[ADR-023 Reuse before building]].

## Standards we follow
| Area | Standard | Where it's used |
|---|---|---|
| Barcodes | **GS1 General Specifications**: GTIN check digit (mod 10); restricted-circulation prefixes **20–29** for in-store and variable-measure (weighed) items | [[Module - barcode]]. Our internal prefix `200` and the scale prefixes `21`/`23` are inside the GS1 in-store range |
| Scale stickers | **Odoo barcode nomenclature** pattern syntax (`21.....{NNDDD}`) | [[Module - barcode]] |
| Receipt printers | **Epson ESC/POS** command set | [[Module - printing]] via the `escpos` crate |
| Label printers | **TSC TSPL** | [[Module - printing]] |
| Scales | Vendor protocols, e.g. **Mettler-Toledo MT-SICS**, CAS, Toledo 8217 | [[Module - hardware]] scale profiles |
| Currencies | **ISO 4217** (codes and number of decimals) | `iso_currency` crate in [[Module - core]] and the setup wizard |
| Phone numbers | **E.164**, via Google's libphonenumber rules | `phonenumber` crate in [[Module - customers]] |
| Units | **UN/ECE Recommendation 20** unit codes (e.g. `H87` piece, `KGM` kilogram, `GRM` gram, `LTR` litre, `MLT` millilitre), stored as an optional code on each unit | [[Module - catalog]], ready for future e-invoicing |
| Dates and times | **ISO 8601**; timestamps in UTC | [[Data Conventions]] |
| IDs | **ULID** specification | `ulid` crate |
| CSV | **RFC 4180** | `csv` crate |
| Number formats | **Unicode CLDR** (the data behind `Intl.NumberFormat`) | Screens: `Intl.NumberFormat`. Receipts: `num-format` |
| Passwords | **OWASP Password Storage Cheat Sheet**: argon2id, 19 MiB memory, 2 iterations, 1 lane | [[Module - auth]] |
| Accessibility | **WCAG 2.2**, AAA contrast for text | [[Design System]] |
| Permissions | Role-based access control (**RBAC**) | [[Module - auth]], [[Roles and Permissions]] |
| Versions and changelog | **Semantic Versioning**, **Keep a Changelog**, **Conventional Commits** for commit messages | [[Release Process]] |
| Decision records | **Michael Nygard's ADR format** | `04 Decisions/` |
| Acceptance criteria | **Given / When / Then** (from BDD / Gherkin) | `02 Requirements/` |

## Established patterns we use
| Pattern | Source | Where |
|---|---|---|
| Money as integer minor units + **allocate** with largest remainder | Martin Fowler's *Money* pattern (*Patterns of Enterprise Application Architecture*); same approach as Stripe's API | [[Money Tax and Rounding]] |
| Append-only ledgers with cached balances | Accounting ledgers; Odoo's stock moves | [[ADR-010 Append-only ledgers]] |
| Price snapshots on order lines | Every major POS/ERP (Odoo, ERPNext) | [[ADR-011 Snapshots on sale lines]] |
| Tax included/excluded in price | Odoo `price_include`, ERPNext "included in print rate" | [[ADR-008 Tax model]] |
| Pricing rules and promotions | Odoo pricelists and loyalty programs, ERPNext pricing rules | [[Module - pricing]] |
| X and Z reports, cash float, blind close | Standard retail/fiscal POS practice | [[Req - Shifts and Cash]] |
| FEFO for expiring stock | Standard warehouse practice (Odoo removal strategies) | [[Module - inventory]] |
| `cargo xtask` for project scripts | The common Rust convention (matklad's cargo-xtask) | `xtask/` |
| Workspace of small crates | Common Rust practice for large apps | [[ADR-004 One crate per module]] |

## Libraries instead of our own code
| Need | Library | Status |
|---|---|---|
| Module crates must not depend on each other | **cargo-deny** `bans` with `wrappers = ["app"]` (also checks dependency **licenses**, advisories and sources), set up in `deny.toml` | **In use** (A1). CI also runs `scripts/check-module-bans.sh` to prove a bad dependency is refused |
| UI module folders must not import each other | **eslint-plugin-boundaries**, with **eslint-import-resolver-typescript** so it can follow imports written without `.ts`/`.tsx` | **In use** (A1) |
| Screen text and future languages | **i18next + react-i18next** with typed resources (`text/en.ts` files stay as namespaces, registered in `ui/src/app/i18n.ts`) | **In use** (A1) |
| Digit grouping (Western / Indian) on receipts | **num-format** (`Grouping::Standard` / `Grouping::Indian`) | **In use** (A1), in `core::format_money` |
| Money and number display on screens | Browser built-in **`Intl.NumberFormat`** | C tasks |
| Only one copy of the app may run | **tauri-plugin-single-instance** | **In use** (A1), in `app::run()` |
| File open/save dialogs, files | **tauri-plugin-dialog**, **tauri-plugin-fs** | C tasks |
| Remember window size/position | **tauri-plugin-window-state** | C-M2 |
| ESC/POS encoding + network/USB/serial/file connections, cut, QR | **escpos** crate (MIT) | **In use**: B-printing for the start and cut commands; B-hardware for the network, serial and file connections. Two commands are written by hand (see below) |
| Text on receipts and labels: shaping, line breaking, drawing letters | **cosmic-text** (with `swash`) on a **tiny-skia** page | **In use** (B-printing). Only our bundled Noto Sans fonts, never the system's |
| QR codes on receipts | **qrcode** | **In use** (B-printing) |
| Shop logo to black and white (Floyd–Steinberg dithering), PNG previews | **image** | **In use** (B-printing) |
| Byte snapshots in tests | **insta** | **In use** (B-printing), ESC/POS and TSPL bytes |
| Serial ports (scales, serial printers, port list) | **serialport** (MPL-2.0, allowed for this crate only: see below) | **In use** (B-hardware) |
| Patterns that find the weight in a scale's line | **regex** | **In use** (B-hardware) |
| Barcode images / test decoding | **barcoders**, **rxing** | B-barcode |
| Currency list (codes, decimals, symbols) | **iso_currency** (MIT, `iterator` feature) | **In use** (B-settings): `settings::currency_choices`, `currency_format_for` and the currency code check. Screens in C-M2 |
| Phone number cleaning | **phonenumber** (libphonenumber port, Apache-2.0) | **In use** (B-customers), in `customers::normalize_phone` |
| Log files with daily rotation and "keep 14" | **tracing-appender** (`Rotation::DAILY`, `max_log_files`) | C-M2 |
| Keyboard shortcuts | **react-hotkeys-hook** (scopes, works in inputs), listed in one file (`ui/src/shared/keyboard/shortcuts.ts`) | **In use** (C-M1: the scan simulator's `Ctrl+Shift+S`) |
| Screen flows in a real browser | **Playwright** (`@playwright/test`, Chromium) against the fake bridge | **In use** (C-M1), `pnpm e2e` |
| PNG pictures sent to the screen as `data:` addresses | **base64** (already inside Tauri) | **In use** (C-M1), `app::preview` |
| Toast messages | **sonner** (shadcn/ui default) | C tasks |
| Command palette (`Ctrl+K`) | **cmdk** (shadcn/ui `Command`) | C tasks |
| Tooltips, dialogs, menus | **Radix UI** via shadcn/ui | All screens |
| Password, PIN and recovery-code hashing (argon2id) | **argon2** (RustCrypto) | **In use** (B-auth) |
| Random recovery codes | **rand** (`rand::rng()`, a secure generator) | **In use** (B-auth) |
| Reading CSV files (RFC 4180) | **csv** | **In use** (B-io) |
| Reading Excel (.xlsx) files | **calamine** (MIT) | **In use** (B-io) |
| Writing Excel (.xlsx) files with typed cells and number formats | **rust_xlsxwriter** (MIT/Apache-2.0) | **In use** (B-io) |
| Temporary files in tests | **tempfile** | **In use** (B-io tests; reports benchmark) |
| Fake data for big test databases | **fake** crate | `xtask seed` |
| Benchmarks | **criterion** | **In use** (B-catalog: `cargo bench -p catalog`; B-auth: `cargo bench -p auth`; reports: `cargo bench -p reports`). `xtask bench` later |
| Test coverage | **cargo-llvm-cov** | CI |
| CI caching / tool install | `Swatinem/rust-cache`, `taiki-e/install-action`, `tauri-apps/tauri-action` | CI |
| Database backup | SQLite's built-in `VACUUM INTO` | [[Module - db]] |
| Product search | SQLite's built-in **FTS5** | **In use** (B-catalog), [[Module - catalog]] |
| Reading and resizing product pictures (PNG, JPEG, WebP) | **image** crate (MIT/Apache-2.0), only the `png`, `jpeg` and `webp` features | **In use** (B-catalog), [[Module - catalog]] |

## Things we write ourselves (and why)
| Thing | Why no library fits |
|---|---|
| Per-module migration runner (`db`) | `rusqlite_migration` and `refinery` keep **one** version counter for the whole database. Our modules must add migrations independently, so versions are tracked per module. Small (~180 lines) and tested |
| Pricing engine | Pricing rules are business-specific. We follow Odoo/ERPNext rule patterns, but there's no reusable Rust library |
| Receipt layout (document → picture) | There's no receipt-layout library; text drawing itself uses **cosmic-text** and **tiny-skia** |
| Suggested cash denominations (`settings::suggested_denominations`) | ISO 4217 lists currencies, not their notes and coins. A short table for BDT, INR, USD, EUR and GBP lives in `crates/settings/src/currency.rs` |
| ESC/POS picture strips (`GS v 0`) and the drawer pulse (`ESC p m t1 t2`) | The `escpos` crate prints pictures only from image files: it decodes them, shrinks them to 512 dots and converts them again, while our picture is already in the printer's format. Its `cash_drawer()` (0.20) leaves out `t1 t2`, so the printer would swallow the next two bytes. Both are a few bytes, tested byte for byte in `printing` |
| TSPL label commands | No maintained Rust crate writes TSPL. It is five text commands (`SIZE`, `GAP`, `CLS`, `BITMAP`, `PRINT`) |
| Scale protocol profiles | No Rust crate covers scale protocols; profiles follow each vendor's published protocol |
| CSV separator guess (comma, semicolon or tab) in `io` | The `csv` crate doesn't guess separators, and `csv-sniffer` has had no release for years. Ours counts the three characters in the header line (about 20 lines, tested) |
| CSV formula guard in `io` | A few lines following the **OWASP "CSV Injection"** advice (leading apostrophe), plus the reverse on reading so exports round-trip. No maintained crate does both |
| Windows print spooler "raw" connection | The `escpos` crate has no spooler driver (its Windows driver, `usbprint`, talks to USB printers directly and skips the spooler). So `hardware` calls winspool through the `windows` crate, in `printer/windows_spooler.rs` (about 150 lines) |
| Preview printer's ESC/POS reader | No crate reads ESC/POS back into pictures. It only needs `GS v 0` pictures and the `ESC p` drawer pulse (about 60 lines, in `hardware`) |

## Open-source projects to learn from
| Project | Licence | Use it for |
|---|---|---|
| Odoo Point of Sale | LGPL-3.0 | Ideas: barcode nomenclature, pricelists, loyalty, sessions (shifts), receipts |
| ERPNext / POS Awesome | GPL-3.0 | Ideas: pricing rules, tax templates, POS closing |
| Open Source Point of Sale (OSPOS) | MIT | Ideas: barcode labels, stock, reports. Code can be reused with attribution |

**Licences we accept for Tauri's own dependencies:** a few crates that Tauri needs are under **MPL-2.0** (`cssparser`, `cssparser-macros`, `selectors`, `dtoa-short` through `tauri-utils`; `option-ext` through `dirs`). MPL-2.0 only asks that changes to those files stay open; we use them unchanged. `deny.toml` allows MPL-2.0 for exactly these crates, not for any other dependency. **serialport** (MPL-2.0) is allowed too, for the same reason: `hardware` uses it unchanged for scales and serial printers, and no permissively licensed serial port crate is as widely used and maintained. Three advisories are ignored in `deny.toml`, all "unmaintained" notices rather than security holes. `proc-macro-error` is unmaintained (RUSTSEC-2024-0370), but it only runs at build time, inside Tauri's Linux window toolkit (gtk-rs). `ttf-parser` is unmaintained (RUSTSEC-2026-0192), but it comes only through `cosmic-text` (whose newest release still uses it) and reads only the fonts bundled in `crates/printing/fonts`. `atomic-polyfill` is unmaintained (RUSTSEC-2023-0089); it comes from `phonenumber` → `postcard` → `heapless` 0.7, which reads the built-in phone rules. On our targets it only uses the standard atomics, and no safe upgrade exists yet.

**Fonts:** Noto Sans and a two-letter cut of Noto Sans Bengali (for ৳) are bundled in `crates/printing/fonts` under the **SIL Open Font License 1.1** (`OFL.txt` there). It lets us bundle and change the fonts; we may not sell the fonts on their own.

**Licence rule:** learn ideas from any project. **Copy code only from permissively licensed sources** (MIT, Apache-2.0, BSD, ISC, Zlib), with a comment saying where it came from. Never copy code from GPL, LGPL or AGPL projects. `cargo-deny` checks that our dependencies' licences are on the allowed list. *The product owner still has to choose this project's own licence. Until then, it's treated as closed source.*

## How to use this page
- Before writing a new helper, search here and on crates.io / npm for an established option.
- If you add a library: it must be maintained (a release in the last year), widely used, and permissively licensed. Add it to this page in the same change.
- If you write something yourself, add a row to "Things we write ourselves" with the reason.
