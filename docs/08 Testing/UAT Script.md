---
tags: [testing, uat]
---
# UAT Script: one simulated trading day

**Who:** the product owner (as admin) and one real cashier. **Where:** the real Windows till PC with the real printer, drawer, scanner and scale. **Data:** `uat` dataset + barcode test sheet.
Record each step: ✅ pass / ❌ fail (write a bug report using the template).

## Morning
1. Fresh install from the installer. Run the setup wizard: BDT, rounding to 1 taka, VAT 15% inclusive + 0% exempt, admin, till T1, hardware tests, backups to a USB drive. Import `uat` products from Excel.
2. Admin creates the cashier user (PIN) and a "Supervisor" role with discount up to 10%.
3. Cashier logs in and opens a shift with a 2,000.00 float counted by notes.

## Selling (at least 40 sales)
4. Scan 10 different items quickly. Every scan appears instantly; repeated scans merge.
5. Weigh potatoes on the scale; weigh onions manually.
6. Scan a scale-printed label (weight) and a price label.
7. Search a product by part of its name and by part of the SKU.
8. Change a quantity, remove a line, cancel a whole sale.
9. Hold a sale, serve another customer, resume the held sale.
10. Pay with exact cash, with change, split cash + wallet, by card with a reference.
11. Attach a customer by phone, earn points, and on a later sale pay partly with points.
12. Sell on credit to a customer; try to exceed the limit → override by admin.
13. A 5% discount by the cashier → override needed (cashier limit 0%). The Supervisor does 8% without an override.
14. A buy-2-get-1 promotion applies automatically and shows on the receipt.
15. Sell a kit; check the component stock went down.
16. Unplug the printer, complete a sale, reconnect, Retry print.
17. **Pull the power cable** in the middle of a sale with 5 items. Restart: the cart is back. Complete it.

## Afternoon
18. Return 1 item from an earlier receipt by scanning the receipt barcode (restock). Return a damaged item (write-off).
19. Exchange: return a shirt-size item and take another.
20. Admin voids a sale from this shift.
21. Admin receives a delivery (GRN) with batches and expiry dates; prints labels for it; checks the new average cost.
22. Admin does a partial stock count of one category with 2 deliberate differences and posts it.
23. Cash drop to the safe; paid-out for a delivery person.
24. Customer pays part of their baki at the till.

## Evening
25. X report, then close the shift with a counted shortage of 20.00. The Z report prints and is correct.
26. Daily summary, sales by hour, profit by category, stock value, tax report; export one to Excel and one to PDF.
27. Check the audit log shows every override, the void, the cancel and the cash movements.
28. Close the app → an on-close backup appears on both drives. Restore yesterday's backup on a copy of the PC, then restore back.
29. Hover over every button on the checkout and payment screens: each shows a clear tooltip with its shortcut key.
30. Ask the cashier what green, red, amber, blue and purple mean on the till. They should know from using it, without reading the manual.

## Pass criteria
Zero data-loss bugs, zero blocking bugs, every receipt readable, every total checked by hand on at least 5 receipts and the Z report.
