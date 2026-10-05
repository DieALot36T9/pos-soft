---
tags: [ui]
---
# Keyboard Shortcuts

One list (`ui/src/shared/keyboard/`) defines every shortcut, wired up with **react-hotkeys-hook** (scopes keep checkout keys from clashing with dialogs), so there are no conflicts and the help overlay (`F1`) is generated from it. Every button's tooltip also shows its shortcut, so staff learn the keys just by hovering.

## Checkout
| Key | Action |
|---|---|
| `Enter` | Submit input (scan/SKU/search). On an empty input: open payment |
| `F1` | Help overlay |
| `F2` | Focus search |
| `F3` | Customer lookup |
| `F4` | Quantity of the selected line |
| `n*` then scan | Add n of the next item (e.g. `3*`) |
| `+` / `-` | Quantity +1 / −1 on the selected line (only when the input is empty) |
| `F5` | Hold sale |
| `F6` | Held sales |
| `F7` | Line discount · `Shift+F7` invoice discount |
| `F8` | Price override |
| `F9` | Cancel sale |
| `F10` | Payment |
| `F11` | Shift menu (cash in/out, X report, close) |
| `F12` | Lock |
| `Del` | Remove the selected line |
| `↑` / `↓` | Select line / move in lists |
| `Ctrl+P` | Reprint last receipt |
| `Ctrl+R` | Return / exchange |
| `Ctrl+L` | Price list switch |
| `Esc` | Close dialog / clear input |

## Payment dialog
| Key | Action |
|---|---|
| `↑` / `↓` | Move between payment methods |
| `F1`–`F4` in dialog | Quick cash amounts |
| `Enter` | Complete (when fully paid) |
| `Esc` | Back to cart |

## Everywhere
| Key | Action |
|---|---|
| `Ctrl+K` | Command palette (jump to any back-office screen) |
| `Ctrl+S` | Save the current form |
| `Ctrl+N` | New item in the current list |
| `Ctrl+F` | Focus the list search |
| `Ctrl+Shift+S` | Scan simulator (dev builds only, HW-10) |

Function keys are used because they never conflict with scanner input (scanners type digits, letters and Enter only).
