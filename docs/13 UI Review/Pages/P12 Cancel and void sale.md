---
tags: [ui, page-spec]
page: P12
status: not-started
---
# P12 Cancel & void sale

**Status:** ⬜ Not started · **Area:** Till · **Type:** Dialog · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Throw away the current sale, or void a completed one from this shift.

| | |
|---|---|
| **Who uses it** | Cashier / supervisor |
| **How often** | Sometimes |
| **Requirements** | [[Req - Checkout]] CHK-05, [[Req - Returns and Voids]] RET-04 |
| **Today's code** | `CancelDialog`, `VoidDialog` |

## What it should contain (starting point for the mock)
What will happen in plain words, reason (void), confirm in red, approval if needed.

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
`ui-mocks/src/pages/P12-…/` · route `/p12`
