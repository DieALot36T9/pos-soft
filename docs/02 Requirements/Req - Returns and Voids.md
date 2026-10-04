---
tags: [requirements]
modules: [sales, inventory, customers, shifts]
---
# Requirements: Returns and Voids (RET)

Modules: [[Module - sales]], [[Module - inventory]], [[Module - customers]]. Workflows: [[Workflows#Return items]], [[Workflows#Void sale]].

### RET-01 Return with receipt
- Scan the invoice barcode on the receipt, or type the invoice number
- The original lines are listed with the quantity still returnable (sold − already returned)
- Choose lines and quantities, a reason, and for each line **Restock** or **Write off (damaged)**
- Refund method defaults to the original payment method. Cash or store credit can be chosen instead
- Given a line sold × 3 with 1 already returned, when trying to return 3, then it's refused (max 2)

### RET-02 Exact refund amounts
- The refund for a partial return is calculated **pro rata from the stored line amounts**, including discounts and tax. The return of the **final** remaining quantity refunds exactly what's left, so totals always match. See [[Money Tax and Rounding#Returns]]

### RET-03 Return without receipt
- Needs `ReturnWithoutReceipt`. Pick products. The price is the current price, and it can be edited only with `PriceOverride`

### RET-04 Void a completed sale
- Needs `SaleVoid` and a reason. Allowed only while **the shift that made the sale is still open**. After that, a return must be used
- The void reverses stock, shift payments, points and credit. The sale is marked VOID but keeps its number and contents
- A void receipt is printed

### RET-05 Exchange
- Start a return, then "Exchange" moves straight to a new cart. The refund becomes a credit on the new sale, and only the difference is paid or refunded
- Saved as one return and one sale, linked to each other

### RET-06 Return receipts
- Heading "RETURN", return number (`T1-R000045`), original invoice number, lines, refund method and amount

### RET-07 Reversals
- Points earned on the returned amount are taken back (the balance may go negative, which blocks redeeming until it's positive again)
- A return of a credit sale reduces the customer's debt first. Anything beyond that is refunded by the chosen method
