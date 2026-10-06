---
tags: [ui]
---
# Checkout Screen

The screen cashiers use all day. Requirements: [[Req - Checkout]], [[Req - Payments]]. Look and feel: [[Design System]] (flat, colour-coded, tooltips everywhere).

## Layout (works from 1366×768 up)
```
┌──────────────────────────────────────────────────────────────────────────────────────┐
│ Shop name · T1 · Shift: Rahim (since 09:02)     🟦 Customer: —      🟩Printer 🟩Scale  12:41 │
├───────────────────────────────────────────────────────┬──────────────────────────────┤
│ 🔍 [ Scan or search…                              ]   │  Items              12        │
├───────────────────────────────────────────────────────┤  Subtotal      1,452.50       │
│ #  Product                     Qty      Price   Amount│  🟪 Discounts     −75.00       │
│ 1  Milk 1L                     2      90.00   180.00  │  VAT (incl.)     163.70       │
│ 2  Potato  ⚖              1.250 kg   60.00    75.00   │  Rounding         −0.50       │
│ 3  Soap 100g                   3      50.00   100.00  │ ┌──────────────────────────┐ │
│    🟪 Buy 2 get 1                            −50.00   │ │ TOTAL        1,377.00    │ │
│ 4  🟦 Rice Miniket 5kg  (selected)  1  420.00 420.00  │ └──────────────────────────┘ │
│ …                                                     │                              │
│                                                       │  [ 🟩 ✔ Pay   F10 ]          │
├───────────────────────────────────────────────────────┴──────────────────────────────┤
│ [🟦 Search F2] [🟦 Customer F3] [⬜ Qty F4] [🟨 Hold F5] [🟨 Held (2) F6]            │
│ [🟪 Discount F7] [🟪 Price F8] [🟥 Remove Del] [🟥 Cancel sale F9] [🟦 Reprint Ctrl+P] │
│ [🟥 Return Ctrl+R] [⬜ Shift F11] [⬜ Lock F12]                                          │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

## Colour at a glance
| What the cashier sees | Meaning |
|---|---|
| Big **green** Pay button, bottom-right | "Finish here" |
| **Red** buttons | Something will be removed, cancelled or refunded |
| **Amber** buttons and the amber "Held (2)" count | Paused sales waiting |
| **Purple** lines and amounts | A discount or promotion changed the price |
| **Blue** highlight on a line | The selected line (arrow keys move it) |
| **Green / red** status lights top-right | Printer and scale connected / not connected |
| **Amber** strip on a line | Low stock or near expiry warning |
| **Red** message bar under the scan box | The last scan or action failed. It says why |

## Tooltips on this screen (examples; full text lives in `checkout/text.ts`)
| Element | Tooltip |
|---|---|
| Scan box | "Scan a barcode or type a name, code or SKU, then press Enter." |
| Pay | "Take payment for this sale." `F10` |
| Hold | "Put this sale aside and serve the next customer. You can bring it back later." `F5` |
| Held (2) | "2 sales are on hold. Open the list to continue one." `F6` |
| Remove | "Remove the selected item from this sale." `Del` |
| Cancel sale | "Throw away this whole sale. Nothing is charged." `F9` |
| Discount | "Give a discount on the selected item. Above your limit, an admin must approve." `F7` |
| ⚖ icon on a line | "This item is sold by weight." |
| Printer light | "Receipt printer is connected." / "Receipt printer is not responding. Sales still save; you can reprint later." |
| Disabled Pay | "Add an item first." |

## Behaviour
- The **scan box always has focus** when no dialog is open. Clicking anywhere else gives focus back after the click.
- Search results drop down under the scan box (thumbnail, name, price, stock).
- Arrow keys ↑/↓ move the blue selection. The last added line is selected automatically.
- TOTAL is the largest text on the screen.
- All amounts come from the cart sent by Rust (`CartView`).
- As built in C-M8: the customer strip sits between the scan box and the cart (customer, tier, points, debt or store credit, price list, sale discount). The button bar also has **Sale discount** (`Shift+F7`) and **Price list** (no key; it lists the switched-on lists). A line with a typed price shows a purple "Price typed" badge. Anything above the user's rights opens the admin approval dialog and runs again once allowed.

## Payment dialog
```
┌───────────────────────── Payment ─────────────────────────┐
│  Due                                         1,377.00     │
│  ─────────────────────────────────────────────────────    │
│  🟩 Cash        [ 1,500.00 ]   Quick: 1,400  1,500  2,000 │
│  🟦 Card        [          ]   Ref [        ]             │
│  🟢 Wallet      [          ]   Ref [        ]   (teal)    │
│  🟨 Credit      [          ]   (limit 5,000, owes 0)      │
│  🟪 Points      [          ]   (1,200 pts = 120.00)       │
│  ─────────────────────────────────────────────────────    │
│  Paid 1,500.00    Remaining 0.00    CHANGE  123.00        │
│  [⬜ Back  Esc]                     [🟩 ✔ Complete  Enter] │
└───────────────────────────────────────────────────────────┘
```
- One row per active payment method, each in its own colour ([[Design System#Payment method colours (defaults; the admin can change them from the same palette)]]). ↑/↓ moves between rows; typing replaces the amount.
- `Enter` completes when nothing remains to pay. CHANGE is shown in big green numbers and stays on screen until the next scan.

## Weight dialog
Shows the live weight with a 🟩 "Stable" badge when the scale is steady, and price × weight = amount. `Enter` accepts (only when stable and above zero). `M` switches to typing the weight (if allowed).

## Admin approval dialog
Amber header: "This needs an admin: <action>". Fields: admin (choose), password. `Enter` approves, and the original action runs automatically.

## Feedback
- Good scan: a short beep, and the new line's background flashes for 0.3 s.
- Problem: a different sound, plus a red message bar under the scan box for 3 s (or until the next key press).

## States
| State | What happens |
|---|---|
| No open shift | An overlay says "Open a shift to start selling" with a green **Open shift** button |
| Cart restored after a crash | Blue info bar: "Your previous sale was restored" |
| Printer failed after a sale | Amber bar with **Retry** and **Print on another printer** |
| Locked | Full-screen lock with the user's tile; the cart is hidden |
