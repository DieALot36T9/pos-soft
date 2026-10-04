---
tags: [adr]
status: accepted
---
# ADR-002 React, TypeScript and Tailwind for the UI

## Context
The product owner asked Claude (the developer) to choose the stack it's most comfortable with, and asked whether StyleX is good for AI-written code. The first plan proposed Svelte 5.

## Decision
**React 19 + TypeScript (strict) + Vite + Tailwind CSS v4 + shadcn/ui**, with TanStack Query/Table/Virtual, React Router, react-hook-form. shadcn components are restyled flat ([[ADR-021 Flat colour-coded UI with tooltips]]).

## Reasons
- React + TS + Tailwind + shadcn is the stack Claude writes most reliably: it has the most examples and the fewest syntax mix-ups. Svelte 5's runes are newer, and Svelte 4 and 5 syntax get confused.
- Tailwind puts styles next to the markup, so a whole component is readable in one file, with no runtime cost.
- shadcn/ui components are copied into the repo, so we own them and can change them.
- React Compiler handles memoization automatically. Performance is more than enough for a cart of 100 lines and virtualized lists of 50k.

## Why not StyleX
Fewer examples (so more invalid AI-written code), strict static-analysis authoring rules that fail only at build time, an extra build plugin, and no component library on top of it. Its main benefit, scaling CSS across very large teams, doesn't apply here. Details in [[Tech Stack#Why not StyleX?]].

## Consequences
- ✅ Fast iteration with Vite hot reload and the mock bridge.
- ⚠️ React is bigger than Svelte's output. This is mitigated by route splitting; the till-mode bundle must stay under 300 KB gzipped.
