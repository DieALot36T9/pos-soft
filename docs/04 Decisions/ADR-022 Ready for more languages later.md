---
tags: [adr]
status: accepted
---
# ADR-022 English now, ready for more languages later

## Context
The product owner doesn't need Bangla (or any other language) now, but wants to keep the door open for languages in the future. Adding translation machinery everywhere now would add code nobody uses yet. Building with English typed straight into components would make a future translation a rewrite.

## Decision
v1.0 ships **English only**, built so a new language is **added files, not changed code**:
| Part | How it stays ready |
|---|---|
| Screen text | All labels, tooltips and messages live in each module's `text/en.ts`, never inline in components. `en.ts` defines a TypeScript type; another language file must match it exactly, or the build fails. A tiny `useText()` hook picks the file by the `language` setting |
| Error messages | Rust sends error **codes**, never sentences. The UI turns codes into words using `shared/text/en/errors.ts` |
| Receipts and printouts | Every word printed (e.g. "Subtotal", "Change") comes from one `ReceiptLabels` struct filled by `app`, not hard-coded in the layouts |
| Receipt drawing | cosmic-text can draw any script. Adding a language means bundling its font (e.g. Noto Sans Bengali) |
| Numbers, money, dates | Already formatted from the shop's settings (separators, digit grouping, symbol position) |
| Data | SQLite stores any text (UTF-8). Search already handles accented and non-Latin letters |
| Settings | A `language` setting exists (only `en` today). The language picker stays hidden until a second language exists |

## Consequences
- ✅ No translation library, no unused code paths, no extra fonts today.
- ✅ A future language = translate `text/en.ts` files + `ReceiptLabels` + add a font. TypeScript lists anything missed.
- ⚠️ Writers must never put screen text directly in components. ESLint's `react/jsx-no-literals` rule enforces it.
- ⚠️ A per-product second-language name was removed from the catalog. If a future language needs product names in two languages, that's a new catalog field (one migration).
