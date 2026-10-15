---
tags: [ui, guidelines]
status: active
---
# UI Guidelines

The rules every screen in `ui/src/` follows so the whole app feels like **one** cohesive, industrial-grade POS and Back Office.

Colours, contrast and colour meanings come from [[Design System]] (`ui/src/shared/theme.css`).

---

## 1. Building blocks (`ui/src/shared/ui/`)
We use our flat **Tailwind CSS v4 + Radix UI Tooltip** component kit in `ui/src/shared/ui/` (no external CSS-in-JS framework):

| Need | Component |
|---|---|
| Back-office shell (top bar + grouped sidebar + context banner) | `OfficeFrame.tsx` |
| Command palette (`Ctrl+K`) | `CommandPalette.tsx` |
| Till shell (checkout + shift dialogs + preview printer) | `TillFrame.tsx` + `CheckoutPage.tsx` |
| Semantic action buttons | `ActionButton.tsx` / `KitButton` (`PosKit.tsx`) |
| Icon-only action buttons (back office only, with tooltip) | `IconButton.tsx` |
| Self-explaining text / number inputs | `Field.tsx` (renders label + persistent `tip` description below label + input + error) |
| Self-explaining select dropdowns | `SelectField.tsx` (renders label + persistent `tip` description below label + select + error) |
| Checkboxes | `CheckBox.tsx` |
| Modals / Dialogs | `Dialog.tsx` |
| Right-hand slide-over drawers (quick view & cart line editor) | `KitDrawer` (`PosKit.tsx`) |
| Touch & mouse numpad (login PIN, lock screen, line editor) | `KitNumpad` (`PosKit.tsx`) |
| Status badges (soft colour + icon + word) | `Badge.tsx` / `KitBadge` (`PosKit.tsx`) |
| Page banners & alerts | `Alert.tsx` |
| Keyboard & colour help overlay (`F1`) | `HelpDialog.tsx` |

## 2. Theme (`ui/src/shared/theme.css`)
- **Forced light scheme:** `color-scheme: light only`. The OS dark mode is ignored so the screen looks identical on every till.
- **Zero shadows, blurs, gradients, or animations:** Enforced by `ui/src/shared/flat-style.test.ts`. Panels use 1 px solid `--border` (`#B9BEC4`).
- **WCAG AAA 7:1+ contrast:** Every text/background token pair in `theme.css` is verified by `ui/src/shared/theme.contrast.test.ts`.
- **Semantic colours only:** Green = finish / money in / save; Red = remove / cancel / money out / danger; Amber = wait / hold / attention / admin approval; Blue = find / info / selected; Purple = discounts / promotions / price changes; Teal = mobile wallet; Grey = neutral / back / lock.

## 3. Layouts

### 3a. Back Office (`OfficeFrame.tsx`)
- **Top Bar (48 px):** `Back Office` badge, active `Group / Page` breadcrumb, central `Search or jump to screen… Ctrl+K` command palette trigger, and green `Open Till` button.
- **Collapsible Grouped Sidebar (256 px):**
  - **Overview:** Home
  - **Sell:** Checkout
  - **Catalog:** Products, Categories, Brands, Units, Labels and shelf tags
  - **Pricing:** Prices and promotions
  - **Stock:** Stock, Stock counts
  - **Buying:** Purchasing (Suppliers, Purchase orders, Receive goods, Supplier returns, What we owe)
  - **Customers:** Customers
  - **Reports & Data:** Reports, Import and export
  - **Admin:** Users, Roles, My PIN and password, Settings, Tax rates, Printer and devices, Backups, Audit log
- **Main Content Area:** Contextual description banner at the top of the page, followed by filter cards (`bg-panel border-border rounded-ui p-4`), high-contrast tables (`bg-bg` header row, `tabular-nums` right-aligned number columns, hover highlight, slide-over `KitDrawer` on row click where applicable), and pagination/action bars.

### 3b. Till (`CheckoutPage.tsx`)
- **Top Header (`TillHeader.tsx`):** Till status badge, active cashier & shift start time, Business Day badge, and `Help F1` button (`HelpDialog.tsx`).
- **Left 60% (`ProductCatalogPane.tsx`):** Persistent-focus `ScanBox`, category filter pills (`All Product Tiles`, `⚖ Weighed / Fresh`, `Low Stock`), touch product tiles (`min-h-[96px]`), and `ButtonBar` (5 primary buttons + `More` toggle).
- **Right 40% (`min-w-[420px]`):** `CustomerBar`, status/alert bars, scrollable `CartTable` (with `LineEditDrawer` numpad inspector), and bottom-docked `TotalsPanel` (`56px` TOTAL + `72px` green `Pay F10` button).
