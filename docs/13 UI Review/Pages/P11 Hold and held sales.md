---
tags: [ui, page-spec]
page: P11
status: not-started
---
# P11 Hold & held sales

**Status:** ⬜ Not started · **Area:** Till · **Type:** Dialog · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Park a sale and bring it back later.

| | |
|---|---|
| **Who uses it** | Cashier |
| **How often** | Daily |
| **Requirements** | [[Req - Checkout]] CHK-09, CHK-10 |
| **Today's code** | `HoldDialog`, `HeldListDialog` |

## What it should contain (starting point for the mock)
Hold with a note, held list (time, cashier, items, total, note), resume, cancel held.

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
`ui-mocks/src/pages/P11-…/` · route `/p11`
