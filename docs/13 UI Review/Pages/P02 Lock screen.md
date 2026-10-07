---
tags: [ui, page-spec]
page: P02
status: not-started
---
# P02 Lock screen

**Status:** ⬜ Not started · **Area:** Getting in · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Hide the till while away; come back with your PIN.

| | |
|---|---|
| **Who uses it** | Everyone |
| **How often** | Many times a day |
| **Requirements** | [[Req - Users and Security]] USR-03 |
| **Today's code** | `modules/auth/pages/LockScreen.tsx` |

## What it should contain (starting point for the mock)
Big user tile, PIN pad, 'Switch user' (only with an empty or held cart), admin unlock.

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
`ui-mocks/src/pages/P02-…/` · route `/p02`
