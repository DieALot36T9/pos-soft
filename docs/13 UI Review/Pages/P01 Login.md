---
tags: [ui, page-spec]
page: P01
status: not-started
---
# P01 Login

**Status:** ⬜ Not started · **Area:** Getting in · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Pick your name and type your PIN or password.

| | |
|---|---|
| **Who uses it** | Everyone |
| **How often** | Every shift |
| **Requirements** | [[Req - Users and Security]] USR-01, USR-02, USR-09 |
| **Today's code** | `modules/auth/pages/LoginPage.tsx` |

## What it should contain (starting point for the mock)
User tiles (photo initials, name, role), PIN pad, password field for admins, lockout countdown, 'Forgot admin password'.

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
`ui-mocks/src/pages/P01-…/` · route `/p01`
