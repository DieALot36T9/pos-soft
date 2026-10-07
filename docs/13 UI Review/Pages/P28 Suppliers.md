---
tags: [ui, page-spec]
page: P28
status: not-started
---
# P28 Suppliers

**Status:** ⬜ Not started · **Area:** Buying · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Manage suppliers and see what's owed to each.

| | |
|---|---|
| **Who uses it** | Owner |
| **How often** | Weekly |
| **Requirements** | [[Req - Purchasing]] PUR-01, PUR-06 |
| **Today's code** | `PurchasingPage` → `SuppliersPanel`, `StatementPanel` |

## What it should contain (starting point for the mock)
Table with balance and terms; drawer with details, ledger, statement export.

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
`ui-mocks/src/pages/P28-…/` · route `/p28`
