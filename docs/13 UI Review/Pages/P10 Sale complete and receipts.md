---
tags: [ui, page-spec]
page: P10
status: not-started
---
# P10 Sale complete & receipts

**Status:** ⬜ Not started · **Area:** Till · **Type:** Panel + dialog · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
See the change and receipt status; reprint when needed.

| | |
|---|---|
| **Who uses it** | Cashier |
| **How often** | Every sale |
| **Requirements** | [[Req - Payments]] PAY-09, PAY-10 |
| **Today's code** | `ReprintDialog`, print banner |

## What it should contain (starting point for the mock)
Change shown big until the next scan, receipt printed / not printed with Retry and Print elsewhere, reprint last, find an old receipt.

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
`ui-mocks/src/pages/P10-…/` · route `/p10`
