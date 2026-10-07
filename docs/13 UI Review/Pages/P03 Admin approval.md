---
tags: [ui, page-spec]
page: P03
status: not-started
---
# P03 Admin approval

**Status:** ⬜ Not started · **Area:** Getting in · **Type:** Dialog · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
An admin approves one restricted action on the cashier's screen.

| | |
|---|---|
| **Who uses it** | Cashier + admin |
| **How often** | Several times a day |
| **Requirements** | [[Req - Users and Security]] USR-05, [[ADR-015 Permissions enforced in Rust]] |
| **Today's code** | approval dialog in `shared/` (used by many screens) |

## What it should contain (starting point for the mock)
Amber header naming the action, admin choice, password, Approve; the action then runs by itself.

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
`ui-mocks/src/pages/P03-…/` · route `/p03`
