---
tags: [requirements]
modules: [shifts, reports, printing]
---
# Requirements: Shifts and Cash (SHF)

Module: [[Module - shifts]]. Workflow: [[Workflows#Open / close shift]].

### SHF-01 Open a shift
- Enter the opening float. Counting by denomination is optional
- The shift gets a business date from the cutoff rule

### SHF-02 One open shift per till
- Only one shift can be open on a till at a time, and it belongs to one cashier. To change cashier, close the shift (or hold/complete carts and use force close if needed)

### SHF-03 Cash in and out
- Cash in (e.g. extra change), paid-out (e.g. a delivery person), cash drop (to the safe), each with a reason. Needs `ShiftCashInOut`. Opens the drawer, audited

### SHF-04 No-sale drawer opening
- Needs `ShiftNoSaleDrawer`. Audited with a reason

### SHF-05 X report
- At any time, the cashier can print an X report for their own shift: the same content as the Z report, marked "X – shift still open"

### SHF-06 Close shift (Z report)
- With "count first" on, the cashier enters the count by denomination **before** seeing the expected amount
- Expected cash = float + cash sales − cash refunds + cash in − cash out + cash customer-debt payments
- Given expected 12,450.00 and counted 12,400.00, then the shortage of −50.00 is recorded and printed
- Closing is blocked while carts are open on this shift (they must be held or cancelled first)
- The Z report prints automatically

### SHF-07 Force close
- An admin can close a shift left open (crash, cashier went home). It's marked "force closed", the count is optional, and it's audited

### SHF-08 Z report content
- Shop, till, cashier, opened/closed times, business date
- Number of sales, returns, voids, cancelled carts
- Gross sales, discounts (promotions / manual), returns, net sales
- Tax by rate
- Payments by method (sales − refunds)
- Cash summary: float, cash in/out, expected, counted, difference
- Count per denomination
