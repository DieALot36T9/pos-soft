---
tags: [ui, page-spec]
page: P16
status: not-started
---
# P16 Home dashboard

**Status:** ⬜ Not started · **Area:** Back office · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
See how today is going and what needs attention.

| | |
|---|---|
| **Who uses it** | Owner / manager |
| **How often** | Daily |
| **Requirements** | [[Back Office Screens#Dashboard]] |
| **Today's code** | `app/HomePage.tsx` |

## What it should contain (starting point for the mock)
Today's sales, number of sales, average basket, open shift; alert cards (low stock, near expiry, negative stock, backups, printer) each with a link.

## Problems in today's version
Only a stock-alerts card and a 'Welcome / Check app' card; no sales figures; lots of empty space.

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
`ui-mocks/src/pages/P16-…/` · route `/p16`
