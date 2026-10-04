---
tags: [requirements]
modules: [inventory]
---
# Requirements: Inventory (INV)

Module: [[Module - inventory]]. Design: [[ADR-010 Append-only ledgers]].

### INV-01 Live stock with full history
- Every stock change writes one or more **ledger** rows: product, batch, quantity change, kind (sale, void, return, receive, supplier return, adjust, count, opening), reference document, unit cost, time, business date, user
- Given stock 10, when 3 are sold, then stock is 7 and the ledger has a −3 `sale` row pointing at the invoice

### INV-02 Adjustments
- Admin chooses a product (scan/search), quantity in or out, reason (damage, expiry, theft, count error, opening stock, internal use, other + note), and batch if tracked
- Reasons are configurable. Built-in ones can't be deleted

### INV-03 Reorder levels and low-stock alerts
- Each product can have a reorder level and a reorder quantity
- Given reorder level 10 and stock falls to 9, then the product appears in the Low-stock list and the back-office badge count goes up
- Low-stock alerts appear on the dashboard at login for users with `StockView`

### INV-04 Negative stock
- With "allow negative stock" on (default), sales never block. The product shows negative stock in red in reports
- With it off, adding more than is available to the cart is refused, with the available quantity shown

### INV-05 Batches and expiry
- Products with "track expiry" need an expiry date (and optionally a batch number) when stock is received or adjusted in
- Sales take stock **first expiry, first out** automatically, possibly from several batches in one line
- Near-expiry list: batches expiring within N days (setting). Shown as a dashboard alert
- Selling from an expired batch: warn or block (setting)

### INV-06 Stock counts
- Create a count: full, or partial (by category, brand, location note or product list)
- Counting: scan items and type quantities. The same product can be scanned several times (quantities add up)
- The expected quantity is captured **when each item is counted**, so sales during the count don't create false differences
- The variance report lists expected, counted, difference and cost value of the difference
- Posting creates `count` ledger rows for every difference. Products in a full count that were never counted are treated as 0 after a confirmation

### INV-07 Stock history
- The product page shows ledger rows with a running balance, filterable by date and kind, linking to the source document

### INV-08 Stock value
- Value = on-hand × current cost price. By product, category, brand ([[Req - Reports]])

### INV-09 Kits and non-stock products
- Kits never have stock rows. Their components do. Products with "track stock" off are never in the ledger

### INV-10 Self-check
- An admin "Verify stock data" action (and an automatic check in M13 tests) confirms that each level equals the sum of its ledger. Differences are reported, never fixed silently
