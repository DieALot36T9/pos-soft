---
tags: [ui, page-spec]
page: P48
status: not-started
---
# P48 Database recovery

**Status:** ⬜ Not started · **Area:** Setup · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
The database check failed: restore the newest good backup.

| | |
|---|---|
| **Who uses it** | Owner |
| **How often** | Hopefully never |
| **Requirements** | [[Req - Backup and Logging]] BAK-06 |
| **Today's code** | `RecoveryPage` |

## What it should contain (starting point for the mock)
Plain explanation, list of good backups, Restore, contact info.

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
`ui-mocks/src/pages/P48-…/` · route `/p48`
