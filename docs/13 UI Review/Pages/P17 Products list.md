---
tags: [ui, page-spec]
page: P17
status: not-started
---
# P17 Products list

**Status:** ⬜ Not started · **Area:** Catalog · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Find, check and manage products.

| | |
|---|---|
| **Who uses it** | Owner / manager |
| **How often** | Daily |
| **Requirements** | [[Req - Catalog]] |
| **Today's code** | `modules/catalog/pages/ProductsPage.tsx` |

## What it should contain (starting point for the mock)
Search, category/brand filters, low-stock / no-barcode chips, table (name, SKU, category, price, cost, stock, status), row → drawer, bulk print labels / archive, Add, Import.

## Problems in today's version
Unlabeled icon buttons in an 'Actions' column; no stock column; no sorting; no bulk actions.

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
`ui-mocks/src/pages/P17-…/` · route `/p17`
