---
tags: [ui, page-spec]
page: P37
status: not-started
---
# P37 Import

**Status:** ⬜ Not started · **Area:** Data · **Type:** Wizard page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Bring products, customers, suppliers, stock or prices in from Excel/CSV.

| | |
|---|---|
| **Who uses it** | Owner |
| **How often** | Rarely |
| **Requirements** | [[Req - Import and Export]] IO-01, IO-02 |
| **Today's code** | `ImportWizard` |

## What it should contain (starting point for the mock)
Stepper: what → file → sheet → match columns → preview with errors → import with progress → summary.

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
`ui-mocks/src/pages/P37-…/` · route `/p37`
