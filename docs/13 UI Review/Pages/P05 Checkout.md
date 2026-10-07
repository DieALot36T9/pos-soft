---
tags: [ui, page-spec]
page: P05
status: not-started
---
# P05 Checkout

**Status:** ⬜ Not started · **Area:** Till · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Scan or find products and build the customer's order quickly.

| | |
|---|---|
| **Who uses it** | Cashier |
| **How often** | All day |
| **Requirements** | [[Req - Checkout]], [[Checkout Screen]], [[Keyboard Shortcuts]] |
| **Today's code** | `modules/checkout/pages/CheckoutPage.tsx`, `TotalsPanel` |

## What it should contain (starting point for the mock)
Scan box, category chips, product tiles, order panel with − / + lines, promotions, totals, big Pay, ≤ 5 actions + More.

## Problems in today's version
17 buttons in a wall of colours at the bottom; Pay is small and grey; no product tiles or categories (search only); large empty cart and totals areas; tooltip covers the scan box; a separate 'No shift open' bar.

## Ideas to try in the mock
- Follow the layout for this kind of page in [[UI Guidelines#3. Layouts]] and the patterns in [[UI Guidelines#4. Patterns]].
- Use the shop's demo data (same products and customers as the demo backup).
- Show every state with the Mock switcher: normal, empty, loading, error, lots of data, cashier vs owner.

## Questions for the product owner
- (Add questions here during the review.)

## Decisions (filled in during review)
| Date | Decision |
|---|---|

## Mock
`ui-mocks/src/pages/P05-…/` · route `/p05`
