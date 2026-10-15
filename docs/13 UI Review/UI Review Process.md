---
tags: [ui, process]
---
# UI Review Process

How we review and verify screens in `pos-soft`.

## Where the screens live
All screens live directly in **`ui/src/`** and run against either:
- **The Tauri Rust backend:** `pnpm tauri dev`
- **The browser mock bridge (no hardware or Rust compile needed):** `pnpm --filter ui dev:mock`

## Automated Quality Gates
Every UI change must pass all four automated checks in `ui/`:
1. `pnpm --filter ui typecheck` — Strict TypeScript verification across all components, `ts-rs` bindings (`ui/src/generated/`), and typed `i18next` keys (`ui/src/app/i18next.d.ts`).
2. `pnpm --filter ui test` — 36 Vitest suites (791 tests), including:
   - `ui/src/shared/theme.contrast.test.ts` (verifies ≥ 7:1 WCAG AAA contrast on all 19 colour pairs in `theme.css`)
   - `ui/src/shared/flat-style.test.ts` (verifies zero shadows, gradients, blurs, animations, or transitions across all `.tsx` and `.css` files)
   - `ui/src/app/moduleBoundaries.test.ts` (verifies strict module isolation via `eslint-plugin-boundaries`)
3. `pnpm --filter ui lint` — Zero ESLint warnings/errors and Prettier formatting compliance.
4. `pnpm --filter ui e2e` — Playwright end-to-end user journeys against the mock bridge.

## Review Checklist (for any screen change)
- **First look:** Can you tell what the page is for in 3 seconds from its header description and primary action button?
- **Navigation:** Can you reach it from the grouped sidebar or `Ctrl+K` Command Palette?
- **Colours:** Does every button and badge follow the [[Design System#Colour code]]?
- **Self-explaining fields:** Does every field show its plain-English description under the label without covering the input box?
- **Keyboard & touch speed:** Can common till tasks be completed with keyboard shortcuts (`F1`–`F12`) or touch tiles/numpads?
