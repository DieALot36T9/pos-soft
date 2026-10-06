---
tags: [module]
crate: app
milestone: all
---
# Module: app

**Purpose:** the Tauri application. It's the **only** crate that knows every module. It contains commands, the session, the permission guard, workflows, background jobs and startup/shutdown.

**Design:** [[System Overview]], [[Workflows]], [[Security Model]]. **Depends on:** every crate, `tauri`, `tauri-plugin-dialog`, `tracing*`.

## Structure
```
app/src/
├── main.rs            # builder: plugins, state, commands, setup hook
├── state.rs           # AppState { paths, clock, db, health, session, approvals, backed_up_at_marker, backup_warning, log } (C-M2, C-M3)
├── paths.rs           # AppPaths: database, backups, logs, dev-prints, product images, restore marker in the app data folder (C-M2, C-M4)
├── startup.rs         # start(): pending restore, open, quick_check, pre-upgrade backup, migrate (C-M2)
├── session.rs         # login/logout, lock/unlock, admin approvals, require(Permission), require_or_approval, require_signed_in, require_or_setup (C-M3)
├── logging.rs         # tracing to daily files (keep 14), LogSwitch for the detailed switch, panic hook (C-M2)
├── audit_log.rs       # record(): audit event with till and business date; nothing before the till step (C-M2)
├── migrations.rs      # list of every module's migrations() in a fixed order
├── jobs.rs            # backup timer thread, backup on close (C-M2)
├── hardware_thread.rs # queue for print/drawer/scale requests (C-M7; until then commands use on_worker)
├── devices.rs         # Devices { fonts, preview inbox, fake scale, prints folder }; receipt_printer() (C-M1)
├── preview.rs         # Preview printer jobs → PreviewPrintout (PNG data URLs, drawer, saved files) (C-M1)
├── receipt_words.rs   # every printed word in English: ReceiptLabels (ADR-022) (C-M1)
├── dev_tools.rs       # ON flag, fake scale panel (debug builds or feature `dev-tools`) (C-M1)
├── audit_actions.rs   # every audit action name as a constant, and ALL for the audit screen's filter
├── commands/          # thin: one file per area (checkout.rs, catalog.rs, …)
└── workflows/         # one file per use case (complete_sale.rs, receive_goods.rs, …)
```

## Command pattern
```rust
#[tauri::command]
pub async fn checkout_scan(state: State<'_, AppState>, code: String) -> Result<ScanResult, AppError> {
    run(&state, "checkout_scan", move |ctx| {
        ctx.require(Permission::CheckoutSell)?;
        workflows::scan_code::run(ctx, &code)
    }).await
}
```
`run` = `spawn_blocking` + a tracing span + building `Ctx` + mapping panics to `AppError::Internal`.

## Command list (grouped)
| File | Commands |
|---|---|
| `setup.rs` | setup_status, setup_save_step, setup_finish |
| `auth.rs` | login_tiles, login, logout, lock, unlock, request_override, change_own_secret, recover_admin |
| `checkout.rs` | checkout_state, checkout_scan, checkout_add_product, checkout_set_qty, checkout_remove_line, checkout_set_discount, checkout_set_invoice_discount, checkout_price_override, checkout_set_customer, checkout_set_price_list, checkout_hold, checkout_held_list, checkout_resume, checkout_cancel, checkout_complete, checkout_read_scale, receipt_reprint_last, receipt_reprint |
| `returns.rs` | return_lookup, return_complete, exchange_start, sale_void |
| `shifts.rs` | shift_current, shift_open, shift_cash_move, shift_no_sale, shift_x_report, shift_close, shift_force_close |
| `catalog.rs` (built in C-M4) | Reading needs `StockView`: product_list `{ filter: ProductFilter, page: ProductPage }` → `ProductList`, product_search `{ text, limit }` → `SearchHit[]` (at most 100), product_get `{ id }` → `ProductDetails { product, reorder, on_hand, kit_parts, variants, price_history (with names), image (data address) }`, product_choices → `ProductChoices { units, categories, brands, tax_rates, suppliers, default_tax_rate_id, currency }`, unit_list / category_list / brand_list `{ includeArchived }`. Changes need `CatalogManage`: product_save `{ input: ProductInput }` → id (**also `CatalogPriceEdit` when a sale or cost price changes, or a new product has a price**), product_archive, product_unarchive, product_delete `{ id }`, barcode_generate `{ id }` → the new EAN-13, variants_generate `{ parent, attributes: VariantAttribute[] }` → new ids, kit_save `{ kit, parts: KitPartInput[] }`, product_image_set `{ id, file }`, unit_save `{ input: UnitInput }`, category_save `{ input: CategoryInput }`, brand_save `{ input: BrandInput }`, unit_archive / category_archive / brand_archive `{ id }` |
| `pricing.rs` | price_list_*, tier_*, promotion_* |
| `tax.rs` | tax_rate_list, tax_rate_save, tax_rate_deactivate |
| `inventory.rs` | stock_levels, stock_history, stock_adjust, low_stock, near_expiry, count_create, count_line, count_variances, count_post, count_cancel, stock_verify |
| `purchasing.rs` | supplier_*, po_*, grn_prepare, grn_save, supplier_return_save, supplier_payment, payables_aging, supplier_statement |
| `customers.rs` | customer_lookup, customer_get, customer_save, customer_payment, points_adjust, customer_statement |
| `reports.rs` | report_run, report_export_xlsx, report_print |
| `io.rs` | import_preview, import_commit, import_cancel, export_entity, template_download |
| `labels.rs` | labels_preview, labels_print, shelf_tags_print |
| `users.rs` (built in C-M3) | user_list, user_save `{ input: UserInput }`, role_list → `RolesView { roles, all_permissions }`, role_save `{ input: RoleInput }` |
| `auth.rs` (built in C-M3) | login_tiles, login `{ userId, secret }` → `LoginResult`, logout, lock → `SessionView`, unlock `{ secret }` → `SecretCheck`, unlock_by_admin `{ username, password }`, request_override `{ permission, username, password }` → `ApprovalResult { Approved { token } … }`, change_own_secret `{ kind, current, newSecret }`, recover_admin `{ code, username, password, passwordAgain }` → `RecoveryDone` |
| `audit.rs` (built in C-M3) | audit_query `{ filter, page }` → `AuditList { rows, people, actions }`, audit_export `{ filter, file }` → number written |
| `settings.rs` | settings_get, settings_save, payment_methods_*, hardware_list, hardware_test_print, hardware_open_drawer, hardware_read_scale |
| `backup.rs` | backup_now, backup_list, backup_restore |
| `system.rs` | logs_set_detailed, logs_export, log_client_error, app_info, data_verify (audit_query is in `audit.rs`) |
| `hardware.rs` (built in C-M1) | hardware_test_print `{ paper }`, preview_take_printouts, print_window |
| `dev.rs` (C-M1, dev tools only) | dev_fake_scale_get, dev_fake_scale_set `{ change }`, dev_fake_scale_read |
| `startup.rs` (built in C-M2) | startup_status → `StartupView { damaged, setup, session, backup_warning, idle_lock_min }`, session_current, recovery_backups, recovery_restore `{ file }` (C-M3 removed session_sign_in/session_sign_out: use `login`/`logout`) |
| `setup.rs` (C-M2) | setup_status, setup_save_shop `{ shop }`, setup_save_money `{ step }`, setup_save_tax, setup_save_admin → `AdminCreated { recovery_code, session }`, setup_save_till, setup_save_hardware, setup_save_backup, setup_finish, setup_restore_backup `{ file }` |
| `settings.rs` (C-M2) | settings_get → `SettingsSnapshot`, settings_save `{ change: SettingsChange }` (tagged `{section, value}`), logs_set_detailed `{ detailed }`, currency_choices, currency_format_for `{ code }`, money_preview `{ format }` |
| `tax.rs` (C-M2) | tax_rate_list `{ includeInactive }`, tax_rate_save `{ input: RateInput { id?, rate } }`, tax_rate_deactivate `{ id }` |
| `backup.rs` (C-M2) | backup_now → `BackupDone { file, warning }`, backup_list → `BackupRow[]`, backup_restore `{ file, approval }` (restarts the app; takes an admin's approval token, C-M3) |

**Command shape (C-M2):** each file has plain handler functions (`commands::settings::settings_save(&AppState, …)`, which check the permission and call one workflow; the tests call these) and a `cmd` module with the Tauri commands of the same names, which run the handler through `on_worker(name, …)` (spawn_blocking + a `cmd{name}` log span + duration; a panic or an internal error is logged with its details).

**Built so far (C-M1):** `app_info` (→ `AppInfo { name, version, dev_tools }`), `hardware_test_print` (draws the test receipt and sends it to the receipt printer, which is the Preview printer until printer settings exist), `preview_take_printouts` (→ `Vec<PreviewPrintout { pictures: data:image/png;base64 addresses, drawer_opened, saved_files }>`, each job once), `print_window` (the system print dialog through `WebviewWindow::print`; the screen's print CSS prints only `[data-print-area]`, ADR-013), and the dev commands (→ `FakeScaleView { weight: "1.234", unit, stable, connected }`, `Reading`; a weight that isn't a number is `Validation { field: "weight", code: "weight_invalid" }`). None needs a permission: see `commands::COMMANDS`. Slow work runs through `commands::on_worker` (spawn_blocking; a panic becomes `Internal`). Dev commands are listed in `commands::DEV_COMMANDS` and registered only when `dev_tools::ON` (debug builds, or `--features dev-tools`).

## Tests (`app/tests/`)
- **Built (C-M1):** `tests/test_print.rs`: golden pictures of the test receipt at 80 and 58 mm (`tests/golden/test_receipt_*.png`, dot for dot; `UPDATE_GOLDEN=1 cargo nextest run -p app` after the product owner approves a change), test print on the Preview printer gives the exact picture and no drawer pulse (HW-10), a failing printer gives `AppError::Hardware` (HW-08), printouts come as PNG data and are saved once, a drawer pulse shows as `drawer_opened`. `tests/dev_tools.rs`: the fake scale panel (set, read stable and moving, unplugged → timeout, a bad weight is refused). The permissions table test in `commands/mod.rs` lists every command and dev command.
- **Every workflow**: the happy path plus a failure injected at each step (a fake module error via a test hook) → nothing saved.
- **Permissions table test**: every command → its required permission; a cashier session is refused where it should be.
- **Built (C-M2):** `tests/setup_wizard.rs` (SET-01 fresh install, SET-02 resume, SET-04 name missing, SET-07 tax needs a rate, SET-08 passwords and recovery code, SET-09 till id, SET-10 complete without hardware, SET-11 unwritable folder), `tests/settings_and_tax.rs` (SET-13 permissions with a cashier and signed out, audit with old and new values, SET-05 decimals lock after a sale, TAX-10 deactivate, money preview, BAK-07 logging switch), `tests/backups.rs` (BAK-04 back up now and the list, permissions, BAK-01 timer only after a change, BAK-02 broken second folder warns, BAK-05 restore at the next start with audit and a `pre-restore` backup, a bad file refused, BAK-06 damaged database → recovery restore). Helpers in `tests/common/mod.rs` (fixed clock, `finish_setup`).
- **Built (C-M3):** `tests/login_and_lock.rs` (USR-01 tiles and PIN login, setup must be done, disabled users; USR-02 lockout with audit and other users unaffected; USR-03 lock refuses commands, own unlock, admin unlock with audit, switch user; a user disabled while signed in loses access), `tests/approvals.rs` (USR-05 one restore approved and audited with both users, also after the restart; 60 s expiry; one permission only; ends with the session; only a user with the permission approves; an admin needs none), `tests/users_and_roles.rs` (USR-06 add, reset PIN, disable, last admin, bad PIN saves nothing; USR-04 custom role, Admin fixed, role change counts at once; USR-07 own PIN; USR-09 recovery code and its refusals), `tests/audit_log.rs` (USR-08 needs AuditView, newest first, filters, Excel export read back with `io::read_file`), `tests/permissions.rs` (every guarded command called signed out, as a cashier, as an admin and locked; the Cashier defaults; every guarded command in `COMMANDS` must be listed there).
- **Built (C-M4):** `tests/products.rs` (CAT-01 save with reorder rule and audit, the default tax rate, a refused step saves nothing; CAT-09 price history with the user's name, `CatalogPriceEdit` needed for price changes only; CAT-04 check digits, Code 128 kept, duplicate barcode names the owner, editing barcodes; CAT-11 internal barcode with the prefix and check digit, hand-typed numbers skipped; CAT-06 variants and their list on the parent; CAT-07 kit parts, cycles refused, a kit part can't be deleted; CAT-08 archive, bring back, delete refused after a stock movement, the reorder rule goes with a deleted product; CAT-10 picture resized and shown as a data address, other files refused; CAT-02/CAT-03 units, categories and brands with their refusals and audit; a cashier can look up but not change). `tests/permissions.rs` covers the 21 new commands.
- **Scripted day**: open shift → 200 random sales/returns/voids → close shift → all invariants hold (ledgers = caches, shift expected cash = cash payments sum, sales totals = Σ lines, gapless invoices).

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/app/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/app/` |
| **Screen folder** | `ui/src/app/` (frame, menu, routes, lock screen) and `ui/src/shared/`. Screen flows: `ui/e2e/` (`pnpm e2e`) |
| **Requirements** | All of them; especially [[Workflows]] and [[Roles and Permissions]] |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | — |
| **Used by** (re-run their tests when the public API changes) | — (top of the tree) |
| **Test it on its own** | `cargo nextest run -p app` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Commands stay thin: check permission, call one workflow or query, return.
- Each workflow is one transaction. Printing and the drawer happen after the commit.
- Every new command gets an entry in the permission test.
- If a workflow needs something new from a module, change that module first (with its contract test), then use it here.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- **Built in A0:** the Tauri 2 window (flat, fixed light theme, 1366×768, strict CSP, only `core:default` permissions), `src/lib.rs` `run()`, `src/main.rs`, one command `app_info` → `AppInfo { name, version }` (needs no permission), `commands::COMMANDS` (every command with its permission; the permissions test starts here), `src/migrations.rs` listing every module's `migrations()` in a fixed order, and `tests/contract.rs` (all module migrations apply to an empty database). The binary is called `supershop-pos`.
- **Screens built in A0** (`ui/src/app/`, `ui/src/shared/`): the empty app frame; `shared/ui` `ActionButton`, `IconButton`, `Tip` and the flat shadcn tooltip; the colour tokens in `shared/theme.css` (default Tailwind colours, shadows, blur and animations removed); `text/en.ts` files; the bridge (`call`) with the fake bridge for `pnpm dev:mock` and tests; `errorMessage`; `formatMoney`/`parseMoney` tested against `crates/core/tests/money_vectors.json`; tests for colour contrast (all pairs ≥ 7 : 1) and for the flat look.
- **Changed in A1:** `run()` registers **tauri-plugin-single-instance** first among plugins: starting the app a second time brings the open window to the front (unminimised, focused) instead of opening a second copy. Screen text uses **i18next**: `ui/src/app/i18n.ts` registers every namespace (`common`, `errors`, `app`, and each module's `text/en.ts`), `ui/src/app/i18next.d.ts` types the keys, and components call `useTranslation("<namespace>")`. `errorMessage(error, t)` takes the `errors` namespace's `t`. Module boundaries on screens are checked by **eslint-plugin-boundaries** (`ui/src/app/moduleBoundaries.test.ts` proves a module importing another module fails lint).
- **Added with the reports work:** `src/audit_actions.rs` with `CART_CANCEL`, `SALE_VOID` and `DRAWER_NO_SALE`. Its test checks they match `reports::ACTION_*`, because the reports count events by these names. Later milestones add the other action names here.
- **Built in C-M2:** state, paths, startup, session (with the permission check: `session::require`, there is no separate `guard.rs`), logging, audit_log, jobs, the setup / settings / tax / backup workflows and commands, and the recovery path (see above). Screens: `ui/src/app/AppFrame.tsx` picks the screen from `startup_status` (recovery, setup wizard, sign-in, back office); `SignInPage.tsx` (replaced by the login tiles in C-M3); `OfficeFrame.tsx` (side menu: Home, Printer and devices, Settings, Tax rates, Backups; signed-in name; Sign out; the backup warning banner); `HomePage.tsx`. Module screens: `ui/src/modules/setup/`, `settings/`, `tax/`, `backup/`. Shared: `Alert`, `SelectField`, `Field` (`secret` for passwords), `useSave`, `pickers.ts` (tauri-plugin-dialog; the fake bridge answers fixed paths), `errorField`, `formatPercent`/`parsePercent`. The fake bridge keeps a pretend shop (`shared/bridge/mock/shop.ts`, `resetFakeShop`).
- **Built in C-M3:** login tiles with PIN or password, logout, lock (F12, Lock button, idle timer from `checkout.idle_lock_min`), unlock by the user or by an admin, admin approvals with single-use 60-second tokens (`require_or_approval`), users and roles screens, change own PIN or password, recovery code on the login screen, audit log screen with filters, paging and Excel export (`dialog:allow-save` added to the capability for the save picker). Every permission check reads the user again from `auth`. Screens: `ui/src/modules/auth/` (LoginPage, LockScreen, MyAccountPage, RecoveryForm), `ui/src/modules/users/` (UsersPage, RolesPage), `ui/src/modules/audit/` (AuditLogPage); shared `shared/approval/` (`ApprovalDialog`, `useApproval`), `shared/text/en/permissions.ts` (a plain name per permission), `shared/text/en/approval.ts`, `shared/format/time.ts`, `pickSaveFile`. The menu hides pages the role can't open. The fake bridge keeps pretend users, roles, approvals and audit events (`shared/bridge/mock/people.ts`; owner with password `shop-owner-1`, rina with PIN `1234`). Playwright: `ui/e2e/auth.spec.ts`.
- **Built in C-M4:** `workflows/products.rs` (list, search, choices, details, save, archive, unarchive, delete, internal barcode, variants, kit, picture) and `workflows/catalog_lists.rs` (units, categories, brands), `commands/catalog.rs`, audit actions `product.create`, `product.change`, `product.archive`, `product.unarchive`, `product.delete`, `unit.change`, `category.change`, `brand.change`. Product pictures go in `<data>/images` (`AppPaths::images`). Delete asks `sales`, `inventory` and `purchasing::is_product_used`. Screens: `ui/src/modules/catalog/` (ProductsPage with the ProductEditor and its tabs General, Prices and tax, Barcodes, Stock, Variants, Kit parts, Picture; CategoriesPage, BrandsPage, UnitsPage); menu entries Products (`StockView`) and Categories, Brands, Units (`CatalogManage`); shared `formatQty`/`parseQty`, `pickImageFile`, shortcuts `Ctrl+S` (save) and `Esc` (back). The fake bridge keeps a pretend catalog (`shared/bridge/mock/catalog.ts`). Playwright: `ui/e2e/products.spec.ts`.
- **Product save and pricing (C-M4):** the product form edits only the retail sale price and cost. Price-list prices (Back Office Screens, "Prices & tax") come with the pricing screens (C-M8). Cost changes from the form are recorded with reason "manual"; GRNs use `catalog::set_cost` (C-M10).
- **Product delete (C-M4)** doesn't remove pricing rows (price-list items, promotion targets) or the picture files of the deleted product. Do both when pricing screens exist (C-M8).
- **Approvals so far only on `backup_restore`.** Later milestones call `require_or_approval` for every admin-only till action (void, refunds, price override, discount over the limit, no-sale, cash in/out, force close), and each audit event of such an action records `approved_by`.
- **Lock and carts (C-M6):** "Switch user" on the lock screen simply logs out. When carts exist, refuse it while the locked user has an open cart (USR-03), and hide the cart while locked.
- **Login goes to the back office.** When checkout exists (C-M6), go to Checkout after login if the user has an open shift (USR-01).
- **Recovery code brute force:** `recover_admin` has no lockout. The code has about 98 bits, so guessing is hopeless, but each try costs an argon2 hash; add a short delay if this ever matters.
- **Audit export headers** are English constants in `workflows/audit_view.rs`. Move them next to `ReceiptLabels` when a second language comes (ADR-022). Times are in the PC's current offset, not the offset at the time of the event.
- **Usernames and role names can't be renamed** (auth has no function for it). Add one in `auth` if the shop asks.
- **Users screen** has no "last login" column yet (Back Office Screens); it could come from the newest `session.login` audit event.
- **Not built in C-M2 (later tasks):** the shop logo upload and its preview (SET-04), the receipt's live raster preview (SET-14), payment methods (SET-15), denominations, checkout, inventory, loyalty and credit sections and barcode rules on the Settings screen (SET-16, SET-17; the commands already accept them through `SettingsChange`), printer connections other than Preview in the hardware step and Test print using the saved printer (C-M7), product import and demo data in the wizard (SET-12, C-M12), "Export logs" and the logs folder button (BAK-07), the daily `integrity_check` on the newest backup, backups before imports and count postings (C-M12, C-M5), admins choosing a target folder for "Back up now" (BAK-04), `tauri-plugin-window-state`, React Router and TanStack Query (the frame switches pages with `useState`), and `log_client_error`.
- **Restart after restore** uses `AppHandle::restart()`. Checked on Linux in tests only through the marker file; check the real restart on Windows and the Mac.
- **Pre-upgrade backup** is made when an existing database has pending migrations. No test yet, because every migration so far is the first release's; add one with the first new migration.
- To do: the remaining shared widgets (`MoneyInput`, `QtyInput`, `Dialog`, `SidePanel`, `DataTable`, `StatusLight`, `ColourLegend`), TanStack Query, React Router, the `F1` help overlay, and the ADR-014 test that checks `api.ts` command names against `COMMANDS`.
- **Built in C-M1:** `devices.rs`, `preview.rs`, `receipt_words.rs`, `dev_tools.rs`, `workflows/test_print.rs`, `commands/hardware.rs`, `commands/dev.rs` (see above). The app frame now loads `app_info` at start and shows the "Printer and devices" page (`ui/src/modules/hardware/`) under the welcome panel; menus and routes come in C-M2. New shared widgets: `Field` (label + input + required tip + error under it), `CheckBox`, `Badge`. `errorMessage` shows a message per hardware code (`hardware_timeout`, …). The fake bridge is loaded with a dynamic import, only in mock and test modes. Playwright is set up (`ui/playwright.config.ts`, `ui/e2e/hardware.spec.ts`).
- **Golden test receipts are not approved yet** (`crates/app/tests/golden/`). The product owner looks at them in the Preview printer (Mac build from the pull request) and approves them, or asks for changes.
- **Test print uses sample shop details and a sample logo** (`assets/sample-logo.png`) and a fixed date. When settings are loaded (C-M2), use the shop's header, logo, currency and paper width, and the real date.
- **Permissions (C-M1):** `hardware_test_print` needs no login only because it reaches no device yet. When it can send to a real printer (C-M7), it needs `SettingsManage` (except during first-run setup).
- **Hardware thread (C-M7):** device calls run on Tauri's blocking pool through `on_worker`. Replace with `hardware_thread.rs` (one queue, so print jobs never interleave) when real printers are used.
- **Preview printer as its own window:** for now it is a panel on the "Printer and devices" page that checks for printouts every second. When receipts print after sales (C-M7), open it as a separate window or push an event instead of polling.
- **Print dialog:** `print_window` is checked on Linux only. Check it on the Mac (WKWebView) and on Windows (WebView2, the reference) in M13; the print CSS is in `ui/src/shared/theme.css`.
- **Tooltips that open while the page scrolls** close again (Radix closes a tooltip on scroll). The Tab key scrolls a field into view, so a field below the fold shows its tooltip only on the next focus. Fix when the page gets its final layout, or keep pages short enough not to scroll at 1366×768.
- CI runs Rust tests on Linux (every push) and Windows (pull requests). macOS only builds the `.dmg` (label `build-mac` or a version tag), so the Mac test run in [[Test Strategy]] isn't in CI yet.
