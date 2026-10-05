---
tags: [testing]
---
# Test Strategy

**Tests are written first** ([[TDD Workflow]]). Each requirement ID (e.g. `CHK-08`) is covered by at least one test that names it.

## Test levels
| Level | Where | Tool | What | Runs |
|---|---|---|---|---|
| Unit | Next to the code in each crate | `cargo nextest` | Pure functions: money, tax, pricing engine, barcode parsing, layouts | Every push |
| Property | Module crates | `proptest` | Invariants over random inputs (see below) | Every push |
| Module (DB) | `crates/<m>/tests/` | `cargo nextest` + in-memory SQLite | Module functions with its own migrations | Every push |
| Contract | `crates/<m>/tests/contract.rs` | `cargo nextest` | The public API and views behave as the module page says | Every push |
| Workflow integration | `crates/app/tests/` | `cargo nextest` | Multi-module use cases in one transaction, failure injection | Every push |
| Golden images | `crates/printing/tests/` | PNG compare | Receipts, reports, labels pixel-identical to approved images | Every push |
| UI unit | `ui/src/**/*.test.tsx` | Vitest + Testing Library | Components, keyboard handling, formatting, error mapping, **colour contrast of every token pair ≥ 7 : 1** | Every push |
| UI e2e (mock) | `ui/e2e/` | Playwright + mock bridge (`pnpm e2e` builds the screens in mock mode and serves them with `vite preview`; Chromium at 1366×768; `?shop=fresh` starts the fake shop empty, `?shop=signed_out` set up but signed out) | Full screen flows: printer and devices (C-M1), setup wizard, settings, tax rates, backups (C-M2), later checkout, payment, setup wizard, import | Every push (Linux) |
| App smoke | CI Windows + Linux | tauri-driver | Packaged app starts, setup with fixtures, one sale completes | Every merge to main |
| Performance | `xtask bench` | criterion-style timing on the reference database | NFR-P budgets | Nightly + before release |
| Durability | `xtask powercut` | Kill a child process during writes, reopen, check | NFR-R1/R2 | Nightly + before release |
| Hardware | Manual checklist | Real devices | Printer, drawer, scale, scanner, labels | M13 (first real hardware), M14 |
| UAT | Real shop setup | [[UAT Script]] | A full trading day | M14 |

## Invariants (checked by property tests and the scripted-day test)
1. Each inventory level and batch cache = Σ its ledger rows.
2. Customer and supplier balances = Σ their ledgers.
3. For every sale: Σ line gross = header gross; header total = gross + rounding; Σ payments − change = total.
4. Invoice numbers per till are gapless and increasing.
5. Σ returns of a sale line ≤ the line, and equals it exactly when fully returned.
6. Shift expected cash = float + Σ cash movements.
7. A voided sale's stock, shift and customer effects net to zero.
8. Ledger tables never change (UPDATE/DELETE raise).

## Test data
See [[Test Data]]. Every Rust test builds its database from migrations. There are **no shared fixture files that tests change**.

## Hardware doubles
`MockPrinter` records bytes; `MockScale` replays readings; `FixedClock` controls time. Workflows take these through `Ctx`, so no test needs a device.

## Coverage targets
- Module crates ≥ 80% lines (`cargo llvm-cov`), with 100% of public functions in `core`, `tax`, `pricing`.
- Every Tauri command has at least one workflow or permission test.

## CI gates (merge is blocked if any fail)
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run` (Linux on every push, Windows on every pull request, and a Mac build when a pull request has the `build-mac` label or a version tag is pushed) · `cargo deny check` · `cargo xtask gen-types --check` · `pnpm lint` · `pnpm typecheck` · `pnpm test` · `pnpm e2e`
