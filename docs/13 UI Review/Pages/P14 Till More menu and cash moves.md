---
tags: [ui, page-spec]
page: P14
status: not-started
---
# P14 Till 'More' menu & cash moves

**Status:** ⬜ Not started · **Area:** Till · **Type:** Menu + dialogs · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Rarer till actions in one place.

| | |
|---|---|
| **Who uses it** | Cashier / supervisor |
| **How often** | Daily |
| **Requirements** | [[Req - Shifts and Cash]] SHF-03..05, [[Req - Checkout]] CHK-17 |
| **Today's code** | `ShiftMenu`, `CashMoveDialog`, `PriceListDialog` |

## What it should contain (starting point for the mock)
Price list switch, reprint, cash in / out / drop with reason, no-sale drawer, X report, close shift, back office, lock.

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
`ui-mocks/src/pages/P14-…/` · route `/p14`
