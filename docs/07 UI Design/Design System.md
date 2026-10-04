---
tags: [ui, design]
---
# Design System

**Goal:** a cashier can look at the screen and know what to do **without reading**. The same colour and the same icon always mean the same thing, everywhere in the app. Decision: [[ADR-021 Flat colour-coded UI with tooltips]].

## Rules
1. **Flat and plain.** No shadows, no gradients, no blur, no glass effects, no 3D. Panels are separated by thin 1 px borders.
2. **No movement.** No animations or sliding panels. The only exception: a newly added cart line flashes its background for 0.3 s.
3. **One colour = one meaning** (table below). A colour is never used just for decoration.
4. **Colour is never the only signal.** Every coloured thing also has an icon and a short word, so colour-blind staff (about 1 in 12 men) can still use it.
5. **Things stay in the same place.** The main action is always bottom-right. Back/Cancel is always bottom-left. The scan box is always at the top of the checkout screen.
6. **Every button and field has a floating tooltip** ([[#Tooltips]]).
7. **Calm on the eyes.** Soft off-white background, white panels, dark grey text (not pure black), muted colours, no bright neon.
8. **Big where it matters.** TOTAL and CHANGE are the largest text on screen.
9. **Numbers line up.** Every amount is right-aligned and uses fixed-width digits.
10. **One fixed look, always readable** (no dark or light mode). The app ignores the computer's dark-mode setting and always looks the same. See [[#Always readable]].

## Colour code
| Colour | Means | Used for |
|---|---|---|
| 🟩 **Green** | Money coming in · finish · OK | Pay, Complete sale, Save, cash payment, Cash in, Open shift, device connected, weight stable |
| 🟥 **Red** | Remove · cancel · money going out · problem | Remove line, Cancel sale, Void, Refund, Cash out, Archive/Delete, error messages, negative stock |
| 🟨 **Amber** | Wait · paused · needs attention | Hold sale, Held sales, low stock, near expiry, warnings, "needs admin approval" |
| 🟦 **Blue** | Find · look up · information | Search, Customer, Reprint, Reports, info messages, the selected cart line |
| 🟪 **Purple** | Price changes | Discount, promotion lines ("Buy 2 get 1"), price override, price lists |
| ⬜ **Grey** | Neutral · go back · settings | Back, Close, Lock, Settings, disabled buttons |

### Payment method colours (defaults; the admin can change them from the same palette)
| Method | Colour | Icon |
|---|---|---|
| Cash | Green | banknote |
| Card | Blue | credit card |
| Mobile wallet | Teal | phone |
| Customer credit (baki) | Amber | notebook |
| Loyalty points | Purple | star |

These colours appear on the payment buttons, the payment lines in the cart, the receipt viewer and the reports, so a cashier recognises "the green one" anywhere.

## Colour tokens (CSS variables)
| Token | Hex | Use | Contrast |
|---|---|---|---|
| `--bg` | `#F6F7F5` | Page background (soft off-white, less glare than pure white) | — |
| `--panel` | `#FFFFFF` | Panels, cart, dialogs | — |
| `--border` | `#B9BEC4` | All borders and dividers (clearly visible, not faint) | — |
| `--text` | `#1F2328` | Main text and all numbers | 14.7 : 1 |
| `--text-muted` | `#454C54` | Secondary text (still dark, never light grey) | 8.1 : 1 |
| `--green` / `--green-soft` | `#225A39` / `#E5F2EA` | Strong (button fill, white text) / soft (background behind green text) | 8.1 / 7.0 |
| `--red` / `--red-soft` | `#8C2C26` / `#F9E7E5` | | 8.4 / 7.0 |
| `--amber-fill` / `--amber` / `--amber-soft` | `#F2B544` / `#6D4911` / `#FBF0DC` | Amber buttons are **bright amber with dark text** (8.6 : 1) so they read as amber, not brown. `--amber` is for amber text and icons | 8.6 / 7.1 |
| `--blue` / `--blue-soft` | `#255086` / `#E7EFF9` | Also the keyboard focus outline | 8.2 / 7.0 |
| `--purple` / `--purple-soft` | `#5C3E8B` / `#F0EAF8` | | 8.3 / 7.1 |
| `--teal` / `--teal-soft` | `#175954` / `#E2F3F1` | Mobile wallet only | 8.1 / 7.1 |
| `--grey` / `--grey-soft` | `#4D5054` / `#EEF0F1` | | 8.1 / 7.1 |
Strong colours are used as button fills with white text (amber: dark text). Soft colours are used as backgrounds for badges, cart lines and messages, with the strong colour for their text and icon. The Contrast column shows the measured ratio for button text and for text on the soft colour.

## Always readable
There's no dark mode or light mode. There's **one look, designed to be readable everywhere**: a bright shop floor, a dim corner, an old monitor, or a screen viewed at an angle.
- **Every text and number passes WCAG AAA contrast (7 : 1 or more)**, stricter than the usual AA (4.5 : 1). A unit test reads the token file and fails the build if any pair drops below 7 : 1.
- **The computer's dark mode is ignored.** The page sets `color-scheme: light only`, never uses `prefers-color-scheme`, and the Tauri window theme is fixed. Scrollbars, inputs and the title bar look the same on every computer.
- **Soft off-white background, not pure white**, to cut glare during long shifts. Panels are white so content stands out.
- **No light-grey text anywhere.** Secondary text is still dark (`--text-muted`, 8.1 : 1).
- **No thin fonts.** Normal weight (400) is the lightest used. Amounts and totals are semi-bold (600).
- **Borders are clearly visible** (`--border`), so panels and fields can be told apart without shadows.
- **Minimum text size is 14 px.** Cart lines are 20 px, TOTAL and CHANGE 56 px.
- **The screen stays readable when zoomed** (Ctrl/Cmd + and −, 80–150%), so staff with weaker eyesight can enlarge it. The zoom level is remembered per till.

## Shapes and sizes
- Corner radius: 6 px everywhere. Borders: 1 px. Focus outline: 2 px solid blue (always visible when using the keyboard).
- Buttons on the till: at least 56 px tall, **icon + short label + shortcut key** (e.g. 🟨 ⏸ Hold `F5`). Never icon-only on the till.
- Buttons in the back office: at least 40 px tall. Icon-only buttons are allowed there, but **must** have a tooltip.
- Text sizes: 14 (dense tables), 16 (normal), 20 (cart lines), 28 (headings), 56 (TOTAL / CHANGE).
- Font: the system font (Segoe UI on Windows, SF Pro on Mac). Nothing to bundle.

## Tooltips
- **Every button, icon, status light and form field has a floating tooltip.**
- It appears after **0.4 s of hovering** (or right away when reached with the Tab key) and disappears when the mouse leaves.
- Content: one short plain sentence about **what it does**, plus the **shortcut key** if there is one.
  Example: *"Put this sale aside and serve the next customer. You can bring it back later."* `F5`
- On form fields, the tooltip says **what to type, with an example**. Example: *"Price the customer pays for one unit. Example: 120.00"*
- On **disabled** buttons, the tooltip says **why** it's disabled. Example: *"Add an item first."* or *"Needs an admin. You'll be asked for approval."*
- Style: dark box (`--text` background, white text, 14.7 : 1), 1 px border, no shadow, no animation, max 280 px wide, 14 px text.
- On touch screens (no hover): a long press shows the tooltip. `F1` shows a help overlay with every button's tooltip at once.
- Tooltip text lives in the module's `text.ts` next to the button label, so writing a button without a tooltip is a TypeScript error (the `tip` prop is required on `ActionButton` and `IconButton`).

## Icons
lucide icons, one fixed icon per action across the whole app:
| Action | Icon | Colour |
|---|---|---|
| Pay / Complete | check-circle | green |
| Save | save | green |
| Remove line | trash | red |
| Cancel sale | x-circle | red |
| Void / Refund | rotate-ccw | red |
| Hold | pause-circle | amber |
| Held sales / resume | list-restart | amber |
| Search | search | blue |
| Customer | user | blue |
| Reprint | printer | blue |
| Discount | percent | purple |
| Price override | tag | purple |
| Lock | lock | grey |
| Settings | settings | grey |

## Shared components (`ui/src/shared/ui/`)
`ActionButton` (colour + icon + label + shortcut + tooltip, all required), `IconButton` (icon + tooltip required), `Tip`, `MoneyInput`, `QtyInput`, `Field` (label + input + tooltip + error text), `Dialog`, `SidePanel`, `DataTable` (virtualized), `Badge` (soft colour + icon + word), `StatusLight`, `ColourLegend` (shown in the `F1` help).
These start from shadcn/ui and are **restyled flat**: shadow utilities are removed from the Tailwind theme, so `shadow-*` classes don't exist and can't creep back in.
