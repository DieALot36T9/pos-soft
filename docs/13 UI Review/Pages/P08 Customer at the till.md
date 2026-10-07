---
tags: [ui, page-spec]
page: P08
status: not-started
---
# P08 Customer at the till

**Status:** ⬜ Not started · **Area:** Till · **Type:** Dialog · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Find a customer by phone, or add one quickly, and attach them to the sale.

| | |
|---|---|
| **Who uses it** | Cashier |
| **How often** | Often |
| **Requirements** | [[Req - Checkout]] CHK-11, [[Req - Customers and Loyalty]] CUS-02 |
| **Today's code** | `CustomerDialog` |

## What it should contain (starting point for the mock)
Phone search as you type, results with points and balance, quick add (phone + name), tier badge.

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
`ui-mocks/src/pages/P08-…/` · route `/p08`
