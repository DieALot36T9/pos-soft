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
├── state.rs           # AppState { db, session: Mutex<Option<Session>>, overrides, clock, fonts, hw: HardwareHandles, log_reload }
├── session.rs         # Session { user, role, permissions, max_discount }
├── guard.rs           # Ctx::require(Permission), require_or_override(Permission, token)
├── migrations.rs      # list of every module's migrations() in a fixed order
├── jobs.rs            # backup timer thread
├── hardware_thread.rs # queue for print/drawer/scale requests
├── audit_actions.rs   # every audit action name as a constant
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
| `catalog.rs` | product_list, product_get, product_save, product_archive, product_delete, product_search, barcode_generate, variants_generate, kit_save, unit_*, category_*, brand_*, product_image_set |
| `pricing.rs` | price_list_*, tier_*, promotion_* |
| `tax.rs` | tax_rate_list, tax_rate_save, tax_rate_deactivate |
| `inventory.rs` | stock_levels, stock_history, stock_adjust, low_stock, near_expiry, count_create, count_line, count_variances, count_post, count_cancel, stock_verify |
| `purchasing.rs` | supplier_*, po_*, grn_prepare, grn_save, supplier_return_save, supplier_payment, payables_aging, supplier_statement |
| `customers.rs` | customer_lookup, customer_get, customer_save, customer_payment, points_adjust, customer_statement |
| `reports.rs` | report_run, report_export_xlsx, report_print |
| `io.rs` | import_preview, import_commit, import_cancel, export_entity, template_download |
| `labels.rs` | labels_preview, labels_print, shelf_tags_print |
| `users.rs` | user_list, user_save, role_list, role_save |
| `settings.rs` | settings_get, settings_save, payment_methods_*, hardware_list, hardware_test_print, hardware_open_drawer, hardware_read_scale |
| `backup.rs` | backup_now, backup_list, backup_restore |
| `system.rs` | logs_set_detailed, logs_export, log_client_error, audit_query, app_info, data_verify |

## Tests (`app/tests/`)
- **Every workflow**: the happy path plus a failure injected at each step (a fake module error via a test hook) → nothing saved.
- **Permissions table test**: every command → its required permission; a cashier session is refused where it should be.
- **Scripted day**: open shift → 200 random sales/returns/voids → close shift → all invariants hold (ledgers = caches, shift expected cash = cash payments sum, sales totals = Σ lines, gapless invoices).

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/app/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/app/` |
| **Screen folder** | `ui/src/app/` (frame, menu, routes, lock screen) and `ui/src/shared/` |
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
- To do: `state.rs` (AppState), `session.rs`, `guard.rs`, `jobs.rs`, logging (tracing + reload layer + daily files), the `run` wrapper (spawn_blocking + span + panic mapping), opening the database and migrating at start-up (after a backup) — milestone C-M2.
- To do: the remaining shared widgets (`Field`, `MoneyInput`, `QtyInput`, `Dialog`, `SidePanel`, `DataTable`, `Badge`, `StatusLight`, `ColourLegend`), TanStack Query, React Router, the `F1` help overlay, Playwright e2e (`pnpm e2e`), and the ADR-014 test that checks `api.ts` command names against `COMMANDS`.
- CI runs Rust tests on Linux (every push) and Windows (pull requests). macOS only builds the `.dmg` (label `build-mac` or a version tag), so the Mac test run in [[Test Strategy]] isn't in CI yet.
