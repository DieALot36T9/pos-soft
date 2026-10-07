---
tags: [ui, page-spec]
page: P15
status: not-started
---
# P15 Close shift

**Status:** ⬜ Not started · **Area:** Till · **Type:** Dialog · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Count the drawer and close; print the Z report.

| | |
|---|---|
| **Who uses it** | Cashier |
| **How often** | End of every shift |
| **Requirements** | [[Req - Shifts and Cash]] SHF-06, SHF-08 |
| **Today's code** | `CloseShiftDialog` |

## What it should contain (starting point for the mock)
Count by note (blind first), expected after counting, difference highlighted, note, Close & print.

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
`ui-mocks/src/pages/P15-…/` · route `/p15`
