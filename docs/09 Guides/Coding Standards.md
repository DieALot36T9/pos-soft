---
tags: [guide, standards]
---
# Coding Standards

The project rule: **simple code that's easy to understand, while staying fast and solid.** Read [[Writing Style]] too. It covers comments, names and screen text.

## General
- Short files (400 lines or fewer), short functions (50 lines or fewer), clear full-word names.
- Write the obvious version first. Make it faster only when a measurement shows it's needed.
- Every public function has a one-line plain-English comment. Other comments explain **why**.
- Use the words from the [[Glossary]]: you `cancel` a cart, `void` a sale and `return` items. Never mix them up.

## Rust
- Plain structs, enums and functions. Use traits only where we swap real and fake versions (`Clock`, `RawPrinter`, `Scale`).
- **Not allowed** in module crates: our own macros, generic "frameworks", `async`, clever iterator chains that need a second read.
- Errors: one error enum per module, turned into `AppError`. Use `?`. No `unwrap`/`expect` outside tests and `main`.
- Money, quantities and rates only through `Money`, `Qty` and `Bps`. Never floating-point numbers for these.
- SQL is written as plain text in the module that owns the tables, with named parameters (`:product_id`).
- Keep "load data" functions apart from "apply the rule" functions, so rules can be tested without a database.
- Formatting: `cargo fmt`. Lints: `cargo clippy -- -D warnings`.

## TypeScript / React
- `strict` mode. No `any`. No `!` (non-null) without a comment saying why it's safe.
- Function components with hooks. No class components, no global state library.
- Components 200 lines or fewer.
- Talk to Rust only through the module's `api.ts`. Use types only from `generated/`.
- Style with Tailwind classes using the colour tokens from the [[Design System]]. **No shadows, gradients or animations.**
- Every button and field gets a tooltip (`tip` prop, required).
- **Screen text** (labels, tooltips, messages) lives in the module's `text.ts`, never written inline in components. This keeps all wording in one place, easy to review and easy to translate later.
- Formatting: Prettier. Lints: ESLint (React, hooks, import rules).

## Changes and commits
- Small changes with one purpose each. The commit message says what changed and why, and names the requirement IDs.
- A change to a module also updates its page in `05 Modules/` ([[Definition of Done]]).
