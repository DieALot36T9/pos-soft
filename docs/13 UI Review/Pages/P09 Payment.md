---
tags: [ui, page-spec]
page: P09
status: not-started
---
# P09 Payment

**Status:** ⬜ Not started · **Area:** Till · **Type:** Full-screen dialog · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Take the money, split if needed, and see the change.

| | |
|---|---|
| **Who uses it** | Cashier |
| **How often** | Every sale |
| **Requirements** | [[Req - Payments]] |
| **Today's code** | `PaymentDialog` |

## What it should contain (starting point for the mock)
Amount due (huge), method tiles in their colours, numpad, quick-cash notes, split list, credit and points limits, green Complete, big change.

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
`ui-mocks/src/pages/P09-…/` · route `/p09`
