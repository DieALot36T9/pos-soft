---
tags: [requirements]
modules: [purchasing, inventory, catalog]
---
# Requirements: Purchasing (PUR)

Module: [[Module - purchasing]]. Workflows: [[Workflows#Receive goods (GRN)]], [[Workflows#Supplier return]].

### PUR-01 Suppliers
- Fields: name (required), contact person, phone, email, address, tax number, payment terms (days), notes. Archive instead of delete

### PUR-02 Purchase orders
- Statuses: Draft → Sent → Partially received → Received, or Cancelled
- Lines: product, quantity, expected unit cost
- "Suggest from low stock": fills lines with low-stock products whose preferred supplier is this supplier, quantity = reorder quantity
- "Send" means printing, saving as PDF or exporting to Excel (offline), which marks it Sent
- PO numbers: `PO-000001`

### PUR-03 Receive goods (GRN)
- From a PO (lines pre-filled with remaining quantities) or directly, without a PO
- For each line: quantity received, unit cost, batch number and expiry (needed if the product tracks expiry)
- Header: supplier invoice number, invoice total. A difference from the line total is shown as a warning
- On save, in one transaction: stock is added (ledger `receive`), product cost is updated by the cost method, the supplier's balance goes up by the invoice total, and the PO's received quantities and status are updated
- Cost update (moving average) example:
  - Given stock 10 at cost 50.00, when 20 are received at 56.00
  - Then the new cost is (10×50 + 20×56) ÷ 30 = **54.00**
  - If stock on hand is ≤ 0, the new cost is the received cost
- "Print labels for this delivery" opens label printing with received quantities ([[Req - Catalog]] CAT-12)
- GRN numbers: `GRN-000001`. A GRN note can be printed

### PUR-04 Supplier returns
- Pick a supplier (optionally the original GRN), then products, quantities and batches, with a reason
- Stock goes down (ledger `supplier_return`). The supplier balance goes down by the return value (at the cost entered, default current cost)
- Numbers: `SR-000001`

### PUR-05 Payables
- Each supplier has a balance = invoices − payments − returns ± adjustments, from an append-only ledger
- Record a payment: amount, method, reference, date. A cash payment from the drawer during an open shift records cash-out on the shift
- Aging report: current, 1–30, 31–60, 61–90, 90+ days past the payment terms

### PUR-06 Supplier statement
- Printable or exportable ledger for a date range with opening and closing balances
