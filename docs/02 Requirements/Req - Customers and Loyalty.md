---
tags: [requirements]
modules: [customers, pricing, sales]
---
# Requirements: Customers and Loyalty (CUS)

Module: [[Module - customers]].

### CUS-01 Customer records
- Phone (required, unique, stored normalized as digits plus an optional leading +), name, email, address, tier, credit limit, "no loyalty" flag, notes. Archive instead of delete

### CUS-02 Lookup at the till
- Typing part of a phone number or name shows matches as you type. Enter attaches the customer to the cart
- Quick create needs only phone and name

### CUS-03 Purchase history
- The customer page lists sales and returns (date, invoice, total) and opens any receipt. It also shows total spend and visit count

### CUS-04 Loyalty points
- Settings: earn N points for each spend unit (e.g. 1 point per 100.00), value per point, minimum points to redeem
- Points are earned on the amount paid **excluding** the part paid with points or customer credit
- Given 1 point per 100.00 and a sale of 1,250.00 cash, then +12 points
- Points are recorded in a ledger (earn, redeem, reverse, adjust). The balance is shown on the till and on the receipt
- Admin adjustments need `LoyaltyAdjust` and a reason

### CUS-05 Customer credit (baki)
- Sales on credit increase the customer's debt (ledger). The credit limit is checked ([[Req - Payments]] PAY-04)
- Taking a payment against debt at the till: cash goes into the shift (`CustomerReceivePayment`). In the back office: with a method and reference
- A printable statement for a date range shows opening and closing balances
- Receivables aging report: 0–30, 31–60, 61–90, 90+ days

### CUS-06 Store credit
- A refund given as store credit makes the balance negative (the shop owes the customer). Paying with "Customer credit" uses store credit first

### CUS-07 Tiers
- Tiers are defined in Pricing ([[Req - Pricing and Promotions]] PRC-03) and assigned to customers by an admin

### CUS-08 Receipt information
- With a customer attached, the receipt shows the customer's name, masked phone (`01•••••789`), points earned, points balance and credit balance (each can be switched off in the receipt layout)
