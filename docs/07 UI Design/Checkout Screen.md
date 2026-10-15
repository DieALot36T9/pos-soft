---
tags: [ui]
---
# Checkout Screen

The screen cashiers use all day. Requirements: [[Req - Checkout]], [[Req - Payments]]. Look and feel: [[Design System]] (flat, colour-coded, self-explaining).

## 60/40 Split POS Layout (`CheckoutPage.tsx`)

```text
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│ 🟩 Till  Shift: Rahim (since 09:02)                  🕒 Business Day: 2026-10-15 [Help F1]│
├───────────────────────────────────────────────────┬──────────────────────────────────────┤
│ 🔍 [ Scan or search…                          ]   │ 🟦 Rahim · +88017…  🟪 Gold  🟪 120pt│
├───────────────────────────────────────────────────┤──────────────────────────────────────┤
│ [All Product Tiles] [⚖ Weighed / Fresh] [Low] [🔍]│ #  Product            Qty  Price  Amt│
├───────────────────────────────────────────────────┤ 1  Milk 1L              2  90.00  180│
│ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐│ 2  Potato ⚖         1.250  60.00   75│
│ │ Milk 1L      │ │ Potato ⚖     │ │ Soap 100g    ││ 3  Soap 100g            3  50.00  100│
│ │ P000001      │ │ P000002      │ │ P000003      ││    🟪 Buy 2 get 1             −50.00│
│ │ ৳90.00  24pc │ │ ৳60.00  18kg │ │ ৳50.00   4pc ││ 4  🟦 Rice 5kg [Numpad] 1 420.00  420│
│ └──────────────┘ └──────────────┘ └──────────────┘│                                      │
│                                                   ├──────────────────────────────────────┤
├───────────────────────────────────────────────────┤ Items 7 · Subtotal 825.00 · VAT 95.00│
│ [🟦 Customer F3] [🟨 Hold F5] [🟨 Held (2) F6]    │ ┌──────────────────────────────────┐ │
│ [🟪 Discount F7] [🟥 Return Ctrl+R]   [More ▲/▼]  │ │ TOTAL                 ৳775.00    │ │
│ ───────────────────────────────────────────────── │ └──────────────────────────────────┘ │
│ Secondary row (when More is open): Search F2,     │ [        🟩 ✔ Pay   F10            ] │
│ Qty F4, Sale discount Shift+F7, Price F8,         │                                      │
│ Price list, Remove Del, Cancel F9, Reprint Ctrl+P,│                                      │
│ Shift F11, Lock F12, Back office                  │                                      │
└───────────────────────────────────────────────────┴──────────────────────────────────────┘
```

## Key Components in `ui/src/modules/checkout/`
1. **Left 60% — `ProductCatalogPane.tsx`:**
   - **Top:** `ScanBox.tsx` with persistent focus when no dialog/drawer is open, 80 ms search-as-you-type dropdown, `+`/`-` quantity stepping, and `n*code` multiplier support.
   - **Middle:** Category filter pills (`All Product Tiles`, `⚖ Weighed / Fresh`, `Low Stock`) and a local tile filter input, rendering 96 px+ touch-friendly product tiles with SKU, price, weighed badge (`⚖`), and colour-coded on-hand stock badge.
   - **Bottom:** `ButtonBar.tsx` with the 5 primary actions (`Customer F3`, `Hold F5`, `Held F6`, `Discount F7`, `Return Ctrl+R`) and a `More` toggle for secondary till actions.
2. **Right 40% (min 420 px) — Order & Totals Column:**
   - **Top strip:** `CustomerBar.tsx` (customer name/phone, tier badge, loyalty points, credit/baki or store credit, active price list, whole-sale discount), plus `ExchangeBar.tsx`, `SaleDoneBar.tsx`, `RefundDoneBar.tsx`, and `PrintProblem.tsx` banners.
   - **Center:** `CartTable.tsx` showing line numbers, badges (`⚖`, `Sticker`, `Price typed`, `Price changed`, `Low stock`, `Expired stock`), purple promotion/discount rows, and right-aligned `tabular-nums` amounts. Double-clicking any row (or clicking the `Numpad` button on the selected line) opens `LineEditDrawer.tsx` (`P06`), which provides a unified touch numpad for Quantity (`F4`), Discount (`F7`), Price Override (`F8`), and Remove (`Del`).
   - **Bottom:** `TotalsPanel.tsx` displaying Items, Subtotal, purple Discounts, Tax, Rounding, a bordered `56px` **TOTAL** box, and a full-width `72px`-tall green **Pay (`F10`)** button.
