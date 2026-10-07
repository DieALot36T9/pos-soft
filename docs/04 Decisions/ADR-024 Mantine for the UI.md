---
tags: [adr]
status: proposed (confirmed when page P00 is approved)
---
# ADR-024 Mantine for the UI

## Context
After M12 the product owner tried the app: the features are good, but the UI is not. The screens were hand-built: apart from the tooltip and shortcut helpers there's no component library, data table, form library or routing. Each session made its own pieces, so screens are inconsistent and cluttered (e.g. a wall of 17 buttons on the till). The owner asked for a standard, battle-tested, easy-to-navigate UI with quality-of-life improvements, and asked whether the frontend should be written in Rust instead.

## Decision
- Keep React + TypeScript for the screens; Rust keeps all business logic. **Not** a Rust UI framework: those (Leptos, Dioxus, Slint, egui) lack mature component sets (data tables, date pickers, searchable selects, command palettes), so even more would be hand-built, iteration would be slower, and every existing screen would be lost.
- Use **Mantine** (MIT; version 9 at the time of writing) for every component: `@mantine/core`, `hooks`, `form`, `dates`, `notifications`, `modals`, `spotlight`, plus **mantine-datatable** (MIT) for tables.
- Theme it to keep our rules: flat (no shadows), no animations, forced light scheme, our colour meanings, 7:1 contrast ([[UI Guidelines#2. Theme]]).
- Use the standard layouts: an admin layout for the back office and the split product/order layout of mainstream POS systems for the till ([[UI Guidelines#3. Layouts]]).
- Redesign through reviewed mocks first, one page at a time ([[UI Review Process]]).

## Consequences
- ✅ Consistent screens from one tested component set; far less hand-built UI code.
- ✅ Built-in accessibility and keyboard support from Mantine.
- ⚠️ Replaces shadcn/ui and Tailwind-styled components ([[ADR-002 React TypeScript Tailwind for UI]] is superseded for components; React + TypeScript stay). Tailwind may stay for small layout utilities or be dropped; decided during P00.
- ⚠️ Every screen is rebuilt once. The Rust side doesn't change.

## Alternatives
- **Ant Design:** very complete and common in enterprise admin apps, denser and more "office software" in feel, heavier to restyle flat.
- **Fix the shadcn/ui setup:** possible, but shadcn components are copied into the repo and edited per screen, which is how inconsistency crept in.
- **Rust UI framework:** see Decision.
