---
tags: [module]
crate: sales
milestone: M6, M9
---
# Module: sales

**Purpose:** carts (in progress and held), completed sales with snapshots, payments, payment methods, invoice numbers, returns and voids.

**Requirements:** [[Req - Checkout]], [[Req - Payments]], [[Req - Returns and Voids]], [[Req - Setup and Settings]] SET-15, [[Req - Backup and Logging]] BAK-05 (restore list). **Design:** [[ADR-011 Snapshots on sale lines]], [[Workflows]], [[Money Tax and Rounding]].
**Depends on:** `core`, `rusqlite`, `serde` + `serde_json` (the tax parts JSON on sale lines), `thiserror`. Tests also use `proptest`.

**Built in task B-sales.** No `app` wiring and no screens yet (those come in C-M6 and C-M9).

## Tables
The full SQL is in `crates/sales/migrations/001_init.sql`. In short:
```sql
CREATE TABLE sales_payment_methods (
  id TEXT PRIMARY KEY, name TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('cash','card','wallet','other','credit','points','exchange_credit')),
  requires_reference INTEGER NOT NULL DEFAULT 0, active INTEGER NOT NULL DEFAULT 1,
  sort INTEGER NOT NULL DEFAULT 0, is_system INTEGER NOT NULL DEFAULT 0
);  -- + one system method per system kind (unique index); the migration adds Cash, Customer credit,
    --   Loyalty points and Exchange credit with fixed ids 0…01 to 0…04
CREATE TABLE sales_sequences (till_id TEXT NOT NULL, doc TEXT NOT NULL CHECK (doc IN ('sale','return')),
  next INTEGER NOT NULL CHECK (next >= 1), PRIMARY KEY (till_id, doc));   -- rows made on first use

-- Carts: saved on every change. A cashier has at most one open cart per till (unique index).
CREATE TABLE sales_carts (
  id TEXT PRIMARY KEY, till_id TEXT NOT NULL, shift_id TEXT NOT NULL, user_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('open','held')),
  customer_id TEXT, price_list_id TEXT, note TEXT,
  invoice_discount_kind TEXT CHECK (invoice_discount_kind IN ('percent','amount')), invoice_discount_value INTEGER,
  exchange_return_id TEXT,
  created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE sales_cart_lines (
  id TEXT PRIMARY KEY, cart_id TEXT NOT NULL REFERENCES sales_carts(id) ON DELETE CASCADE,
  seq INTEGER NOT NULL, product_id TEXT NOT NULL, qty_milli INTEGER NOT NULL CHECK (qty_milli > 0),
  source TEXT NOT NULL CHECK (source IN ('scan','search','scale_label','manual')),
  embedded_price_minor INTEGER, price_override_minor INTEGER, override_by TEXT,
  manual_discount_kind TEXT CHECK (manual_discount_kind IN ('percent','amount')), manual_discount_value INTEGER,
  added_at INTEGER NOT NULL
);

-- Completed sales: never updated except void fields (triggers refuse anything else)
CREATE TABLE sales_sales (
  id TEXT PRIMARY KEY, invoice_no TEXT NOT NULL UNIQUE,
  till_id TEXT NOT NULL, shift_id TEXT NOT NULL, user_id TEXT NOT NULL, business_date TEXT NOT NULL, completed_at INTEGER NOT NULL,
  customer_id TEXT, customer_name TEXT, customer_phone TEXT, price_list_id TEXT,
  status TEXT NOT NULL CHECK (status IN ('completed','voided')),
  base_minor INTEGER NOT NULL, promo_discount_minor INTEGER NOT NULL, tier_discount_minor INTEGER NOT NULL,
  manual_discount_minor INTEGER NOT NULL, invoice_discount_minor INTEGER NOT NULL,
  net_minor INTEGER NOT NULL, tax_minor INTEGER NOT NULL, gross_minor INTEGER NOT NULL,
  rounding_minor INTEGER NOT NULL, total_minor INTEGER NOT NULL, change_minor INTEGER NOT NULL,
  cost_minor INTEGER NOT NULL,
  tax_mode TEXT NOT NULL CHECK (tax_mode IN ('inclusive','exclusive')), exchange_return_id TEXT,
  voided_at INTEGER, voided_by TEXT, void_approved_by TEXT, void_reason TEXT
);  -- indexes: business_date, shift_id, (customer_id, completed_at), (till_id, completed_at)
CREATE TABLE sales_lines (
  id TEXT PRIMARY KEY, sale_id TEXT NOT NULL REFERENCES sales_sales(id), seq INTEGER NOT NULL,
  product_id TEXT NOT NULL, name TEXT NOT NULL, sku TEXT NOT NULL, unit_symbol TEXT NOT NULL,
  category_id TEXT NOT NULL, brand_id TEXT,
  qty_milli INTEGER NOT NULL, unit_price_minor INTEGER NOT NULL, retail_price_minor INTEGER NOT NULL,
  base_minor INTEGER NOT NULL, promotion_id TEXT, promotion_name TEXT, promo_discount_minor INTEGER NOT NULL,
  tier_discount_minor INTEGER NOT NULL, manual_discount_minor INTEGER NOT NULL, invoice_discount_minor INTEGER NOT NULL,
  net_minor INTEGER NOT NULL, tax_minor INTEGER NOT NULL, gross_minor INTEGER NOT NULL,
  tax_rate_id TEXT NOT NULL, tax_name TEXT NOT NULL, tax_bps INTEGER NOT NULL, tax_components_json TEXT NOT NULL,
  unit_cost_minor INTEGER NOT NULL, cost_minor INTEGER NOT NULL, price_overridden INTEGER NOT NULL DEFAULT 0
);  -- tax_components_json: [{"name":"CGST","bps":900,"tax_minor":652}], or [] when the tax has no parts

CREATE TABLE sales_returns (
  id TEXT PRIMARY KEY, return_no TEXT NOT NULL UNIQUE, sale_id TEXT REFERENCES sales_sales(id),
  till_id TEXT NOT NULL, shift_id TEXT NOT NULL, user_id TEXT NOT NULL, approved_by TEXT,
  business_date TEXT NOT NULL, at INTEGER NOT NULL, customer_id TEXT, reason TEXT,
  net_minor INTEGER NOT NULL, tax_minor INTEGER NOT NULL, gross_minor INTEGER NOT NULL,
  rounding_minor INTEGER NOT NULL, total_minor INTEGER NOT NULL, cost_minor INTEGER NOT NULL
);
CREATE TABLE sales_return_lines (
  id TEXT PRIMARY KEY, return_id TEXT NOT NULL REFERENCES sales_returns(id), sale_line_id TEXT REFERENCES sales_lines(id),
  product_id TEXT NOT NULL, name TEXT NOT NULL, qty_milli INTEGER NOT NULL,
  net_minor INTEGER NOT NULL, tax_minor INTEGER NOT NULL, gross_minor INTEGER NOT NULL,
  tax_rate_id TEXT NOT NULL, tax_bps INTEGER NOT NULL, unit_cost_minor INTEGER NOT NULL, cost_minor INTEGER NOT NULL,
  restock INTEGER NOT NULL CHECK (restock IN (0,1))
);

CREATE TABLE sales_payments (
  id TEXT PRIMARY KEY, doc_type TEXT NOT NULL CHECK (doc_type IN ('sale','return')), doc_id TEXT NOT NULL,
  method_id TEXT NOT NULL REFERENCES sales_payment_methods(id), kind TEXT NOT NULL,
  amount_minor INTEGER NOT NULL, tendered_minor INTEGER, reference TEXT, at INTEGER NOT NULL
);
-- Triggers: sales_lines, sales_payments, sales_returns and sales_return_lines refuse every UPDATE and DELETE.
-- sales_sales refuses DELETE, and any UPDATE except completed → voided with the void fields.
```

**Changes from the first plan** (made in B-sales, before any release):
- `sales_sales` has `tier_discount_minor` and `invoice_discount_minor`, so each kind of line discount adds up to its own header total. Before, the header had nowhere to put them.
- `sales_lines.cost_minor` and `sales_return_lines.cost_minor` store round(unit cost × quantity). Returns need the cost already refunded to give back exactly what is left (RET-02), and profit reports can read it directly.
- `sales_payment_methods` gets the four system methods from the migration. `sales_sequences` rows are made on first use, so setup doesn't need to create them.

## Published views
| View | Columns | Used by |
|---|---|---|
| `sales_v_sales` | every `sales_sales` column | reports: daily summary, shift report |
| `sales_v_lines` | every `sales_lines` column + `invoice_no, business_date, completed_at, till_id, shift_id, user_id, customer_id, status` | reports: sales by, profit, tax, discounts |
| `sales_v_return_lines` | every `sales_return_lines` column + `return_no, business_date, at, till_id, shift_id, user_id, customer_id, sale_id` | reports: sales by, profit, tax |
| `sales_v_payments` | every `sales_payments` column + `business_date, shift_id, doc_status` (`voided` for a voided sale, otherwise `completed`) | reports: daily summary, shift report |

## Public API
Functions that change data take `&rusqlite::Transaction`; functions that read take `&rusqlite::Connection`. Every function returns `Result<_, SalesError>`, which turns into `AppError`.
```rust
// carts (cart.rs, cart_edit.rs, cart_hold.rs)
pub fn open_cart(tx, till: &Id, shift: &Id, user: &Id, now) -> Result<Id>;         // cart_already_open
pub fn get_cart(conn, cart: &Id) -> Result<Cart>;                                    // with its lines, in order
pub fn current_cart(conn, till: &Id, user: &Id) -> Result<Option<Cart>>;             // the open cart (CHK-19)
pub fn add_line(tx, cart: &Id, NewCartLine, merge: bool, now) -> Result<Id>;          // returns the line used
pub fn set_qty(tx, line: &Id, Qty, now) -> Result<()>;                               // 0 removes
pub fn remove_line(tx, line: &Id, now) -> Result<()>;
pub fn set_line_discount(tx, line: &Id, Option<Discount>, now) -> Result<()>;
pub fn set_price_override(tx, line: &Id, Option<PriceOverride>, now) -> Result<()>;
pub fn set_invoice_discount(tx, cart: &Id, Option<Discount>, now) -> Result<()>;
pub fn set_customer(tx, cart: &Id, Option<&Id>, now) -> Result<()>;
pub fn set_price_list(tx, cart: &Id, Option<&Id>, now) -> Result<()>;
pub fn set_exchange_return(tx, cart: &Id, Option<&Id>, now) -> Result<()>;           // RET-05
pub fn hold(tx, cart: &Id, note: Option<String>, now) -> Result<()>;                 // cart_empty
pub fn held_carts(conn, till: &Id) -> Result<Vec<HeldCartRow>>;                      // oldest first
pub fn resume(tx, cart: &Id, user: &Id, shift: &Id, now) -> Result<()>;              // cart_not_held, cart_in_progress
pub fn cancel(tx, cart: &Id) -> Result<Cart>;                                         // returns the snapshot for audit
pub fn open_carts_on_shift(conn, shift: &Id) -> Result<u32>;                         // open carts with lines
// payments (settle.rs)
pub fn settle_payments(total_due: Money, &[EnteredPayment]) -> Result<Settlement>;   // pure: applied amounts + change
// completion (complete.rs, sale_read.rs, void.rs)
pub fn complete(tx, CompletedSale /* header + snapshot lines + payments */) -> Result<SaleRef>; // assigns invoice_no, deletes the cart
pub fn get_sale(conn, &Id) -> Result<Sale>;
pub fn find_by_invoice(conn, &str) -> Result<Option<Sale>>;
pub fn last_sale(conn, till: &Id) -> Result<Option<Sale>>;                            // the one saved last
pub fn returnable_qty(conn, sale: &Id) -> Result<HashMap<Id /*line*/, Qty>>;
pub fn void(tx, sale: &Id, by: &Id, approved_by: Option<&Id>, reason: &str, now) -> Result<()>;
pub fn sales_summary(conn) -> Result<SalesSummary>;                                  // BAK-05: sale count + last invoice no
pub fn has_any_sale(conn) -> Result<bool>;                                             // true once any sale or return exists
// returns (returns.rs, return_read.rs)
pub fn refund_amounts(conn, sale: &Id, &[ReturnQty]) -> Result<Vec<LineRefund>>;      // preview, saves nothing
pub fn record_return(tx, NewReturn) -> Result<ReturnRef>;                             // assigns return_no
pub fn get_return(conn, &Id) -> Result<Return>;
// payment methods (payment_method.rs)
pub fn payment_methods(conn) -> Result<Vec<PaymentMethod>>;                          // in list order
pub fn payment_method(conn, &Id) -> Result<PaymentMethod>;
pub fn system_payment_method(conn, PaymentKind) -> Result<PaymentMethod>;           // e.g. the cash method
pub fn save_payment_method(tx, PaymentMethodInput) -> Result<Id>;                   // id None = add
pub fn migrations() -> ModuleMigrations;
```
Main types: `Cart`, `CartLine`, `CartStatus`, `LineSource`, `Discount` (`Percent(Bps)` / `Amount(Money)`), `PriceOverride`, `NewCartLine`, `HeldCartRow`; `CompletedSale`, `LineSnapshot`, `SaleTotals`, `TaxMode`, `TaxPart`, `PromotionSnapshot`, `CustomerSnapshot`, `NewPayment`, `SaleRef`, `Sale`, `SaleLine`, `SaleStatus`, `VoidInfo`, `Payment`; `EnteredPayment`, `Settlement`; `ReturnQty`, `LineRefund`, `LineAmounts`, `NewReturn`, `NewReturnLine` (`FromSale` / `WithoutReceipt`), `ReturnedSaleLine`, `ReturnedProduct`, `ReturnRef`, `Return`, `ReturnLine`, `ReturnTotals`; `SalesSummary`; `PaymentKind`, `PaymentMethod`, `PaymentMethodInput`; `SalesError`.

The sales module **does not calculate prices or tax**. `complete` receives fully calculated snapshot lines from the workflow and checks that they're consistent before saving anything.

## Rules and notes
**What `complete` checks** (a failed check is `SalesError::TotalsDontAddUp`, an internal error, because it means the workflow has a bug):
- The cart exists, is open, is on the same till, and its lines have the same products and quantities in the same order.
- Each line: quantity > 0; no negative amount; `net + tax = gross`; `base − promo − tier − manual − invoice` equals `gross` (tax inclusive) or `net` (tax exclusive); tax parts, if any, add up to `tax`. So a discount can never make a line negative (CHK-12).
- Each header total equals the sum over the lines: base, each discount, net, tax, gross, and cost (Σ round(unit cost × quantity)). `total = gross + rounding`.
- Payments: each method exists and is active; a reference is given if the method needs one (PAY-02); only cash has a `tendered` amount, never less than its `amount`; Σ amounts = total; change = Σ cash (tendered − amount). Paying too little is `payment_not_enough` (a validation error, with `remaining_minor`).
- Then it takes the next invoice number, saves header, lines and payments, and deletes the cart, all in the caller's transaction.

**Payments** (`settle_payments`, PAY-01, PAY-03): applied in the order entered. Non-cash payments may not be more than what is left (`payment_over_remaining`); cash may, and the extra is change. Its output (`Settlement.payments`, `change`) is what `complete` expects.

**Invoice and return numbers** (PAY-08, RET-06): `{till_code}-{n:06}` and `{till_code}-R{n:06}`, gapless per till. The number is taken inside the same transaction as the document, so a rollback gives it back.

**Refunds** (RET-02, E14): `refund_amounts` and `record_return` work them out from the stored sale line, never from today's prices. Returning the last units refunds exactly what is left of net, tax, gross and cost. Otherwise gross, tax and cost are each round(stored × returned ÷ sold), never more than what is left, and net = gross − tax. The "never more than what is left" limit is needed because many small returns can otherwise round up to more than the line (5 minor units over 7 units). A property test checks that any sequence of returns adds up to the line exactly.

**Returns:** a return with a receipt has `sale_id` and only `FromSale` lines; a return without a receipt (RET-03) has no `sale_id` and only `WithoutReceipt` lines, priced by the workflow. Refund payments must add up to `gross + rounding` exactly. A voided sale can't be returned (`sale_voided`).

**Voids** (RET-04): only a completed sale with no returns can be voided (`sale_has_returns`, `sale_already_voided`); a reason is required. The workflow checks `SaleVoid` and that `Sale::shift_id` is still open.

**Backup restore list** (BAK-05): `app` opens a backup with `db::open_read_only` and calls `sales_summary` on it. It counts every sale, voided ones included (they keep their number), and gives the invoice number of the sale saved last. It only reads, and a backup made before the sales tables existed gives 0 sales and no invoice number.

**Exchange** (RET-05): the return is recorded first (refund method `exchange_credit`). The new cart gets `set_exchange_return`; the sale stores `exchange_return_id`. The link is kept on the sale, because the return is saved first and is never changed afterwards.

**Error codes** (`SalesError::Invalid` → `AppError::Validation`, `SalesError::Conflict` → `AppError::Conflict`):
`qty_not_positive`, `qty_too_large`, `price_negative`, `discount_invalid`, `cart_empty`, `till_code_missing`, `payment_amount_invalid`, `payment_not_needed`, `payment_over_remaining`, `payment_not_enough`, `payment_reference_missing`, `payment_method_inactive`, `payment_method_name_missing`, `payment_kind_reserved`, `payment_kind_fixed`, `payment_cash_required`, `return_qty_too_large` (`max_milli`), `return_line_repeated`, `return_lines_mixed`, `return_empty`, `void_reason_missing` · conflicts: `cart_already_open`, `cart_not_open`, `cart_not_held`, `cart_in_progress`, `sale_already_voided`, `sale_has_returns`, `sale_voided`.

## Tests
Every test names its requirement. `cargo nextest run -p sales` (84 tests).

| Page promise | Tests |
|---|---|
| Invoice numbers are gapless per till under 1,000 sequential completes | `complete::pay_08_invoice_numbers_are_gapless_over_1000_sales`, `pay_08_each_till_counts_on_its_own`, `numbers::pay_08_invoice_numbers_have_the_till_code_and_six_digits` |
| A failed transaction doesn't use a number | `complete::pay_07_a_failed_transaction_does_not_use_a_number`, `carts::chk_05_cancel_returns_the_cart_and_uses_no_invoice_number` |
| `complete` refuses inconsistent totals | `complete::pay_07_complete_refuses_header_totals_that_dont_match_the_lines`, `pay_07_complete_refuses_a_line_that_doesnt_add_up`, `pay_07_complete_refuses_lines_that_dont_match_the_cart`, `pay_07_complete_refuses_a_held_or_missing_cart`, `pay_03_complete_refuses_a_short_payment`, `pay_01_change_that_doesnt_match_the_cash_is_refused`, `pay_03_non_cash_payments_have_no_tendered_amount`, `pay_02_a_method_that_needs_a_reference_refuses_a_payment_without_one` |
| A sale is saved with its snapshot; it never changes | `complete::pay_07_complete_saves_the_snapshot_and_deletes_the_cart`, `pay_01_complete_stores_cash_tendered_and_change`, `sale_history::pay_07_a_completed_sale_can_never_be_edited_or_deleted`, `sale_history::pay_10_ret_01_reading_sales_back_by_invoice_and_last_on_the_till`, `sale_history::pay_10_last_sale_is_the_one_saved_last_even_in_the_same_millisecond` |
| Payments and change (E12, E13) | `settle::e12_pay_01_cash_change`, `e13_pay_03_split_wallet_then_cash`, `pay_03_*` (5 more) |
| Merge vs non-merge scans | `carts::chk_01_repeat_scan_merges_into_the_same_line`, `chk_01_without_merge_each_scan_is_a_new_line`, `chk_08_price_label_lines_never_merge`, `chk_12_a_discounted_line_is_not_merged_into` |
| Quantity 0 removes | `carts::chk_04_quantity_zero_removes_the_line`, `chk_04_negative_quantities_are_refused`, `chk_05_remove_line` |
| Hold / resume / cancel | `carts::chk_09_*` (3), `chk_10_*` (3), `chk_05_cancel_returns_the_cart_and_uses_no_invoice_number`, `chk_19_a_cashier_has_only_one_open_cart_per_till`, `chk_11_12_13_17_cart_settings_are_saved`, `chk_12_impossible_discounts_are_refused`, `shf_06_open_carts_on_shift_counts_only_carts_with_lines` |
| Cart restored after reopening the database | `crash_recovery::chk_19_cart_is_back_after_reopening_the_database` (a file database, closed and reopened; held carts too) |
| `returnable_qty` after partial returns | `returns::ret_01_returnable_quantity_after_partial_returns`, `ret_01_returning_more_than_is_left_is_refused` (sold × 3, 1 returned, return 3 → max 2), `ret_01_a_line_from_another_sale_or_repeated_is_refused` |
| Exact refunds (E14) | `refund::e14_ret_02_three_single_returns_refund_the_line_exactly`, `returns::e14_ret_02_three_single_returns_refund_the_line_exactly`, `ret_02_the_last_return_refunds_exactly_what_is_left`, `refund::ret_02_refunds_of_a_line_always_add_up_to_the_line` (proptest), `ret_02_many_small_returns_never_refund_more_than_the_line` |
| Returns: numbers, rounding, without a receipt | `returns::ret_06_return_numbers_count_per_till_with_an_r`, `ret_01_refunds_must_add_up_to_the_return_total`, `ret_03_return_without_a_receipt`, `ret_06_returns_can_never_be_edited_or_deleted` |
| Void (the workflow checks the shift; the module gives the shift id) | `voids::ret_04_void_keeps_the_number_and_contents` (checks `shift_id`), `ret_04_void_needs_a_reason_and_happens_once`, `returns::ret_04_a_sale_with_returns_cannot_be_voided_and_a_void_sale_cannot_be_returned` |
| Exchange link | `sale_history::ret_05_exchange_sale_keeps_the_link_to_its_return` |
| Payment methods | `payment_methods::set_15_*` (6), `pay_02_an_inactive_method_cannot_take_a_payment` |
| Backup restore list (sale count, last invoice number) | `backup_summary::bak_05_a_database_without_sales_shows_none`, `bak_05_a_backup_made_before_the_sales_tables_existed_shows_none`, `bak_05_counts_every_sale_and_gives_the_last_invoice_number`, `bak_05_works_on_a_backup_opened_read_only` |
| The views return the expected columns (contract, [[Module Rules]] R2) | `contract::sales_v_sales_has_the_sale_columns`, `sales_v_lines_has_the_line_and_header_columns`, `sales_v_return_lines_has_the_line_and_header_columns`, `sales_v_payments_has_the_payment_columns_and_the_document_date`, `views_show_sales_returns_and_voids_for_reports` |
| Migrations and names ([[Module Rules]] R2, R4); errors ([[Errors and Logging]]) | `contract::migrations_belong_to_this_module`, `every_table_view_index_and_trigger_is_prefixed_with_sales`, `errors_turn_into_app_errors_with_stable_codes`, `error::*` (3) |

Test helpers live in `crates/sales/tests/support/mod.rs`: `test_db()` (in-memory SQLite with foreign keys on and this module's migrations), a `Till`, and `priced_line`/`sale_of_cart`/`sell`, which stand in for pricing and tax.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/sales/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/sales/` |
| **Screen folder** | `ui/src/modules/checkout/` (till screen, payment, held sales, returns) and `ui/src/modules/sales/` (sales journal, receipt viewer) |
| **Requirements** | [[Req - Checkout]], [[Req - Payments]], [[Req - Returns and Voids]], SET-15 in [[Req - Setup and Settings]], BAK-05 in [[Req - Backup and Logging]] |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | **Fully priced** lines from `app` (sales never calculates prices or tax), plus the shift, till (id and code) and user ids, the business date and "now" |
| **Used by** (re-run their tests when the public API changes) | All checkout commands, returns, voids, closing a shift (`open_carts_on_shift`), settings (`has_any_sale`, payment methods), backup restore list (`sales_summary`), reports (`sales_v_*` views) |
| **Test it on its own** | `cargo nextest run -p sales` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- Invoice numbers must be gapless. They're assigned inside `complete()`, in the same transaction.
- Carts are saved on every change, so a crash loses nothing. Every cart change takes `now` and updates `updated_at`.
- A completed sale is never changed, except its void fields. Triggers in the database enforce this too.
- `complete()` refuses totals that don't add up. Don't loosen this check.
- `complete()` expects the lines in the cart's order, one per cart line. If pricing ever splits or adds lines, change the check here in the same change.
- Repeat scans merge only into a **plain** line (no scale price, override or discount). The workflow passes `merge = false` for weighed items or when the shop turned merging off (SET-17).
- An empty open cart doesn't block closing a shift (`open_carts_on_shift` counts carts with lines only).
- Never sort by `id` to find "the newest": ULIDs made in the same millisecond don't sort by creation. `last_sale` uses `rowid` (sales are never deleted, so it only grows).

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- **No `serde`/`ts-rs` derives on the public types yet.** Add them in C-M6 when the screens need `Cart`, `HeldCartRow`, `PaymentMethod`, `Sale` and `Return` (and run `cargo xtask gen-types`). `ts-rs` must then be added to this page's "Depends on".
- **Payment method colour** ([[Design System]], "Payment method colours": the admin can change it): there is no column for it yet. Add one in a new migration when C-M2 builds the Settings screen.
- **Stale shift on an open cart:** if a shift is force-closed (SHF-07) while a cashier's cart is open, that cart keeps the old `shift_id`. `resume` updates the shift of a held cart, but there is no function to move an open cart to a new shift. C-M6 should decide: hold it during force close, or add `move_to_shift`.
- **Voiding an exchange sale** leaves its return's exchange credit unspent. C-M9 decides what the void workflow does with it (for example, refund it in cash).
- **Workflows page names:** [[Workflows]] says `sales::cart(conn, id)` and `sales::open_carts(shift)`. The functions are `get_cart` and `open_carts_on_shift`. Fix the wording there when C-M6 writes the workflow.
- **First-run setup** ([[Workflows]], "First-run setup") says the last step creates the first sequence rows. It doesn't need to: `sales_sequences` rows are made on first use.

## Change log (significant API/table changes)
| Date | Change |
|---|---|
| 2026-10-04 | B-sales: first version. Header gets `tier_discount_minor` and `invoice_discount_minor`; lines and return lines get `cost_minor`; system payment methods added by the migration; `get_cart`, `set_exchange_return`, `settle_payments`, `refund_amounts`, `get_return`, `payment_method`, `system_payment_method` added; every cart change takes `now` |
| 2026-10-05 | `last_sale` finds the sale saved last by `rowid` (ids made in the same millisecond don't sort by creation). `sales_summary` added for the backup restore list (BAK-05) |
