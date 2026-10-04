---
tags: [requirements]
modules: [sales, shifts, customers]
---
# Requirements: Payments (PAY)

Modules: [[Module - sales]], [[Module - shifts]], [[Module - customers]]. Workflow: [[Workflows#Complete sale]].

### PAY-01 Cash and change
- `F10` (or `Enter` on an empty input) opens payment. The cash amount defaults to the amount due
- Quick buttons show the next useful notes (e.g. due 470 → 500, 1000)
- Given due 470.00 and tendered 500.00, then change due is **30.00**, shown large

### PAY-02 Other methods
- Card, wallet and other methods are recorded with an optional or required reference (e.g. transaction ID) according to the method's settings. **No card numbers are stored**

### PAY-03 Split payment
- Given due 1,000.00
- When 400.00 by wallet and 700.00 cash are entered
- Then change is 100.00
- Only **cash** may go over the remaining amount. Non-cash payments are capped at the remaining amount
- The sale completes only when the amount paid is at least the amount due

### PAY-04 Customer credit (baki)
- Needs an attached customer and `SellOnCredit`
- Given a credit limit of 5,000.00 and a balance of 4,500.00, when 800.00 goes on credit, then the override dialog appears (`CreditLimitExceed`)

### PAY-05 Loyalty points
- Given the customer has 1,200 points, the value per point is 0.10 and the minimum to redeem is 100
- When paying with points
- Then up to 120.00 can be paid with points, in whole points

### PAY-06 Rounding
- Rounding follows [[Req - Setup and Settings]] SET-06. If it's set to "cash only", rounding is only applied when the remaining amount is paid in cash

### PAY-07 Atomic completion
- When the sale completes, **in one transaction**: the sale, lines and payments are saved, the invoice number is assigned, stock is deducted (FEFO for batches, components for kits), shift payments are recorded, and customer points and credit are updated
- If any step fails, nothing is saved, the cart stays as it was, and the error is shown
- After the commit: the receipt prints, and the drawer opens if any cash was involved

### PAY-08 Invoice numbers
- The format is `{till_code}-{6+ digit number}`, e.g. `T1-000123`. Numbers are **gapless and increasing for each till**. Voided sales keep their number

### PAY-09 Printing failures
- Given the printer is offline
- When a sale completes
- Then the sale is saved, and a banner says "Receipt not printed" with **Retry** and **Print to system printer**

### PAY-10 Reprint
- `Ctrl+P` reprints the last receipt of this till (anyone)
- Reprinting any older receipt needs `ReceiptReprintAny`
- Reprints are marked **"COPY"**
