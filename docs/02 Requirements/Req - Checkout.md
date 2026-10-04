---
tags: [requirements]
modules: [sales, catalog, barcode, pricing, hardware]
---
# Requirements: Checkout (CHK)

Screen: [[Checkout Screen]]. Modules: [[Module - sales]], [[Module - catalog]], [[Module - barcode]], [[Module - pricing]].

### CHK-01 Scan a barcode
As a cashier, I want a scan to add the item instantly, so that the queue keeps moving.
- Given product "Milk 1L", barcode `8901234567890`, price 90.00
- When the scanner types `8901234567890⏎` into the checkout input
- Then a line "Milk 1L × 1 = 90.00" appears within 100 ms, and the input is cleared and focused
- When the same code is scanned again and "merge repeated scans" is on
- Then the same line becomes × 2

### CHK-02 Unknown barcode
- Given no product has code `123`
- When `123⏎` is entered
- Then an error sound plays, "No product with code 123" is shown, and the cart is unchanged
- If the user has `CatalogManage`, a "Create product with this barcode" button is offered

### CHK-03 Search
- Given the input contains text that isn't an exact barcode/SKU
- When the user types 2+ characters
- Then matching products (name, SKU, barcode, category, brand) appear within 150 ms, with thumbnails if they have images
- Arrow keys move, Enter adds, Esc closes
- Archived products never appear

### CHK-04 Quantity
- `F4` or clicking the quantity opens quantity entry for the selected line
- Typing `3*` before scanning adds 3 at once
- Given a unit with 0 decimals (piece), when entering 1.5, then it's refused
- Given quantity 0, then the line is removed

### CHK-05 Remove a line, cancel the sale
- `Del` removes the selected line
- `F9` cancels the whole cart after confirmation. The cancelled cart and its lines are written to the audit log. No invoice number is used

### CHK-06 Weighed item: manual weight
- Given product "Potato" is weighed, unit kg, price 60.00/kg, and no scale is configured (or the user chooses manual)
- When it's scanned or picked
- Then a weight dialog asks for the weight (accepts `1.25` kg or `1250 g`)
- Then the line is "Potato 1.250 kg × 60.00 = 75.00"

### CHK-07 Weighed item: connected scale
- Given a scale is configured
- When a weighed product is added
- Then the dialog shows the live weight and a "stable" indicator. Enter accepts only a **stable, non-zero** weight
- If the scale doesn't answer within 2 s, the dialog says so and offers manual entry (needs `CheckoutManualWeight`)

### CHK-08 Scale-label barcodes
Rules are configurable ([[Module - barcode]]). Defaults match Odoo's nomenclature.
- Given the weight rule `21.....{NNDDD}` and product "Beef" with scale item code `00123`, price 750.00/kg
- When `2100123012343` is scanned (weight 01.234 kg; last digit is the EAN check digit)
- Then a line "Beef 1.234 kg = 925.50" is added
- Given the price rule `23.....{NNNDD}` and product "Cheese" with scale code `00456`, price 1,200.00/kg
- When a label with price 150.00 is scanned
- Then the line amount is **exactly 150.00**, and the quantity is 150.00 ÷ 1,200.00 = 0.125 kg (used for stock)
- Given a label with a wrong check digit, then it's rejected like an unknown code

### CHK-09 Hold a sale
- `F5` holds the cart with an optional note ("blue shirt, forgot rice"). The screen clears for the next customer
- Held carts survive restarts and power cuts

### CHK-10 Resume a held sale
- `F6` lists held carts (time, cashier, item count, total, note). Enter resumes
- Prices and promotions are **recalculated** on resume. If a price changed, the line shows a "price changed" badge
- Held carts on this till can be resumed by any cashier with an open shift

### CHK-11 Attach a customer
- `F3` opens a phone lookup (partial match). Enter attaches. "New customer" creates one quickly (phone + name)
- Attaching applies the customer's tier price list/discount, and the receipt shows points and balance

### CHK-12 Discounts
- `F7` on a line: % or amount discount. `Shift+F7`: discount on the whole invoice
- Given the cashier role's limit is 5%
- When a 10% discount is entered
- Then the override dialog appears ([[Req - Users and Security]] USR-05)
- A discount can't make a line negative

### CHK-13 Price override
- `F8` changes a line's unit price (needs `PriceOverride`). The original price is kept for the audit log and reports

### CHK-14 Kits
- Given kit "Eid Box" made of 2 × Sugar 1kg + 1 × Oil 1L
- When the kit is sold
- Then one line "Eid Box" appears on the receipt, and stock decreases by 2 sugar and 1 oil

### CHK-15 Variants
- A parent product (e.g. "Shampoo") can't be sold itself. Search lists its variants ("Shampoo 200 ml Mango"). Scanning a variant's barcode adds that variant

### CHK-16 Promotions show automatically
- Given the promotion "Buy 2 get 1 free" on Soap (price 50.00)
- When 3 soaps are in the cart
- Then a line discount of −50.00 labelled "Buy 2 get 1" shows on the soap line. The receipt shows the saving

### CHK-17 Price list switch
- With `PriceListSwitch`, the cashier can switch the cart to another price list (e.g. Wholesale). Every line is repriced

### CHK-18 Cart totals
- The cart always shows: item count, subtotal, discounts, tax (with its label), rounding, **total due** (large)
- All of these come from Rust. The UI never calculates them

### CHK-19 Crash recovery
- Given a cart with 5 lines, when the power fails and the app restarts and the same cashier logs in
- Then the cart is back exactly as it was

### CHK-20 An open shift is required
- Given no open shift for this till
- When a cashier opens Checkout
- Then they're asked to open a shift first ([[Req - Shifts and Cash]])

### CHK-21 Stock warnings
- Adding an item with stock ≤ 0 shows a non-blocking warning (or blocks if "allow negative stock" is off)
- Adding an item from an expired batch warns or blocks according to the setting

### CHK-22 Sound and visual feedback
- A good scan gives a short beep and briefly highlights the line. Errors give a distinct sound and a red message. Sounds can be switched off
