---
tags: [ui]
---
# Back Office Screens

Common pattern for list screens: **search box + filters on top, virtualized table, row click opens a side panel or detail page, "New" button top-right, Export button** (if permitted). Forms show Rust validation errors on the right field. **Every button and field has a tooltip.** Colours follow the [[Design System]]: green Save/New, red Archive/Delete, blue Search/Export, grey Back.

## Dashboard
Cards: today's net sales, number of sales, average basket, open shift info. Alert lists: low stock (count + link), near expiry, negative stock, backup warnings, printer not configured.

## Products
- **List:** columns name, SKU, category, brand, price, cost, stock, status. Filters: category, brand, archived, low stock, no barcode.
- **Edit page** (tabs): General (name, SKU, category, brand, unit, kind) · Prices & tax (sale, cost, tax rate, price-list prices, price history) · Barcodes (list, add, generate internal) · Stock (track stock/expiry, reorder level/qty, preferred supplier, current stock and batches, link to history) · Variants (attributes, generate, variant table) · Kit (components) · Image.
- Categories (tree editor), Brands, Units: simple lists with inline edit.
- **Labels & shelf tags:** choose products (search, category, "from GRN", "price changed since"), template, quantity per product, preview (real PNG), print.

## Pricing
Price lists (list + item grid with bulk paste), Tiers, Promotions (form: kind, values, targets picker, schedule, tier, preview "Soap 50.00 × 3 → 100.00").

## Inventory
- Stock levels (filters: category, low, negative, expiring) with on-hand and value.
- Product history (ledger with running balance).
- Adjustment form (scan product, +/− qty, reason, batch, note).
- Batches (by expiry).
- **Stock counts:** list → count screen (scan + qty, big input, running list) → variance review → Post.

## Purchasing
Suppliers (list, detail with balance, ledger, statement) · Purchase orders (list by status, editor with "suggest from low stock", Print/PDF/Excel) · **Receive goods** (choose a PO or a supplier; grid: product, qty, cost, batch, expiry; header: supplier invoice no and total; difference warning; Save → offer labels) · Supplier returns · Payments.

## Customers
List (phone, name, tier, points, balance) · Profile (details, purchase history, points ledger, credit ledger, statement, take payment).

## Returns
Invoice lookup (scan the receipt barcode) → lines with returnable qty → qty, restock/write-off, reason → refund method → complete (prints). "Without receipt" mode (permission).

## Shifts
Open (float + optional denomination grid), cash in/out (amount, reason), X report, Close (denomination grid → expected shown after the count if blind close is on → difference → print Z). Admin: shift list with force close.

## Reports
Left: report list grouped (Sales, Profit, Stock, Tax, Customers & suppliers, Control). Right: parameters (date range presets: today, yesterday, this week, this month, custom) → generic `ReportTable` view → Print (80 mm or A4), Excel, PDF.

## Import / Export
Wizard: entity → file → sheet → column mapping → match rule → preview with errors → import with progress → summary + error file. Export: entity, format, filters.

## Users & roles
Users (tiles, role, active, last login), user form (PIN/password reset). Roles: permission checklist grouped by area, max discount %.

## Settings
Sections: Shop · Money & rounding · Tax · Receipt (live preview) · Hardware (printer, drawer, scale, label printer, each with a Test button) · Payment methods · Denominations · Checkout · Inventory · Loyalty · Credit · Barcode rules (scale labels, internal prefix, with a test field: paste a code → see how it parses) · Backups · Logs (detailed switch, export, open folder).

## Audit log
Filters (date, user, action), table, detail panel with JSON shown as a readable list, export.

## Backups
List (date, reason, size, verified ✓, sales count, last invoice), Back up now, Restore (admin, with a confirmation that names the date).
