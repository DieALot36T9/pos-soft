---
tags: [ui, page-spec]
page: P30
status: not-started
---
# P30 Receive goods

**Status:** ⬜ Not started · **Area:** Buying · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Record a delivery: stock up, costs updated, supplier owed.

| | |
|---|---|
| **Who uses it** | Owner / staff |
| **How often** | Weekly |
| **Requirements** | [[Req - Purchasing]] PUR-03 |
| **Today's code** | `ReceivePanel` |

## What it should contain (starting point for the mock)
Choose order or supplier; grid of lines (qty, cost, batch, expiry); invoice no. and total with difference warning; Save → print labels.

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
`ui-mocks/src/pages/P30-…/` · route `/p30`
