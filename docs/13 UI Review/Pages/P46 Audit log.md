---
tags: [ui, page-spec]
page: P46
status: not-started
---
# P46 Audit log

**Status:** ⬜ Not started · **Area:** Admin · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
See who did what (voids, discounts, overrides…).

| | |
|---|---|
| **Who uses it** | Owner |
| **How often** | Weekly |
| **Requirements** | [[Req - Users and Security]] USR-08 |
| **Today's code** | `AuditLogPage` |

## What it should contain (starting point for the mock)
Filters (date, user, action), table, detail drawer in plain words, export.

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
`ui-mocks/src/pages/P46-…/` · route `/p46`
