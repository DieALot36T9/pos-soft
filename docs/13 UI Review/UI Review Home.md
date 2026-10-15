---
tags: [ui, home]
---
# UI Review & Overhaul Status

**What we did:** We overhauled the entire `ui/` frontend directly in production code (`ui/src/`) using **React 19 + TypeScript + Tailwind CSS v4 + Radix Primitives + `PosKit.tsx`**, wired to the real `ts-rs` generated types (`ui/src/generated/`) and fake/Tauri bridge (`ui/src/shared/bridge/`).

## Key Improvements Delivered
1. **App Shell & Navigation (`OfficeFrame.tsx`, `CommandPalette.tsx`):** Replaced the flat 18-item sidebar with 9 collapsible navigation groups, a top breadcrumb & plain-English page description strip, and a `Ctrl+K` Command Palette.
2. **Self-Explaining Form Fields (`Field.tsx`, `SelectField.tsx`, `tooltip.tsx`):** Every form field renders its plain-English description and example persistently below the label, and hover tooltips open above elements (`side="top"`, `pointer-events-none`) so they never block inputs while typing.
3. **60/40 Split POS Till (`CheckoutPage.tsx`, `ProductCatalogPane.tsx`, `LineEditDrawer.tsx`, `ButtonBar.tsx`, `HelpDialog.tsx`):**
   - Left pane with persistent-focus scan box, category filter pills, touch product tiles, and a clean 5-button action bar + `More` toggle.
   - Right pane with customer bar, cart table, slide-over `LineEditDrawer` touch numpad, `56px` TOTAL box, and full-width `72px` green `Pay (F10)` button.
   - `F1` opens the interactive Keyboard Shortcuts & Colour Guide (`HelpDialog.tsx`).
4. **Two-Column Touch Login & Lock Screens (`LoginPage.tsx`, `LockScreen.tsx`):** Staff avatar tiles with role badges on the left and an integrated `KitNumpad` PIN pad on the right.
5. **Standardized Back-Office Pages:** Unified filter cards, table headers, right-hand quick-view drawers (`ProductsPage.tsx`, `AuditLogPage.tsx`), and KPI cards on `HomePage.tsx`.

## Running & Verifying the UI
- **Interactive Browser Mode (Mock Bridge):** `pnpm --filter ui dev:mock`
- **Typecheck:** `pnpm --filter ui typecheck`
- **Unit & Integration Tests (36 suites, 791 tests):** `pnpm --filter ui test`
- **Lint & Format Check:** `pnpm --filter ui lint`
- **End-to-End Playwright Suite:** `pnpm --filter ui e2e`
