---
tags: [ui, page-spec]
page: P04
status: not-started
---
# P04 Open shift

**Status:** ⬜ Not started · **Area:** Till · **Type:** Dialog · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Count the starting cash and open the till.

| | |
|---|---|
| **Who uses it** | Cashier |
| **How often** | Start of every shift |
| **Requirements** | [[Req - Shifts and Cash]] SHF-01 |
| **Today's code** | `modules/shifts/pages/OpenShiftDialog.tsx` |

## What it should contain (starting point for the mock)
Float amount (MoneyInput), optional count by note, Open.

## Problems in today's version
The tooltip covers the amount field while typing.

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
`ui-mocks/src/pages/P04-…/` · route `/p04`
