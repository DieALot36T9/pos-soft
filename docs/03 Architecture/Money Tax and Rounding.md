---
tags: [architecture, money, tax]
---
# Money, Tax and Rounding

**This page is the single source of truth for every calculation that involves money.** Each formula is a function in `core`, `tax` or `pricing`, and every example below is a unit test.

## Types
| Type | Stored as | Meaning |
|---|---|---|
| `Money(i64)` | minor units | With 2 currency decimals, `12345` = 123.45. With 0 decimals, `12345` = 12345 |
| `Qty(i64)` | thousandths of the product's unit | `1000` = 1 piece / 1 kg / 1 litre. `1234` = 1.234 kg |
| `Bps(u32)` | hundredths of a percent | `1500` = 15%, `750` = 7.5% |

- All intermediate maths uses **`i128`**, so it can't overflow.
- **Rounding rule everywhere:** half away from zero (`round_half_away(numer, denom)`). 2.5 → 3, −2.5 → −3.
- Floats (`f32`/`f64`) are **forbidden** for these values (clippy lint `float_arithmetic` is denied in `core`, `tax`, `pricing`, `sales`).

## Line calculation (in order)
1. **Base amount**: `base = round(unit_price × qty_milli / 1000)`
   - Price-embedded scale labels: `base = embedded_price` exactly
2. **Promotion discount** (pricing engine, at most one promotion): `promo_discount`, 0 ≤ promo_discount ≤ base
3. **Tier discount** (only if no promotion): `tier_discount = round((base) × tier_bps / 10000)`
4. **Manual line discount**: % → `round((base − promo − tier) × bps / 10000)`, or a fixed amount. The total discount can't exceed base
5. **Invoice discount share**: the invoice discount is spread across lines in proportion to `(base − line discounts)` using the **largest remainder method** (ties go to the earliest line), so the shares add up exactly
6. **Line amount**: `amount = base − promo − tier − manual − invoice_share`
7. **Tax** (rate `r` in bps):
   - Inclusive: `tax = round(amount × r / (10000 + r))`, `net = amount − tax`, `gross = amount`
   - Exclusive: `net = amount`, `tax = round(amount × r / 10000)`, `gross = net + tax`
8. **Tax components**: each component gets `floor(tax × comp_bps / r)`. The remainder goes to the last component

## Cart totals
- `subtotal = Σ gross` (after discounts and tax)
- `tax_total = Σ tax`
- `rounding = round_to_increment(subtotal, increment, mode) − subtotal` (0 if rounding is off)
- `total_due = subtotal + rounding`
- Rounding is **not taxed**. It's its own line in the sale and the reports.

## Payments and change
- Payments are applied in the order entered. Non-cash payments are capped at the remaining amount.
- `change = Σ cash tendered − (total_due − Σ non-cash)`, only if > 0.
- When rounding is "cash only": if no cash is involved, `rounding = 0`.

## Returns
For returning `r` units of a line originally sold as `q` units:
- If `r` + already returned = `q` (the last of it): refund **exactly what remains** of each stored amount (`gross`, `tax`, `net`, `cost`).
- Otherwise: `refund_gross = round(gross × r / q)`, and the same for `tax` and `net`, each from the stored line values.
- This guarantees that the sum of all returns of a line equals the line exactly.
- Refund rounding follows the same increment rule as sales.

## Loyalty points
- `eligible = total_due − paid_by_points − paid_by_credit`
- `earned = floor(eligible / spend_unit_minor) × points_per_unit`
- `points_value = points × value_per_point_minor`

## Moving average cost (receiving)
- If on-hand ≤ 0: `new_cost = received_unit_cost`
- Else: `new_cost = round((on_hand × old_cost + received_qty × received_cost) / (on_hand + received_qty))`, with quantities in milli units. [[ADR-018 Moving average cost]]

## Worked examples (unit tests)
| # | Case | Inputs | Expected |
|---|---|---|---|
| E1 | Piece item | 90.00 × 2 | base 180.00 |
| E2 | Weighed | 120.00/kg × 1.234 kg | base 148.08 |
| E3 | Weighed, rounding | 99.99/kg × 0.333 kg | 3329.667 → base 33.30 |
| E4 | Inclusive tax | 115.00 at 15% | tax 15.00, net 100.00 |
| E5 | Inclusive tax | 100.00 at 15% | tax 13.04, net 86.96 |
| E6 | Exclusive tax | 99.99 at 7.5% | tax 7.50, gross 107.49 |
| E7 | Components | tax 13.05 on GST 18% = 9% + 9% | CGST 6.52, SGST 6.53 |
| E8 | Invoice discount split | lines 30.00, 70.00, discount 10.00 | shares 3.00, 7.00 |
| E9 | Invoice discount, remainder | lines 10.00 × 3, discount 10.00 | shares 3.34, 3.33, 3.33 |
| E10 | Cash rounding | 123.45, increment 1.00, nearest | due 123.00, rounding −0.45 |
| E11 | Cash rounding | 123.50, increment 1.00, nearest | due 124.00, rounding +0.50 |
| E12 | Change | due 470.00, cash 500.00 | change 30.00 |
| E13 | Split | due 1000.00, wallet 400.00, cash 700.00 | change 100.00 |
| E14 | Partial returns | line gross 100.00 × 3 units; return 1, then 1, then 1 | 33.33, 33.33, 33.34 |
| E15 | Moving average | 10 @ 50.00 + 20 @ 56.00 | 54.00 |
| E16 | Points | eligible 1,250.00, 1 pt / 100.00 | 12 points |

## Property tests (proptest)
- Σ line gross = subtotal; Σ line tax = tax_total
- Σ invoice-discount shares = invoice discount
- Σ refunds of a line across any return sequence = line gross
- Inclusive: net + tax = amount for every amount and rate
- Rounding is never larger than half the increment
