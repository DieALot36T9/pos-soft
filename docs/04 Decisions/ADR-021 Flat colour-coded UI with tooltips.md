---
tags: [adr]
status: accepted
---
# ADR-021 Flat, colour-coded UI with tooltips

## Context
The product owner wants a UI with "nothing fancy": no shadows or effects. It should be solid, reliable and easy to guess, colour-coded so a cashier can work by looking rather than reading, pleasing to the eyes, and with floating tooltips on hover so people learn on their own.

## Decision
- A **flat** look: borders instead of shadows, no gradients, blur or animations (except a 0.3 s flash on a new cart line).
- A **fixed colour code**: green = money in / finish, red = remove / money out, amber = wait / attention, blue = find / info, purple = price changes, grey = neutral. Full table in [[Design System]].
- Colour always comes with an **icon and a word**.
- **Every** button, icon, status light and field has a floating tooltip (0.4 s hover delay) with a plain sentence and the shortcut key. The `tip` prop is required by TypeScript.
- **No dark or light mode.** One fixed look that ignores the computer's dark-mode setting, with every text pair at 7 : 1 contrast or more (WCAG AAA), checked by a test. System font.

## Consequences
- ✅ Staff learn the till fast: "green to pay, red to remove, amber to hold".
- ✅ Less UI code (no themes, no animations), so fewer UI bugs.
- ✅ Always readable, whatever the lighting or the computer's settings.
- ✅ Tooltips double as built-in training and reduce the need for the manual.
- ⚠️ Writing a tooltip for everything takes time. This is accepted, and the required `tip` prop enforces it.
- ⚠️ Colour meanings must never be reused for decoration. Checked in review against the [[Design System]].
