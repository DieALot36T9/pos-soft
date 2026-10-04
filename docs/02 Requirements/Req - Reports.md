---
tags: [requirements]
modules: [reports, io]
---
# Requirements: Reports (RPT)

Module: [[Module - reports]]. Every report returns the same `ReportTable` shape, so every report can be shown on screen, printed, and exported to Excel and PDF the same way.

### RPT-01 Daily summary
- For a business date (and optionally one till): number of sales, gross sales, promotion discounts, manual discounts, returns, net sales, tax by rate, rounding total, payments by method (sales − refunds), voids (count and value), cancelled carts (count), average basket, items sold
- Printable on the receipt printer and on A4

### RPT-02 Shift reports
- X and Z reports as in [[Req - Shifts and Cash]] SHF-08. Past Z reports can be reprinted

### RPT-03 Sales analysis
- Sales by product, category, brand, cashier, hour of day, day of week and date, for any date range
- Columns: quantity, net sales, tax, discount, number of sales, % of total
- Sorting, top N, and filtering by category/brand/cashier

### RPT-04 Profit and margin
- Using the **cost saved on each sale line** (cost at the moment of sale): net sales, cost, gross profit, margin %, by product, category or brand
- Returns subtract their own stored cost

### RPT-05 Stock reports
- Stock value (on-hand × cost) by product, category, brand
- Low stock (with supplier and suggested reorder quantity)
- Near expiry and expired batches
- Stock movement summary for a period: opening, received, sold, returned, adjusted, closing

### RPT-06 Tax report
- [[Req - Tax]] TAX-09

### RPT-07 Payables and receivables
- Supplier aging and customer aging. Supplier and customer statements

### RPT-08 Discounts and overrides
- Per cashier: manual discounts, price overrides, voids, cancelled carts, no-sale drawer openings, overrides approved (and by whom). Helps detect abuse

### RPT-09 Export
- Every report exports to **Excel (.xlsx)** with real numbers (not text) and a header with the shop name, report name, date range and generation time
- Every report exports to **PDF** through the system print dialog's "Save as PDF" (A4 layout)

### RPT-10 Visibility
- Cashiers see only X/Z reports for their own shifts. Everything else needs `ReportsViewAll`

### RPT-11 Speed
- Any report over 1 year of reference data in ≤ 3 s ([[Non-Functional Requirements]])
