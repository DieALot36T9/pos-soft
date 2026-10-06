# Changelog

All notable changes to this project are recorded here.
Format: [Keep a Changelog](https://keepachangelog.com/). Versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]
### Added (task C-M4: products)
- **Products screen** (CAT-01, CAT-08): list with search by name or SKU, category (with sub-categories) and brand filters, archived products on request, prices in the shop's money format, 50 a page.
- **Product page** with tabs (CAT-01 to CAT-11): General (kind, name, SKU, category, brand, unit, weighing, scale item code, notes), Prices and tax (sale, cost, tax rate with the shop default, price history with who changed it), Barcodes (add by scanning or typing, remove, "Make shop barcode" from the internal prefix), Stock (track stock and expiry, reorder level and quantity, preferred supplier), Variants (attributes → every combination), Kit parts (find products, quantities), Picture. `Ctrl+S` saves, `Esc` goes back. Archive, bring back, and delete for good (only if never used).
- **Save product** workflow: catalog fields, barcodes (check digits checked for EAN/UPC codes), reorder rule and audit in one transaction. Changing a price also needs the "Change prices" permission.
- **Categories** (tree shown as paths), **Brands** and **Units** screens, each audited.
- `catalog::unarchive_product`, a `parent_id` filter on `catalog::list`, and TypeScript types for the catalog shapes. `sales`, `inventory` and `purchasing::is_product_used` (with contract tests), so a used product can only be archived.
- `crates/app/tests/products.rs`, the new commands in `tests/permissions.rs`, screen tests and `ui/e2e/products.spec.ts`.

### Added (task C-M3: login, roles, admin approval and audit)
- **Login screen** (USR-01): a tile for each active user, then their PIN or password. Five wrong tries in a row lock that user for 60 seconds, with a countdown, and the lockout is audited (USR-02). Logins and logouts are audited.
- **Lock screen** (USR-03): F12, the Lock button or the idle timer (`checkout.idle_lock_min`) lock the till; every command is refused until the same user unlocks it, or an admin unlocks to help. "Switch user" logs out.
- **Admin approval** (USR-05): when a role lacks a permission, the amber dialog asks an admin's password; Rust gives a single-use token for that one permission, good for 60 seconds, and audits who asked and who approved. First used by "Restore backup".
- **Users and Roles screens** (USR-04, USR-06): add users with a PIN and/or password, reset them, disable instead of delete (never the last active admin); custom roles with permission checkboxes grouped by area and a discount limit. Every change is audited.
- **My PIN and password** (USR-07) and **Forgot admin password** with the recovery code, which shows a new code once (USR-09).
- **Audit log screen** (USR-08): filters by business date, user and action, newest first, 50 a page, and export to Excel.
- `auth::list_users`, `auth::list_roles`, `auth::session_user` (with contract tests).
- `crates/app/tests/permissions.rs`: every guarded command called signed out, as a cashier, as an admin and on a locked till.

### Changed (task C-M3)
- The interim username/password sign-in (`session_sign_in`, `session_sign_out`) is replaced by `login_tiles`, `login` and `logout`.
- Every permission check reads the user again, so a disabled user or a changed role counts at once.
- The back-office menu hides pages the user's role can't open (Rust still checks every command).
- The window may open the save-file picker (`dialog:allow-save`), for the Excel export.

### Added (task C-M2: settings, setup wizard and backups)
- **Startup** (BAK-06): pending restore → open → `quick_check` → backup before an upgrade → migrations. A damaged database opens only the **recovery screen**, which restores the newest checked backup.
- **Setup wizard** (SET-01 to SET-12): welcome (new shop or restore from a backup), shop, money with live preview, tax with rates, first admin with the recovery code shown once, till, printer (can be skipped), backups, products ("add later"). Each step is saved on Next, so setup resumes where it stopped.
- **Settings** screens (SET-04 to SET-14, SET-13 audit): shop, money and rounding, tax, till, receipt, backups, logs. Decimals lock after the first sale (SET-05). Every change is audited with old and new values.
- **Tax rates** screen (TAX-02, TAX-07, TAX-10): add, change, deactivate (never delete), each audited.
- **Backups** (BAK-01 to BAK-05): timer (skipped when nothing changed), on close, before an upgrade and a restore; second folder copy with a warning banner; pruning; Backups screen with "Back up now" and restore with a confirmation naming the date (the app restarts).
- **Logging** (BAK-07): log files per day kept 14 days; errors and warnings always; the detailed switch works at once.
- Interim **sign-in** with username and password (C-M3 adds login tiles and PINs), and a back-office frame with a side menu.
- Shared screens parts: `Alert`, `SelectField`, `useSave`, file and folder pickers, `formatPercent` / `parsePercent`; a message for every new error code.

### Changed (task C-M2)
- Every command runs through `on_worker`, which logs it in a span and logs internal errors with their details.
- Playwright tests run against a mock build served by `vite preview` (`pnpm e2e:serve`) instead of the dev server, so pages never reload mid-test.
- `money_preview` shows 1234567 for a currency without decimals.

### Added (task C-M1: receipt pictures and fake hardware)
- **Test print** (HW-02, HW-03): `hardware_test_print` draws a sample receipt with text, the ৳ sign, a logo, a barcode and a QR code at 58 or 80 mm and sends it to the receipt printer. Until printer settings are saved (C-M2, C-M7) that is always the Preview printer.
- **Preview printer** (HW-10): `preview_take_printouts` hands the screen each printout as the exact PNG picture, notices drawer pulses ("Drawer opened"), and saves every picture in the app's data folder (`dev-prints/`).
- **Printing through the computer's print dialog** (HW-07, ADR-013): `print_window` opens the system print dialog; only the newest printout is printed, so any normal printer or "Save as PDF" works.
- **Dev-only tools** (HW-10), in debug builds and in Mac pull-request builds (`dev-tools` feature): the fake scale panel (`dev_fake_scale_get`, `dev_fake_scale_set`, `dev_fake_scale_read`) and the scan simulator (`Ctrl+Shift+S`) with a scan test box.
- The first golden receipt pictures in `crates/app/tests/golden/` (test receipt at 58 and 80 mm), waiting for the product owner's approval.
- Screens: `ui/src/modules/hardware/` (test print, Preview printer, fake scale, scan simulator); shared `Field`, `CheckBox` and `Badge`; a message for each device error code.
- Playwright screen flows (`pnpm e2e`, `ui/e2e/`), run in CI on Linux.

### Changed (task C-M1)
- `AppInfo` tells the screens whether the dev tools are built in (`dev_tools`).
- The fake bridge is loaded only in mock and test modes, so the real app doesn't carry its demo data.

### Added (reports follow-up)
- Z and X reports show the cash count per denomination (SHF-08), from the new `shifts_v_counts` view (shifts migration 002, with its contract test).
- `supplier_statement` and `customer_statement` reports (RPT-07): opening balance, each ledger row with the balance after it, and the closing balance.
- The RPT-11 speed check: one year of reference data (50,000 products, 1,500 sales a day) and a budget test (`cargo nextest run -p reports --release --run-ignored only`) plus `cargo bench -p reports`. Every report now meets its budget: daily summary 16 ms, every year-long report under 3 s (product-level sales and profit about 2.9 s, close to the limit).
- `app::audit_actions` with the three action names the reports count, tested against `reports::ACTION_*`.

### Changed (reports follow-up)
- Year-long reports add up rows in Rust instead of SQL `GROUP BY`, and their look-ups use indexes. Before: daily summary 3.2 s, tax report 10 s, stock movement 8.5 s, sales by 6 s.
- `sales` migration 002 replaces the `sales_lines(sale_id)` index with the covering index `sales_lines_report`, so reports read a year of lines from the index.
- `reports::ReportsError::ShiftNotFound` is now `NotFound { entity, id }` (also used for suppliers and customers).

### Added (task B-reports)
- `reports` crate: every report as one `ReportTable` — daily summary (RPT-01), X/Z shift reports (RPT-02, SHF-05, SHF-08), sales by product, category, brand, cashier, hour, weekday or date with filters and top N (RPT-03), profit and margin from the cost saved on each line (RPT-04), stock value, low stock with supplier and suggested order, near expiry, stock movement (RPT-05), tax by rate and component (RPT-06, TAX-09), supplier and customer aging (RPT-07), and discounts and overrides per cashier (RPT-08). Reads only other modules' `*_v_*` views; owns no tables. Rust only; `app` wiring and screens come in Phase C.
- A fixture shop with one scripted day; every report's numbers are checked against hand-worked values. Contract tests check every report's cells against its column kinds, also on an empty database.

### Added (task B-inventory)
- `inventory` crate: the stock ledger with append-only triggers, cached levels and batch stock updated in the same transaction, batches with first-expiry-first-out (FEFO) splitting, negative stock allowed or refused (INV-04), adjustments with configurable reasons (INV-02), reorder rules and the low-stock list (INV-03), near-expiry list and expired-batch check (INV-05), stock counts with expected quantities captured at the first scan (INV-06), history with a running balance (INV-07), `reverse` for voids, and `verify` (INV-10). Rust only; `app` wiring and screens come in Phase C.
- Views `inventory_v_levels` (also lists products that have only a reorder rule), `inventory_v_ledger` and `inventory_v_batches`, with contract tests.
- Randomized test: any sequence of stock changes keeps every level and batch equal to its ledger sum.

### Added (task B-auth)
- `auth` crate: roles (Admin, Cashier, custom), permissions, users, argon2id hashing of PINs and passwords (OWASP settings), login with lockout (5 wrong tries → 60 s), admin approval check (password only), and the recovery code. USR-01, USR-02, USR-04 to USR-07, USR-09.
- Benchmark `cargo bench -p auth`: one PIN check takes about 19 ms on the cloud build machine (target ≤ 300 ms).

### Added (task B-settings)
- `settings` crate: 17 typed settings (shop, currency, rounding, tax, till, receipt, hardware, scale rules, internal barcode, backup, logging, checkout, inventory, loyalty, credit, denominations, language), each with defaults and `validate()`. `get`, `set` (returns the old value for the audit log), setup-step progress (`setup_status`, `complete_step`) and `snapshot`. Tables `settings_values` and `settings_setup_steps` (SET-01 to SET-17).
- The ISO 4217 currency list for the setup wizard comes from the **iso_currency** crate (`currency_choices`, `currency_format_for`). Suggested notes and coins for BDT, INR, USD, EUR and GBP.
- TypeScript types for every setting in `ui/src/generated/`.

### Changed (shared `TaxMode`)
- `TaxMode` now lives in `core` (`pos_core::TaxMode`, with `as_str` and `parse`). `settings` uses it for `TaxSettings::mode`. `tax` and `sales` each had their own copy, and `settings` was adding a third; two of them wrote the same `ui/src/generated/TaxMode.ts`. Now there is one type for the one shop-wide choice (ADR-008).

### Added (task B-customers)
- `customers` crate: customer records with phone numbers cleaned to E.164 by the **phonenumber** crate (CUS-01), search by any part of the phone or the start of name words (CUS-02), the points ledger with earn/redeem/reverse/adjust rules (CUS-04), the credit ledger with debt, payments, store credit used before new debt, statements and oldest unpaid debt for aging (CUS-05, CUS-06), voids that write opposite rows once, and the points maths (E16, PAY-05). Both ledgers are append-only (triggers). Views: `customers_v_customers`, `customers_v_credit_ledger`, `customers_v_points_ledger`.
- `deny.toml` ignores the "unmaintained" notice for `atomic-polyfill`, which `phonenumber` pulls in (see Reuse Map).

### Added (task B-hardware)
- `hardware` crate: printer connections (Windows spooler, network, serial, CUPS, file and **Preview**), with a time limit on every device call. Network, serial and file use the `escpos` crate's drivers; the Windows spooler uses the `windows` crate.
- The Preview printer reads the ESC/POS bytes back into pictures and notices drawer pulses, so `app` can show each receipt without a printer (HW-10).
- Scale reading with profiles (generic continuous, CAS-type, Toledo-type), tested with recorded-format lines; "3 equal readings" when a scale has no stable flag (HW-05).
- Listing installed printers and serial ports. Fakes that act like real devices: `MockPrinter`, `MockScale`, `FakeScale`.
- TypeScript types for the hardware settings (`PrinterConfig`, `ScaleProfile`, `SerialSettings`, `Reading` and others).
- `deny.toml` allows MPL-2.0 for the `serialport` crate only (Reuse Map).

### Added (task B-sales)
- `sales` module: carts and held carts saved on every change, completing a sale from already-priced lines, gapless invoice numbers (`T1-000123`) and return numbers (`T1-R000045`) per till, payment methods with the system methods built in, payments with cash change (`settle_payments`), returns with exact refunds from the stored sale line (the last return refunds exactly what is left), voids, and the `sales_v_sales`, `sales_v_lines`, `sales_v_return_lines` and `sales_v_payments` views.
- `complete()` refuses a sale whose lines, totals or payments don't add up, before it uses an invoice number.
- Database triggers stop completed sales, their lines and payments, and returns from being changed or deleted (only the void fields of a sale can be set, once).
- Sale headers also store tier and invoice discount totals; sale and return lines store their cost.
- `sales_summary` gives the number of sales and the last invoice number of a database or backup, for the backup restore list (BAK-05).

### Added (task B-pricing)
- `pricing` crate: price lists, tiers and promotions (tables, migration `001_init`, view `pricing_v_promotions`) and their admin functions with save checks.
- The pricing engine `price_cart`, a pure function: unit price (override, price list, retail), the single best promotion per product on the combined quantity (ADR-019), tier discount on lines without a promotion, manual line discounts and the invoice discount split. `load_context` reads what it needs. `manual_discount_bps` gives the largest manual discount % for the role limit check.
- Tests for every PRC-06 example, PRC-01 to PRC-07, CHK-08, CHK-12, CHK-13, CHK-16 and CHK-17, contract tests for the API and view, and a property test that no line goes below zero.
### Changed (task B-pricing)
- Module - pricing page: `price_cart` returns a `Result`; a promotion's saving on several lines of one product is split in proportion to each line's base; scale sticker and overridden lines get no automatic discounts but still take manual and invoice discounts.

### Added (task B-purchasing)
- `purchasing` crate: suppliers (PUR-01), purchase orders with PO-numbers, statuses and low-stock suggestions (PUR-02), goods received against an order or directly (PUR-03), supplier returns (PUR-04), the append-only payables ledger with payments, adjustments and aging (PUR-05), and supplier statements (PUR-06). Rust only; `app` wiring and screens come in Phase C.

### Added (task B-tax)
- `tax`: tax rates with optional named components (TAX-02, TAX-03), `calc_line` for inclusive and exclusive prices (TAX-05), `split_components` (TAX-03), create, update, deactivate, get, get_many and list. Rates can't be deleted, only deactivated (TAX-10); the database refuses a DELETE too. View `tax_v_rates`. Tests for the TAX-05 table, examples E4–E7, proptest rules and contract tests. TypeScript types for the rate data in `ui/src/generated/`.

### Added (task B-shifts)
- `shifts` crate: tables `shifts_shifts`, `shifts_movements` (append-only), `shifts_counts` and views `shifts_v_shifts`, `shifts_v_movements`. Functions `open`, `current`, `record`, `reverse_ref`, `expected_cash`, `close`, `summary`. Tests for SHF-01, SHF-02, SHF-03, SHF-06, SHF-07, SHF-08 and RET-04.

### Added (task B-audit)
- `audit`: the append-only `audit_events` table with triggers that refuse UPDATE and DELETE, the `audit_v_events` view, `record` (refuses badly named actions, non-object details and secret-sounding keys) and `query` (newest first, filters by business date, user and action, pages of up to 500). USR-08.

### Added (task B-io)
- `io`: reads .csv (BOM, comma/semicolon/tab, quoted newlines, must be UTF-8) and .xlsx (first or chosen sheet) into a table of text cells; writes .csv (UTF-8 BOM, formula guard) and .xlsx (real numbers with number formats); exports a report to .xlsx with the shop name, report name, date range and time made (RPT-09); matches file headers to fields (`auto_map`); reads money, quantities, yes/no and dates without ever rounding (IO-04); lists bad cells with row and column (`CellErr`). 50k rows read in about 1.3 s (IO-05). Uses `csv`, `calamine` and `rust_xlsxwriter` (added to the Reuse Map).

### Added (task B-db)
- `db`: backups with `VACUUM INTO` (`backup_to`, then a read-only `quick_check`), `verify` before a restore (refuses damaged files, newer or edited migrations), `open_read_only`, `quick_check` for the startup check, change tracking for the backup timer (`change_marker`, `has_changed_since`), `backup_file_name` and `prune_backups` (newest N plus the last of each day). BAK-01, BAK-03, BAK-05, BAK-06.
- `db`: power-cut test: a child process that keeps committing is killed at random moments, and the last committed row must survive (NFR-R1).
### Changed (task B-db)
- `db` has no `db_meta` table: nothing needed it. `verify` no longer returns a sales count; `app` reads it through the sales module using `open_read_only`, so `db` still knows no module's tables.

### Added (task B-printing)
- `printing` crate: a document model; layouts for sale, void and return receipts, X/Z reports, labels and shelf tags; drawing to a 1-bit picture with cosmic-text on a tiny-skia page; PNG previews; shop logo dithering; ESC/POS (picture strips, cut, drawer pulse) and TSPL bytes. Every printed word comes from `ReceiptLabels` (ADR-022). Requirements HW-02 to HW-04, HW-06, HW-10, CAT-12, CAT-13, SHF-08, RET-04, RET-06, TAX-08, SET-14.
- Bundled fonts: Noto Sans Regular and Bold, plus a two-letter cut of Noto Sans Bengali for the taka sign ৳ (Noto Sans has none). SIL Open Font License.
- Golden images for 9 printouts in `crates/printing/tests/golden/`, waiting for the product owner's approval in C-M1.
- The cash drawer pulse is written as the full `ESC p m t1 t2`, because the `escpos` crate's own command leaves out the two timing bytes (the printer would swallow the next two bytes it receives).
### Changed (task B-printing)
- `deny.toml` ignores RUSTSEC-2026-0192 (`ttf-parser` unmaintained), which comes only through `cosmic-text` and reads only the bundled fonts.

### Added (task B-catalog)
- `catalog` crate: products, units, categories, brands, barcodes, variants, kits, images, FTS5 search, archive, price history and sequences (CAT-01 to CAT-11, CHK-01 to CHK-03, CHK-14, CHK-15). Rust only; screens come in C-M4.
- Search keeps one FTS5 row per sellable product, rewritten in the same transaction as every product, barcode, category or brand change. A new table `catalog_search_rows` links each product to its search row.
- Pictures are resized with the `image` crate (PNG, JPEG, WebP) to a 512 px JPEG and a 96 px thumbnail.
- 50k-product benchmark: `search` ≤ 150 ms and `find_by_code` ≤ 2 ms (`tests/benchmark.rs`, plus a criterion benchmark). Measured: about 10 ms and 4 µs.

### Added (task B-barcode)
- `barcode` module: reads scale stickers with Odoo-style rules (CHK-08), checks GTIN check digits (CAT-04), makes internal EAN-13 codes and finds rules that clash with the internal prefix (CAT-11), and draws EAN-13, EAN-8, UPC-A and Code 128 as SVG or as a printer picture (CAT-12). Uses `barcoders`; tests read every picture back with `rxing`.
- The barcode module page now says the pattern leaves out the check digit, and that functions with bad input return `Option`/`Result` instead of panicking.
### Changed (task A1: reuse before building, ADR-023)
- Module crates are kept apart by **cargo-deny** (`deny.toml`, `wrappers = ["app"]`), which also checks licences, advisories and sources. CI runs `cargo deny check` and `scripts/check-module-bans.sh`, which proves a module depending on another module is refused. The hand-written xtask command for this is gone; `cargo xtask new-module` adds new modules to `deny.toml`.
- Screen module folders are kept apart by **eslint-plugin-boundaries**, with a test that proves `pnpm lint` refuses a module importing another module.
- Screen text uses **i18next + react-i18next**. Each `text/en.ts` is a namespace registered in `ui/src/app/i18n.ts`; keys are typed, so a wrong key is a build error. Error messages use `{{name}}` placeholders.
- `core::format_money` groups digits with **num-format**. All money tests and `money_vectors.json` pass unchanged.
- Only one copy of the app can run (**tauri-plugin-single-instance**); a second start brings the open window to the front.
- GitHub Actions moved to `actions/checkout@v7`, `actions/setup-node@v7`, `pnpm/action-setup@v6`, `actions/upload-artifact@v7` (no more Node 20 warning). `scripts/cloud-setup.sh` installs cargo-deny.

### Added
- Cloud sessions set themselves up: a SessionStart hook runs `scripts/cloud-setup.sh`.
- M0 Foundations (task A0): Cargo workspace with 19 crates, each module with its own `CLAUDE.md`.
- `core`: Money, Qty, Bps, Id, Timestamp, BusinessDate, Clock, AppError, Permission, Secret, ReportTable, Bitmap1, rounding, allocate, cash_round, moving_average, business_date, format_money/parse_money, Migration/ModuleMigrations. Examples E1–E16 and shared money format examples.
- `db`: opening SQLite with the agreed settings and the migration runner (`db_migrations`, checksums, refuses edited or newer migrations).
- `app`: Tauri 2 window with the flat app frame and one example command (`app_info`).
- UI workspace: React 19, TypeScript, Vite, Tailwind v4 without shadows or animations, colour tokens with a 7 : 1 contrast test, `ActionButton`/`IconButton`/`Tip` with a required tooltip, `useText()`, fake bridge (`pnpm dev:mock`).
- `cargo xtask check-deps`, `gen-types` and `new-module`. CI for Linux, Windows and the Mac `.dmg`. `scripts/cloud-setup.sh`. `CONTRIBUTING.md`.
- Design vault: charter, scope, requirements, architecture, ADRs, module pages, UI design, test strategy, guides, runbooks.
- Design System: flat look, fixed colour code, tooltips on every button and field (ADR-021).
- Writing Style guide: plain language for docs, code, comments and screen text.
- Glossary section explaining technical words in plain English.
### Changed
- The `core` crate's package is named `pos-core` (`pos_core::Money`), because Rust already has a crate called `core`.
- `Migration` and `ModuleMigrations` live in `core` instead of `db`, so modules can describe their migrations without depending on `db`.
- Money helpers in `core` return `Result<_, AppError>`, so an overflow is an error instead of a wrong number.
- English only for v1.0, with screen and receipt text kept in one place so languages can be added later (ADR-022). Removed the second-language product name and Bangla fonts.
- Testing happens on macOS for now. Windows is still the shop target, checked in CI and on a real Windows PC in M13 (ADR-017).
- Receipts are printed as pictures so any currency symbol and the logo work on every printer (ADR-012).
