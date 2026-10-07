---
tags: [ui, page-spec]
page: P07
status: not-started
---
# P07 Weight entry

**Status:** ⬜ Not started · **Area:** Till · **Type:** Dialog · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Weigh a product with the scale or type its weight.

| | |
|---|---|
| **Who uses it** | Cashier |
| **How often** | Often (fresh items) |
| **Requirements** | [[Req - Checkout]] CHK-06, CHK-07 |
| **Today's code** | `WeightDialog` |

## What it should contain (starting point for the mock)
Live weight, Stable badge, price × weight = amount, manual entry, Accept.

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
`ui-mocks/src/pages/P07-…/` · route `/p07`
