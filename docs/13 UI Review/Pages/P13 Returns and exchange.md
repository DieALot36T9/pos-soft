---
tags: [ui, page-spec]
page: P13
status: not-started
---
# P13 Returns & exchange

**Status:** ⬜ Not started · **Area:** Till · **Type:** Full-screen dialog · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Take items back against a receipt and refund, or exchange.

| | |
|---|---|
| **Who uses it** | Supervisor / admin |
| **How often** | Daily |
| **Requirements** | [[Req - Returns and Voids]] |
| **Today's code** | `ReturnDialog` |

## What it should contain (starting point for the mock)
Receipt lookup (scan), lines with returnable qty, restock or write-off, reason, refund method, totals, Exchange.

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
`ui-mocks/src/pages/P13-…/` · route `/p13`
