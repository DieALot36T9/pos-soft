---
tags: [ui, page-spec]
page: P06
status: not-started
---
# P06 Line panel: quantity, discount, price

**Status:** ⬜ Not started · **Area:** Till · **Type:** Dialog · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Change one cart line: quantity, discount or price, with a numpad.

| | |
|---|---|
| **Who uses it** | Cashier / supervisor |
| **How often** | Often |
| **Requirements** | [[Req - Checkout]] CHK-04, CHK-12, CHK-13 |
| **Today's code** | `QtyDialog`, `DiscountDialog`, `PriceDialog` |

## What it should contain (starting point for the mock)
Line summary, numpad, tabs or buttons for Qty / Discount / Price / Remove, limit hint, approval when over the limit.

## Problems in today's version
Not reviewed yet. Note them when this page's review starts.

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
`ui-mocks/src/pages/P06-…/` · route `/p06`
