---
tags: [ui, design]
---
# Design System

**Goal:** a cashier can look at the screen and know what to do **without reading**. The same colour and the same icon always mean the same thing, everywhere in the app. Decision: [[ADR-021 Flat colour-coded UI with tooltips]].

## Rules
1. **Flat and plain.** No shadows, no gradients, no blur, no glass effects, no 3D. Panels are separated by thin 1 px borders (`--border`). Enforced by `ui/src/shared/flat-style.test.ts`.
2. **No movement.** No animations or transitions.
3. **One colour = one meaning** (table below). A colour is never used just for decoration.
4. **Colour is never the only signal.** Every coloured thing also has an icon and a short word, so colour-blind staff (about 1 in 12 men) can still use it.
5. **Things stay in the same place.** The main action is always bottom-right. Back/Cancel is always bottom-left. The scan box is always at the top of the checkout screen.
6. **Every button and field explains itself** ([[#Tooltips & Field Descriptions]]).
7. **Calm on the eyes.** Soft off-white background (`#F6F7F5`), white panels (`#FFFFFF`), dark grey text (`#1F2328`), muted colours, no bright neon.
8. **Big where it matters.** TOTAL and CHANGE are the largest text on screen (`56px`).
9. **Numbers line up.** Every amount is right-aligned and uses fixed-width digits (`tabular-nums`).
10. **One fixed look, always readable** (no dark or light mode). The app ignores the computer's dark-mode setting and always looks the same.

## Colour code
| Colour | Means | Used for |
|---|---|---|
| 🟩 **Green** | Money coming in · finish · OK | Pay, Complete sale, Save, cash payment, Cash in, Open shift, device connected, weight stable |
| 🟥 **Red** | Remove · cancel · money going out · problem | Remove line, Cancel sale, Void, Refund, Cash out, Archive/Delete, error messages, negative stock |
| 🟨 **Amber** | Wait · paused · needs attention | Hold sale, Held sales, low stock, near expiry, warnings, "needs admin approval" |
| 🟦 **Blue** | Find · look up · information | Search, Customer, Reprint, Reports, info messages, the selected cart line |
| 🟪 **Purple** | Price changes | Discount, promotion lines ("Buy 2 get 1"), price override, price lists |
| ⬜ **Grey** | Neutral · go back · settings | Back, Close, Lock, Settings, disabled buttons |

### Payment method colours
| Method | Colour | Icon |
|---|---|---|
| Cash | Green | banknote |
| Card | Blue | credit card |
| Mobile wallet | Teal | phone |
| Customer credit (baki) | Amber | notebook |
| Loyalty points | Purple | star |

## Colour tokens (`ui/src/shared/theme.css`)
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

Every text/background pair is verified by `ui/src/shared/theme.contrast.test.ts` to maintain at least 7 : 1 WCAG AAA contrast.

## Tooltips & Field Descriptions
- **Every button, icon, and status badge has a floating tooltip** (`Tip.tsx` / `tooltip.tsx`) that appears after 0.4 s of hovering (or on keyboard focus) above the control (`side="top"`, `pointer-events-none`).
- **Form fields never hide their instructions inside hover-only popups:** `Field.tsx` and `SelectField.tsx` display their plain-English `tip` description persistently right under the field label so staff can read what to type (and see an example) without a tooltip covering the input box.
- **Global `F1` Help Guide (`HelpDialog.tsx`):** Pressing `F1` on the till (or clicking `Help F1` in the till header) opens a full colour-code legend and keyboard shortcut reference.

## Shared components (`ui/src/shared/ui/`)
- **Core primitives:** `ActionButton.tsx` (colour + icon + label + shortcut + tooltip), `IconButton.tsx`, `Field.tsx` (label + persistent description + input + error), `SelectField.tsx` (label + persistent description + select + error), `CheckBox.tsx`, `Alert.tsx`, `Badge.tsx`, `Dialog.tsx`, `Tip.tsx`, `tooltip.tsx`, `HelpDialog.tsx`.
- **Workspace & POS Kit (`PosKit.tsx`):** `KitButton`, `PageHeader` (breadcrumbs + title + description + actions), `KitField`, `KitBadge`, `KitDrawer` (right-hand slide-over drawer for master-detail quick view and cart line editing), and `KitNumpad` (touch/mouse keypad for PINs, quantities, discounts, and prices).
