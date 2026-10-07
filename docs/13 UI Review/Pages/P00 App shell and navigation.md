---
tags: [ui, page-spec]
page: P00
status: not-started
---
# P00 App shell & navigation

**Status:** ⬜ Not started · **Area:** Foundations · **Type:** Frame · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Get to any screen in one or two clicks, and always know where you are.

| | |
|---|---|
| **Who uses it** | Everyone |
| **How often** | All day |
| **Requirements** | [[Screen Map]], [[Frontend Architecture]] |
| **Today's code** | `ui/src/app/AppFrame.tsx`, `OfficeFrame.tsx`, `TillFrame.tsx` |

## What it should contain (starting point for the mock)
Header (shop, till, user, Ctrl+K search, Till button), grouped side menu, breadcrumbs, page header pattern, toasts.

## Problems in today's version
18 menu items in one flat list with no groups; Categories, Brands and Units are top-level; no search or command palette; no breadcrumbs; header shows only the app name.

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
`ui-mocks/src/pages/P00-…/` · route `/p00`
