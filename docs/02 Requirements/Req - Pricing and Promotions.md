---
tags: [requirements]
modules: [pricing, customers]
---
# Requirements: Pricing and Promotions (PRC)

Module: [[Module - pricing]]. Calculation rules: [[Money Tax and Rounding]].

### PRC-01 Retail price
- With no price list, tier or promotion, a line's unit price is the product's sale price

### PRC-02 Price lists
- Admins create named price lists (e.g. Wholesale, Member) and set prices for some products
- A product without a price in the active list falls back to retail
- Prices can be entered one by one or imported ([[Req - Import and Export]])

### PRC-03 Tiers (membership)
- A tier has a name, an optional price list and an optional discount % (applied to non-promotion lines)
- Customers are assigned a tier manually by an admin
- Given tier "Gold" = price list Member + 2% discount, when a Gold customer is attached, then Member prices apply and lines without a promotion get 2% off

### PRC-04 Promotion types
| Kind | Settings | Example |
|---|---|---|
| Percent off | percent | 10% off all Detergents |
| Amount off | amount per unit | 5.00 off each Biscuit |
| Fixed price | price per unit | Oil 1L for 180.00 |
| Buy X get Y | buy qty, free qty | Buy 2 Soap get 1 free (3-for-2) |
| Multi-buy | bundle qty, bundle price | 3 Noodles for 100.00 |

- Targets: specific products, categories (including subcategories) or brands
- Limits: start/end date-time, days of the week, tier only (optional), active switch

### PRC-05 How promotions are applied
- Calculated live in the cart, on the **combined quantity of the same product** in the cart
- **One promotion per line, the one that saves the customer the most**. No stacking ([[ADR-019 Single best promotion per line]])
- Tier discount % applies only to lines without a promotion
- Manual discounts apply after promotions

### PRC-06 Worked examples (each one is a unit test)
| Case | Input | Expected |
|---|---|---|
| Buy 2 get 1 | Soap 50.00 × 7 | Free = ⌊7 ÷ 3⌋ = 2 → discount 100.00, line 250.00 |
| Multi-buy | Noodles 40.00 × 7, "3 for 100.00" | 2 bundles = 200.00 + 1 × 40.00 = 240.00 → discount 40.00 |
| Percent off | Detergent 250.00 × 2, 10% | discount 50.00 |
| Fixed price above retail | Retail 170.00, fixed 180.00 | Promotion ignored (never raises the price) |
| Best of two | 10% off category and buy 2 get 1, Soap 50.00 × 3 | Buy 2 get 1 (50.00) beats 10% (15.00) |
| Weighed + percent | Beef 750.00/kg × 1.234 kg, 10% | base 925.50, discount 92.55 |
| Expired | Promotion ended yesterday | Not applied |

### PRC-07 Manual discount limits
- Each role has a maximum manual discount % (Admin: unlimited). Invoice discounts count against the same limit, measured as a % of the subtotal

### PRC-08 Display
- Receipt lines show the original price and the saving ("Buy 2 get 1 −50.00"). The receipt footer shows "You saved 75.00 today"
