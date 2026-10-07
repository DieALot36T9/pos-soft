---
tags: [ui, guidelines]
status: draft (grows as pages are approved)
---
# UI Guidelines

The rules every screen follows, so the whole app feels like **one** app. Every mock (and later the real screens) is built from these. When a page review settles a new pattern, it's added here, so the next page reuses it instead of inventing its own.

**Goals from the product owner:** easy to navigate, battle-tested patterns, consistent, quality-of-life touches, flat and plain, colour-coded so staff work by looking, always readable, tooltips that teach.

Colours, contrast and the colour meanings come from [[Design System]] and don't change. This page adds *how screens are built*.

---

## 1. Building blocks: Mantine
**Use Mantine components for everything. Don't hand-build a component Mantine already has.** ([[ADR-024 Mantine for the UI]])

| Need | Use |
|---|---|
| App frame (header + side menu) | `AppShell` |
| Side menu items and groups | `NavLink` (nested, collapsible) |
| Page title, breadcrumbs | `Title`, `Breadcrumbs` |
| Buttons | `Button`, `ActionIcon` (icon-only, always with `Tooltip`) |
| "More" actions | `Menu` |
| Tables | `mantine-datatable` (sorting, selection, row actions, pagination); `Table` for small fixed tables |
| Search box | `TextInput` with a search icon, or `Autocomplete` |
| Choosing from a list | `Select` (searchable) / `MultiSelect` |
| Numbers and money | `NumberInput` (through our `MoneyInput` / `QtyInput` wrappers) |
| Dates | `@mantine/dates` `DatePickerInput` |
| Forms | `@mantine/form` (`useForm`) |
| Dialogs | `Modal`; confirmations with `@mantine/modals` `openConfirmModal` |
| Quick view / edit beside a list | `Drawer` (from the right) |
| Steps (setup, import) | `Stepper` |
| Tabs inside a page | `Tabs` |
| Toast messages | `@mantine/notifications` |
| Command palette (`Ctrl+K`) | `@mantine/spotlight` |
| Status words | `Badge` (soft colour + icon + word) |
| Messages on a page | `Alert` |
| Empty pages | our `EmptyState` (icon + one sentence + one button), built from Mantine parts |
| Loading | `Skeleton` in the shape of the content (never a bare spinner on a full page) |
| Keyboard shortcuts | `react-hotkeys-hook`, shown with Mantine `Kbd` |
| Icons | `lucide-react`, one fixed icon per action ([[Design System#Icons]]) |

## 2. Theme
- Everything lives in one file, `ui-mocks/src/theme.ts` (later the real `ui` uses the same file).
- `MantineProvider` with `forceColorScheme="light"`: one look, the computer's dark mode is ignored.
- **No shadows:** every `theme.shadows` size is `none`. Panels use `withBorder`.
- **No animations:** transitions set to 0 ms (modals, drawers, menus, tooltips). The only movement allowed is the 0.3 s flash on a new cart line.
- `defaultRadius: "sm"` (6 px). System font. Font sizes from [[Design System#Shapes and sizes]].
- **Colours:** one 10-shade Mantine palette per colour meaning (green, red, amber, blue, purple, teal, grey). Shade 0–1 = our *soft* colour, shade 7 = our *strong* colour. `primaryColor: "blue"`. Every text/background pair keeps **7:1 contrast**, checked by a test.
- Buttons use colour **only by meaning** ([[Design System#Colour code]]): green = finish / money in, red = remove / money out, amber = wait, blue = find / info, purple = price change, grey = neutral. Use `variant="filled"` for the one main action, `variant="light"` for secondary actions, `variant="default"` for neutral ones.

## 3. Layouts

### 3a. Back office: the standard admin layout
```
┌─────────────────────────────────────────────────────────────────────────┐
│ Demo Supershop · T1     [ 🔍 Search or jump to…  Ctrl+K ]   Rina ▾  [Till]│  header 56 px
├──────────────┬──────────────────────────────────────────────────────────┤
│ Home         │ Catalog / Products                         breadcrumbs    │
│ SELL         │ Products                              [ + Add product ]   │  page header
│  Till        │ Everything the shop sells.                                │
│  Sales       │ [search……………] [Category ▾] [Brand ▾] [Low stock] (chips) │  filters bar
│ CATALOG      │ ┌──────────────────────────────────────────────────────┐ │
│  Products    │ │ table: sortable, row click opens the drawer          │ │  content
│  Categories  │ │ …                                                    │ │
│  …           │ └──────────────────────────────────────────────────────┘ │
│ STOCK        │                                       1–25 of 105  ‹ ›   │
│ BUYING …     │                                                           │
└──────────────┴──────────────────────────────────────────────────────────┘
```
- **Menu groups** (collapsible, remembered): Home · **Sell** (Till, Sales) · **Catalog** (Products, Categories & brands, Units, Labels & tags) · **Pricing** (Price lists, Tiers, Promotions) · **Stock** (Stock levels, Stock counts) · **Buying** (Suppliers, Orders, Receive goods, Supplier returns, Payables) · **Customers** · **Reports** · **Data** (Import, Export) · **Admin** (Users & roles, Settings, Tax rates, Printer & devices, Backups, Audit log). Items the user can't use are hidden.
- **Page anatomy, always in this order:** breadcrumbs → title + one-line description → **one** main button top-right → filters bar → content → pagination.
- **Lists open details in a side drawer** for quick look and quick edit. Complex editors (product, purchase order, import) open as a **full page** with a sticky footer (`Cancel` left, `Save` right).
- The header **Till** button always goes back to the till.

### 3b. Till: the standard POS layout (as in Square, Shopify POS, Odoo POS)
```
┌──────────────────────────────────────────────────────────────────────────┐
│ T1 · Rina · since 09:02   Customer: — (F3)     🟩 Printer 🟩 Scale  12:41 🔒 │
├────────────────────────────────────────────┬─────────────────────────────┤
│ [ 🔍 Scan or search…                      ] │ Customer chip / Add (F3)    │
│ (All)(Rice)(Oil)(Snacks)(Drinks)(Fresh)…   │ Milk 1L        [-] 2 [+]  180 │
│ ┌──────┐┌──────┐┌──────┐┌──────┐┌──────┐   │ Potato ⚖ 1.25 kg           75 │
│ │ tile ││ tile ││ tile ││ tile ││ tile │   │ Soap  [-] 3 [+]   150         │
│ └──────┘└──────┘└──────┘└──────┘└──────┘   │   🟪 Buy 2 get 1      −50     │
│ product tiles: favourites or the category  │ ─────────────────────────────│
│                                            │ Subtotal / Discounts / VAT    │
│                                            │ TOTAL           ৳1,377.00    │
│ [Customer F3][Hold F5][Discount F7][Return][More ▾] │ [   🟩 PAY  F10   ]  │
└────────────────────────────────────────────┴─────────────────────────────┘
```
- **Left (about 60%) = find products:** the scan box always has focus; category chips; product tiles (name, price, stock badge; image if any). Tiles are for touch and for things without barcodes (weighed produce).
- **Right (about 40%, min 420 px) = the order:** customer, lines with `−`/`+`, promotion lines in purple, totals, and the **biggest button on screen: green Pay**.
- **At most 5 visible actions** under the products. Everything else is in **More** (and `Ctrl+K`): reprint, price list, cash in/out, no-sale, X report, close shift, back office.
- **Click or tap a cart line** to open the **line panel**: quantity, discount, price, remove, all with one **numpad**.
- **Payment** is a full-screen dialog: amount due very big, method tiles in their colours, numpad, quick-cash notes, then green Complete.

## 4. Patterns
### Tables
- Columns: the most important first; numbers and money right-aligned with fixed-width digits; status as a `Badge`.
- **Row click = open details** (drawer). One visible text action per row at most; the rest in a `…` menu.
- Sort by clicking headers. Search box filters as you type (200 ms).
- Filters are chips and selects above the table; active filters show as removable chips; "Clear filters" link.
- Selection checkboxes only where bulk actions exist (print labels, archive, export).
- 25 rows per page by default; the choice is remembered.
- **Empty table:** icon + sentence + the main button ("No products yet. [Add product] or [Import from Excel]").

### Forms
- Label above the field; help text under it (`description`); `*` for required.
- Errors show under the field when you leave it, in plain words ([[Writing Style]]).
- Long forms: sections with headings, sticky footer with `Cancel` / `Save`.
- **Unsaved changes:** leaving asks "Discard changes?".
- `Enter` saves single-field dialogs; `Esc` cancels.
- Money with `MoneyInput`, quantity with `QtyInput`; never plain text boxes for numbers.

### Dialogs and drawers
- Title says what happens ("Close shift", not "Shift").
- Footer: secondary on the left (`Back`/`Cancel`), main action on the right (coloured by meaning).
- Ask for confirmation **only for things that can't be undone** (void, delete, restore, post count). Everything else just happens, with an **Undo** in the toast where possible.

### Feedback
- Success: a short toast at the bottom-right ("Product saved"), gone after 4 s, with Undo when possible.
- Problem: a red `Alert` near the cause, saying what happened and what to do.
- Loading: skeletons shaped like the content; buttons show a loader while their action runs.
- Hardware status: green/red status lights in the till header, with tooltips.

### Tooltips (the owner's rule: every button and field explains itself)
- **Every button, icon and status light** has a tooltip: what it does + shortcut key. 0.4 s delay.
- **Fields** explain themselves with help text *under* the field, plus an ⓘ icon next to the label whose tooltip holds the longer explanation and an example. **A tooltip never covers the field you're typing in.**
- Disabled buttons say why in their tooltip.

### Keyboard
- `Ctrl+K`: search or jump anywhere (pages, products, customers, actions).
- Till keeps its function keys ([[Keyboard Shortcuts]]). Back office: `Ctrl+N` new, `Ctrl+S` save, `/` focus search, `Esc` close.
- Shortcut hints shown with `Kbd` inside button tooltips, not cluttering the button.

## 5. Quality-of-life checklist (every page)
- [ ] Remembers last filters, sort, page size and open menu groups
- [ ] Main action findable in under 3 seconds
- [ ] Works with keyboard only; focus always visible
- [ ] Works on 1366×768 without horizontal scrolling; touch targets ≥ 44 px (till buttons ≥ 56 px)
- [ ] Empty, loading, error and "no permission" states designed
- [ ] Destructive actions confirmed; others undoable
- [ ] Numbers right-aligned; money formatted by the shop's currency
- [ ] Tooltips on all buttons and icons; field help under fields
- [ ] Every word in plain English, in the module's text file

## 6. Don't
- ❌ Walls of buttons. Group them; put rare actions in **More**.
- ❌ Two coloured "main" buttons on one screen.
- ❌ Icon-only buttons without a tooltip, or anywhere on the till.
- ❌ Hand-built tables, dropdowns, date pickers, dialogs (use Mantine).
- ❌ Shadows, gradients, animations, dark mode.
- ❌ Colour used for decoration.
- ❌ Empty space where useful information could be (e.g. an empty totals panel).

## 7. Sizes
| Thing | Size |
|---|---|
| Back-office header / side menu | 56 px / 248 px wide |
| Till header | 56 px |
| Till order panel | 40% of the width, at least 420 px |
| Till buttons | ≥ 56 px tall; Pay ≥ 72 px |
| Product tiles | 140 × 96 px |
| Back-office buttons | 36–40 px tall |
| Table rows | 44 px (comfortable) |

## Change log
| Date | Change | From page |
|---|---|---|
| 2026-10-07 | First version | — |
