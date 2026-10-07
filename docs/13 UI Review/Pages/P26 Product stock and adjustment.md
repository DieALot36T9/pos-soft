---
tags: [ui, page-spec]
page: P26
status: not-started
---
# P26 Product stock & adjustment

**Status:** ⬜ Not started · **Area:** Stock · **Type:** Page / drawer · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
See one product's batches and history, and adjust its stock.

| | |
|---|---|
| **Who uses it** | Owner / manager |
| **How often** | Daily |
| **Requirements** | [[Req - Inventory]] INV-01, INV-02, INV-05, INV-07 |
| **Today's code** | `StockProductPage` |

## What it should contain (starting point for the mock)
On hand, batches with expiry, history with running balance, Adjust (reason, qty, batch).

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
`ui-mocks/src/pages/P26-…/` · route `/p26`
