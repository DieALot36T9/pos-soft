---
tags: [ui, page-spec]
page: P33
status: not-started
---
# P33 Customers list

**Status:** ⬜ Not started · **Area:** Customers · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Find customers and see balances.

| | |
|---|---|
| **Who uses it** | Owner / staff |
| **How often** | Daily |
| **Requirements** | [[Req - Customers and Loyalty]] CUS-01, CUS-02 |
| **Today's code** | `CustomersPage` |

## What it should contain (starting point for the mock)
Search by phone/name, filters (tier, owes money), table with points and balance, row → profile.

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
`ui-mocks/src/pages/P33-…/` · route `/p33`
