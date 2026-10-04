---
tags: [requirements]
modules: [settings, tax, auth, db]
---
# Requirements: Setup and Settings (SET)

Screens: [[Setup Wizard]]. Modules: [[Module - settings]], [[Module - tax]].

### SET-01 First-run detection
As a new shop owner, I want setup to start automatically, so that the till can't be used half-configured.
- Given a fresh install with no database
- When the app opens
- Then the setup wizard is shown, and no other screen can be reached until the required steps are done

### SET-02 Resume an interrupted setup
- Given the wizard was closed after step 3
- When the app reopens
- Then the wizard opens at step 4 with steps 1–3 filled in as saved

### SET-03 Language
- v1.0 is English only, so the wizard has no language step
- All screen and receipt text is kept in one place per module, so other languages can be added later without code changes ([[ADR-022 Ready for more languages later]])

### SET-04 Shop profile
- Fields: shop name (required), address lines, phone, email, tax registration number (label configurable, e.g. "BIN", "VAT No", "GSTIN"), logo, receipt footer lines
- Given I upload a logo (PNG/JPG, ≤ 2 MB)
- Then it's stored, and a black-and-white preview at receipt width is shown

### SET-05 Currency and number format
- Fields: currency code (ISO 4217, e.g. BDT), symbol (e.g. ৳), symbol before/after, **minor digits (0–3)**, decimal separator, thousands grouping (Western 1,234,567 or Indian 12,34,567)
- Given the settings `৳`, before, 2 digits, Indian grouping
- Then 1234567.5 is shown as `৳12,34,567.50`
- Given at least one sale exists
- When an admin tries to change the minor digits
- Then it's refused with an explanation (stored amounts would change meaning)

### SET-06 Cash rounding
- Fields: increment in minor units (0 = off), mode (nearest / down / up), applies to (all payments / cash-only payments)
- Given increment 100 (one whole unit), mode nearest, subtotal 123.45
- Then the amount due is 123.00 and a rounding line of −0.45 is shown and printed
- Given subtotal 123.50, then the amount due is 124.00 (+0.50)

### SET-07 Tax setup
- Fields: tax label (e.g. "VAT", "GST", "Sales Tax"), price mode (prices **include** / **exclude** tax), one or more named rates, default rate for new products
- Details in [[Req - Tax]]

### SET-08 First admin account
- Given the admin step
- When I enter a username, display name and password (≥ 8 characters, typed twice)
- Then the admin is created, and a **recovery code** is shown once with "Print" and "I have saved it" options ([[Req - Users and Security]] USR-09)

### SET-09 Till identity and business day
- Fields: till code (default `T1`, 1–6 letters/digits), business-day cutoff hour (default 04:00)
- Given cutoff 04:00
- When a sale completes at 01:30 on 5 March
- Then its business date is 4 March

### SET-10 Hardware (optional step)
- Choose a receipt printer, paper width (58/80 mm), drawer kick pin, scale port and profile, label printer
- "Test print", "Open drawer" and "Read scale" buttons give immediate results
- The step can be skipped and finished later

### SET-11 Backups
- Primary folder (default: app data `backups/`), optional second folder (USB/other disk), interval (default 60 min), copies to keep (default 50)
- The folder must be writable, otherwise the step shows an error

### SET-12 Optional product import
- The last wizard step offers: "Import products from CSV/Excel", "Add products later" or "Load demo data" (demo data is marked and can be removed in one action before the first real sale)

### SET-13 Editing settings later
- Given an admin opens Settings
- Then every wizard value can be edited (subject to SET-05). Each change is written to the audit log with old and new values

### SET-14 Receipt layout
- Options: show logo, header lines, footer lines, show tax breakdown, show tax registration no., show the invoice barcode (for returns), show customer points/balance, paper width, number of copies (1–3)
- A live preview renders the real raster image

### SET-15 Payment methods
- Admin manages a list: name, kind (`cash` / `card` / `wallet` / `other`), needs a reference (yes/no), active, order
- System methods `Customer credit` and `Loyalty points` can't be deleted. They're hidden if those features are off

### SET-16 Cash denominations
- Admin lists the notes and coins used for counting (e.g. 1000, 500, 200, 100, 50, 20, 10, 5, 2, 1). These are pre-filled from the currency code where known

### SET-17 Feature settings
- Inventory: allow negative stock (default yes), near-expiry days (default 30), selling expired batches (warn/block), cost method (moving average / last cost)
- Checkout: merge repeated scans into one line (default yes), lock after idle minutes (default 5), count cash before seeing the expected amount at close (default yes)
- Loyalty: on/off, points per spend unit, value per point, minimum points to redeem
- Credit: on/off, default credit limit
