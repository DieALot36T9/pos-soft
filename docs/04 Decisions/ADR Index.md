---
tags: [adr]
---
# Architecture Decision Records

Short notes about decisions that are expensive to reverse. Template: `Templates/Template - ADR`.
Status values: Proposed · **Accepted** · Superseded by ADR-xxx.

| ADR | Decision | Status |
|---|---|---|
| [[ADR-001 Tauri 2 as desktop shell]] | Tauri 2 with a Rust core | Accepted |
| [[ADR-002 React TypeScript Tailwind for UI]] | React 19 + TS + Tailwind v4 + shadcn/ui (not Svelte, not StyleX) | Accepted |
| [[ADR-003 SQLite with rusqlite]] | Bundled SQLite, WAL, synchronous FULL, plain SQL | Accepted |
| [[ADR-004 One crate per module]] | 19 crates; only `app` connects them | Accepted |
| [[ADR-005 No cross-module foreign keys]] | References across modules are plain ids checked by workflows | Accepted |
| [[ADR-006 Integer money in minor units]] | `Money(i64)`, configurable currency digits | Accepted |
| [[ADR-007 Quantities in thousandths]] | `Qty(i64)` for every unit | Accepted |
| [[ADR-008 Tax model]] | One rate per product, bps, inclusive/exclusive, per-line rounding, display components | Accepted |
| [[ADR-009 ULID ids and till identity]] | ULIDs everywhere, till id on documents | Accepted |
| [[ADR-010 Append-only ledgers]] | Stock, points, credit, payables, audit are ledgers; balances are caches | Accepted |
| [[ADR-011 Snapshots on sale lines]] | Sales copy names, prices, tax, cost | Accepted |
| [[ADR-012 Receipts printed as raster images]] | Receipts printed as pictures (any symbol, logo) | Accepted (confirm on a real printer in M13) |
| [[ADR-013 A4 and PDF output through the webview]] | HTML → system print dialog for A4/PDF | Accepted |
| [[ADR-014 Shared types with ts-rs]] | Generate TS types from Rust | Accepted |
| [[ADR-015 Permissions enforced in Rust]] | Command guard + single-use override tokens | Accepted |
| [[ADR-016 Business day cutoff]] | Business date = local date of (now − cutoff) | Accepted |
| [[ADR-017 Windows first all three OS built]] | Windows for shops, Mac for testing now, all three built | Accepted |
| [[ADR-018 Moving average cost]] | Moving average by default; last cost as an option | Accepted |
| [[ADR-019 Single best promotion per line]] | No stacking of promotions | Accepted |
| [[ADR-020 Negative stock allowed by default]] | Never block a sale for wrong stock data (setting) | Accepted |
| [[ADR-021 Flat colour-coded UI with tooltips]] | Flat look, fixed colour meanings, tooltips everywhere | Accepted |
| [[ADR-022 Ready for more languages later]] | English only, text kept in one place for future languages | Accepted |
| [[ADR-024 Mantine for the UI]] | Mantine components and standard POS/admin layouts; React stays, no Rust UI | Proposed |
| [[ADR-023 Reuse before building]] | Standards and libraries first; own code only where nothing fits; permissive licences only | Accepted |
