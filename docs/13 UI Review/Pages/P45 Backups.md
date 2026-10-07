---
tags: [ui, page-spec]
page: P45
status: not-started
---
# P45 Backups

**Status:** ⬜ Not started · **Area:** Admin · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Check backups and restore one.

| | |
|---|---|
| **Who uses it** | Owner |
| **How often** | Weekly |
| **Requirements** | [[Req - Backup and Logging]] BAK-01..05 |
| **Today's code** | `BackupsPage`, `RestoreConfirm` |

## What it should contain (starting point for the mock)
Last backup status, second folder status, list (date, reason, size, verified, sales, last invoice), Back up now, Restore with strong confirm.

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
`ui-mocks/src/pages/P45-…/` · route `/p45`
