---
tags: [architecture, frontend]
---
# Frontend Architecture

How the screens are built. Stack: React 19, TypeScript (strict), Vite, Tailwind v4, Radix UI Tooltip, react-hotkeys-hook, i18next. Look and feel: [[Design System]].

## Folder layout

```
ui/
├── src/
│   ├── main.tsx                 # start-up: TooltipProvider, i18n, and AppFrame
│   ├── app/                     # the frames: AppFrame, OfficeFrame (grouped sidebar + breadcrumbs),
│   │                            #   TillFrame, CommandPalette (Ctrl+K), HomePage, i18n.ts
│   ├── shared/
│   │   ├── ui/                  # flat components: ActionButton, IconButton, Tip, Field, SelectField,
│   │   │                        #   Dialog, Alert, Badge, CheckBox, HelpDialog (F1), PosKit.tsx
│   │   ├── approval/            # ApprovalDialog and useApproval() hook for admin overrides
│   │   ├── print/               # PrintProblem banner and system print helpers
│   │   ├── bridge/              # calls into Rust, error-to-message mapping, fake bridge for browser testing
│   │   ├── format/              # formatMoney, formatQty, formatPercent, formatDateTime
│   │   ├── keyboard/            # shortcuts.ts: the single list of shortcut keys (used with react-hotkeys-hook)
│   │   ├── text/                # i18next settings + shared namespaces (common, errors, approval, permissions, print)
│   │   └── theme.css            # WCAG AAA colour tokens; shadows removed from the theme
│   ├── generated/               # TypeScript types made from the Rust types via ts-rs. Don't edit by hand
│   └── modules/
│       ├── checkout/            # api.ts, text/en.ts, pages/CheckoutPage.tsx, components/ (ProductCatalogPane,
│       │                        #   CartTable, LineEditDrawer, ButtonBar, TotalsPanel, PaymentDialog, …)
│       ├── catalog/  inventory/  pricing/  customers/  purchasing/
│       ├── shifts/  reports/  io/  setup/  settings/  users/  audit/  backup/  tax/  hardware/  auth/
└── e2e/                         # Playwright tests (run against the fake bridge)
```

## Rules
1. **No business maths in TypeScript.** Totals, tax, discounts, stock and permissions all come from Rust. The UI only shows them.
2. **All calls to Rust go through `modules/<x>/api.ts`.** Components never call `invoke` themselves.
3. **Types come from `generated/`.** Never retype a Rust type by hand.
4. **Modules stay separate:** a module folder imports only `shared/`, `generated/` and itself (`eslint-plugin-boundaries` and `moduleBoundaries.test.ts` enforce it).
5. **Screen state** (which dialog/drawer is open, which line is selected) uses plain `useState`. No global store library.
6. **All screen text lives in the module's `text/en.ts`** (labels, tooltips, messages), in plain English ([[Writing Style]]). Each `text/en.ts` is an **i18next namespace** registered in `app/i18n.ts` and typed in `app/i18next.d.ts` (`app/i18n.test.tsx` proves it). See [[ADR-022 Ready for more languages later]].
7. **Every button and field explains itself.** `ActionButton`, `IconButton`, `Field` and `SelectField` require a `tip` prop. `Field` and `SelectField` render their `tip` text persistently below the label and position hover tooltips above the control (`side="top"`, `pointer-events-none`) so tooltips never cover an input while typing.
8. **Flat styling only:** colour tokens from [[Design System]] (`ui/src/shared/theme.css`). `flat-style.test.ts` fails the build if any shadow, blur, gradient, animation, or transition class is used, and `theme.contrast.test.ts` fails if any colour pair drops below 7 : 1 WCAG AAA contrast.
9. **Money & Quantities:** shown with `formatMoney(minor, currency)` and `formatQty(milli, decimals)` using `tabular-nums` alignment.

## Checkout data flow (the fast path)
- The cart is kept in `useTill.ts` on the checkout page. Every cart action returns **the whole new cart (`CartView`)** from Rust.
- `CheckoutPage.tsx` uses a **60/40 Split POS Layout**:
  - **Left 60% (`ProductCatalogPane.tsx`):** Persistent-focus `ScanBox`, category filter pills, touch product tiles, and `ButtonBar` (5 primary buttons + `More` toggle).
  - **Right 40%:** `CustomerBar`, `CartTable` (clicking `Numpad` or double-clicking a line opens `LineEditDrawer` with `KitNumpad`), and `TotalsPanel` (`56px` TOTAL + `72px` green `Pay F10` button).
