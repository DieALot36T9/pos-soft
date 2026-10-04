---
tags: [requirements, security]
---
# Roles and Permissions

**Rule from the product owner:** actions that only an admin should do need the admin password. **Anyone who is logged in can do everything else.**

The app ships two **system roles**:
- **Admin**: every permission. It can't be edited or deleted. There must always be at least one active admin.
- **Cashier**: the default set below (everything marked **C**).

Admins can create **custom roles** (e.g. "Supervisor") by ticking permissions. This keeps the app universal.

When a cashier tries an admin-only action, the app shows an **override dialog**. An admin enters their username and password, and that **one action** runs. The audit log records both users ([[Req - Users and Security]] USR-05).

## Permission list
`C` = Cashier by default · `A` = Admin only by default

| Permission (Rust enum) | Default | What it allows |
|---|---|---|
| `CheckoutSell` | C | Scan, search, add lines, change quantity, remove lines, take payment |
| `CheckoutCancelCart` | C | Cancel an in-progress sale |
| `CheckoutHoldResume` | C | Hold and resume sales |
| `CheckoutManualWeight` | C | Type a weight instead of reading the scale |
| `DiscountApply` | C (limit 0%) | Apply line/invoice discounts up to the role's `max_discount_bps`. Above the limit needs an override |
| `PriceOverride` | A | Change a line's unit price |
| `PriceListSwitch` | A | Sell at a different price list (e.g. wholesale) |
| `SellOnCredit` | C | Use the customer-credit payment within the customer's credit limit |
| `CreditLimitExceed` | A | Go over a customer's credit limit |
| `ReceiptReprintLast` | C | Reprint the last receipt of this till |
| `ReceiptReprintAny` | A | Reprint any past receipt |
| `SaleVoid` | A | Void a completed sale |
| `ReturnWithReceipt` | A | Process a return against an invoice |
| `ReturnWithoutReceipt` | A | Process a return without an invoice |
| `ShiftOpenCloseOwn` | C | Open and close one's own shift; X report for one's own shift |
| `ShiftCashInOut` | A | Paid-outs, cash drops, cash in |
| `ShiftNoSaleDrawer` | A | Open the drawer without a sale |
| `ShiftForceClose` | A | Close another user's abandoned shift |
| `CustomerViewCreate` | C | Look up customers, create new ones, see purchase history |
| `CustomerManage` | A | Edit or archive customers, set tier and credit limit |
| `CustomerReceivePayment` | C | Take a payment against customer debt |
| `LoyaltyAdjust` | A | Manually add or remove points |
| `StockView` | C | See stock levels, batches, history |
| `StockAdjust` | A | Make stock adjustments |
| `StockCountEnter` | C | Enter counted quantities in an open stock count |
| `StockCountManage` | A | Create, post or cancel stock counts |
| `CatalogManage` | A | Create, edit and archive products, categories, brands, units, barcodes, images, variants, kits |
| `CatalogPriceEdit` | A | Change sale and cost prices |
| `PricingManage` | A | Price lists, tiers, promotions |
| `TaxManage` | A | Tax rates and tax mode |
| `PurchasingManage` | A | Suppliers, purchase orders, receiving (GRN), supplier returns |
| `PurchasingPay` | A | Record supplier payments |
| `LabelsPrint` | C | Print barcode labels and shelf tags |
| `ReportsViewAll` | A | All reports (cashiers only see their own shift's X/Z) |
| `DataImport` | A | CSV/Excel import |
| `DataExport` | A | CSV/Excel export |
| `UsersManage` | A | Users and roles |
| `SettingsManage` | A | All settings, including hardware |
| `BackupRun` | C | Run a manual backup now |
| `BackupRestore` | A | Restore a backup |
| `AuditView` | A | View and export the audit log |
| `LogsManage` | A | Switch detailed logging on or off, export logs |

## Rules
- Permissions are checked **only in Rust** (`app` command layer). The UI hides buttons as a convenience, not for security.
- An override token is good for **one permission, one action, for 60 seconds**.
- The Admin role always has every permission, including ones added in later versions.
- New permissions added in later versions default to **Admin only**.
