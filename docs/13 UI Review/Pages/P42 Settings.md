---
tags: [ui, page-spec]
page: P42
status: not-started
---
# P42 Settings

**Status:** ⬜ Not started · **Area:** Admin · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Change shop, money, receipt, checkout, stock, loyalty and credit settings.

| | |
|---|---|
| **Who uses it** | Owner |
| **How often** | Rarely |
| **Requirements** | [[Req - Setup and Settings]] SET-13..17 |
| **Today's code** | `SettingsPage` |

## What it should contain (starting point for the mock)
Left list of sections, one form per section with Save; receipt live preview; payment methods; denominations; barcode rules with a test field.

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
`ui-mocks/src/pages/P42-…/` · route `/p42`
