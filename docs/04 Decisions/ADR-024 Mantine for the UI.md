---
tags: [adr]
status: superseded
---
# ADR-024 Mantine for the UI (Superseded by Direct Tailwind v4 + PosKit Overhaul)

## Context
After M12 the product owner tried the app: the features are good, but the initial hand-built screens were inconsistent and cluttered (e.g. a wall of 17 buttons on the till, 18 ungrouped menu items in the sidebar, and tooltips covering input fields while typing). We initially considered introducing Mantine v9 in a separate `ui-mocks/` folder to prototype replacements.

## Decision
**Superseded.** Rather than introducing a second styling engine (Mantine CSS-in-JS) and maintaining a separate throwaway `ui-mocks/` app, we executed the complete UI overhaul **directly inside `ui/src/`** using our existing **React 19 + TypeScript + Tailwind CSS v4 + Radix UI Tooltip** stack ([[ADR-002 React TypeScript Tailwind for UI]]) and the flat WCAG AAA token system ([[ADR-021 Flat colour-coded UI with tooltips]]).

To provide the standardized building blocks and layouts described in [[UI Guidelines]]:
- We built `ui/src/shared/ui/PosKit.tsx` (`KitButton`, `PageHeader`, `KitField`, `KitBadge`, `KitDrawer`, `KitNumpad`) and `ui/src/shared/ui/HelpDialog.tsx` (`F1` Keyboard & Colour Guide).
- We upgraded `Field.tsx` and `SelectField.tsx` so every form control renders persistent plain-English helper text directly below its label, while positioning hover tooltips above controls (`side="top"`, `pointer-events-none`) so inputs are never obscured while typing.
- We rebuilt `OfficeFrame.tsx` with a collapsible grouped sidebar, breadcrumb context strip, and global `Ctrl+K` `CommandPalette.tsx`.
- We rebuilt `CheckoutPage.tsx` into a 60/40 split POS layout with `ProductCatalogPane.tsx` (touch product tiles + category pills), `ButtonBar.tsx` (5 primary actions + `More` toggle), and `LineEditDrawer.tsx` (numpad drawer for quantity, discount, and price overrides).

## Consequences
- ✅ Zero double-work: all screens are wired directly to the real `ts-rs` generated types (`ui/src/generated/`) and mock/Tauri bridge (`ui/src/shared/bridge/`).
- ✅ All 36 Vitest suites (791 tests)—including `theme.contrast.test.ts` (7:1 WCAG AAA contrast), `flat-style.test.ts` (zero shadows, blurs, gradients, or transitions), and `moduleBoundaries.test.ts`—pass with zero errors.
- ✅ No extra bundle bloat from a second component library; the till bundle remains fast and lightweight.
