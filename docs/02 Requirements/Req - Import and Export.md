---
tags: [requirements]
modules: [io, app]
---
# Requirements: Import and Export (IO)

Module: [[Module - io]]. Workflow: [[Workflows#Import products]].

### IO-01 Product import
1. **Template**: "Download template" gives an .xlsx/.csv with the right headers and an example row
2. **Choose file**: CSV (UTF-8, with or without BOM; comma or semicolon) or XLSX (first sheet or a chosen sheet)
3. **Map columns**: headers are matched automatically by name, including common alternatives (e.g. "Price", "Sale price", "MRP"). The user can fix the mapping
4. **Match rule**: update existing products by SKU (default) or by barcode. Create the rest
5. **Preview**: the first 50 rows as they'll be saved, plus **every error with row number and column** (e.g. "Row 812, Sale price: '12,5O' is not a number")
6. **Import**: only allowed when there are 0 errors (or the user chooses "skip rows with errors"). A **backup is taken first**, then everything is saved in one transaction, so it all succeeds or nothing changes
7. **Summary**: created, updated, skipped, plus a downloadable error file
- Missing categories, brands and units can be created automatically (an option on the mapping screen)

### IO-02 Other imports
- Same flow for: customers, suppliers, opening stock (as `opening` adjustments with optional batch/expiry), price-list prices, barcodes (extra barcodes for existing products)

### IO-03 Exports
- Products (with all barcodes in one column separated by `|`), customers with balances, suppliers with balances, stock levels, price lists, batches
- Formats: CSV (UTF-8 with BOM, so Excel shows symbols like ৳ correctly) and XLSX
- An export can always be imported back without changes (round trip, tested)

### IO-04 Numbers and money in files
- Money is written with the currency's decimals and a `.` decimal point (e.g. `12.50`). On import, more decimals than the currency allows is an error, never silently rounded
- Quantities use up to 3 decimals
- Booleans accept `yes/no`, `true/false`, `1/0`

### IO-05 Size
- 50,000 rows import in ≤ 60 s with a progress bar and a cancel button (cancel = nothing saved)
