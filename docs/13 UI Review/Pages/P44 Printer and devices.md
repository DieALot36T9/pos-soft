---
tags: [ui, page-spec]
page: P44
status: not-started
---
# P44 Printer & devices

**Status:** ⬜ Not started · **Area:** Admin · **Type:** Page · Rules: [[UI Guidelines]] · Process: [[UI Review Process]] · All pages: [[Page List]]

## What this page is for
Set up and test the printer, drawer, scale and label printer.

| | |
|---|---|
| **Who uses it** | Owner |
| **How often** | Rarely |
| **Requirements** | [[Req - Hardware and Printing]] |
| **Today's code** | `HardwarePage` (+ test print, preview printer panels) |

## What it should contain (starting point for the mock)
One card per device with status light, settings, Test button and result.

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
`ui-mocks/src/pages/P44-…/` · route `/p44`
