---
tags: [ui, page-spec]
page: P25
status: not-started
---
# P25 Stock levels

**Status:** ⬜ Not started · **Area:** Stock · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
See what's in stock and what's running out.

| | |
|---|---|
| **Who uses it** | Owner / manager |
| **How often** | Daily |
| **Requirements** | [[Req - Inventory]] INV-03, INV-08 |
| **Today's code** | `StockLevelsPage` |

## What it should contain (starting point for the mock)
Table with on hand, reorder level, value; chips: low, negative, expiring; row → product stock drawer; Adjust.

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
`ui-mocks/src/pages/P25-…/` · route `/p25`
