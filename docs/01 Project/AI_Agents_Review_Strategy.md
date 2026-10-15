---
tags: [project, ai-review]
---
# AI Agents Review Strategy

This document outlines how to divide the Supershop POS project into independent parts so that multiple AI agents can simultaneously review code, find bugs, and verify requirements without colliding.

## System Context
- **Architecture:** Offline-first desktop app. Tauri 2 (Rust backend) + React 19 (Webview frontend).
- **Data:** SQLite (with WAL mode), handled via `rusqlite`.
- **Backend structure:** 19 isolated modules (crates). Modules communicate **only** via the `app` crate workflows. No module directly imports another module (except `core`).
- **Frontend structure:** Strict separation of UI logic and Business logic (all business logic in Rust). Uses standard Tailwind v4 and Primitives.
- **Rule of Thumb:** Focus AI reviews heavily on module boundaries, pure function tests, data ownership (no cross-module Foreign Keys), and append-only ledger integrity.

---

## Part 1: Core Framework, DB & App Shell (Foundations)
**Scope:** `crates/core/`, `crates/db/`, `crates/app/`, `crates/settings/`
**Target Areas:**
- **`core`:** Pure functions for `Money`, `Qty`, rounding. *Check for overflow panics (should be `Result`), strict immutable constraints.*
- **`db`:** SQLite initialization, WAL pragmas, migrations runner, backup timers. *Verify offline-durability, power-cut resistance, missing table drops in migrations.*
- **`app`:** Tauri commands, permission guards, workflows (transactions). *Ensure transactions don't leak, hardware triggers happen **after** DB commits, session guards apply correctly.*
- **`settings`:** Global typed settings. *Check missing validation for default configs.*

## Part 2: Frontend Foundation & Design System (UI Core)
**Scope:** `ui/src/shared/`, `ui/src/app/`, `ui/src/main.tsx`, `ui/package.json`
**Target Areas:**
- **Bridge (`shared/bridge/`):** IPC to Tauri. *Check error handling, timeouts, fake bridge fidelity.*
- **UI Toolkit (`shared/ui/` & `theme.css`):** Flat style, AAA contrast. *Ensure components don't hold business state, strict CSS constraint tests.*
- **State & Routing:** `AppFrame.tsx`, `TillFrame.tsx`. *Check memory leaks in screen switching, unhandled async promise rejections.*
- **i18n:** `react-i18next` integration. *Verify typed keys, missing translations.*

## Part 3: Security & Accountability
**Scope:** `crates/auth/`, `crates/audit/`
**Target Areas:**
- **`auth`:** Users, roles, PIN/passwords, overrides. *Check for timing attacks on passwords (Argon2 setup), missing admin checks, token leaking.*
- **`audit`:** Append-only log. *Verify SQLite triggers effectively block UPDATE/DELETE on audit tables. Ensure no sensitive data (passwords) ends up in JSON details.*

## Part 4: Catalog, Tax & Pricing
**Scope:** `crates/catalog/`, `crates/tax/`, `crates/pricing/`, `crates/barcode/`
**Target Areas:**
- **`catalog`:** Products, variants, FTS5 search. *Check full-text search trigger synchronization, cyclic parent-child variants.*
- **`pricing`:** The Pricing Engine (`price_cart`). *This is critical. Verify best-promotion logic, edge cases around manual vs. automatic discounts, zero-price line handling.*
- **`tax`:** Tax rates and inclusive/exclusive calculations. *Review rounding rules for tax allocation across multiple lines.*
- **`barcode`:** Odoo-style scale labels. *Check bounds on string parsing, EAN check-digit math.*

## Part 5: Sales & Checkout (Core Transaction Flow)
**Scope:** `crates/sales/`, `crates/shifts/`, `ui/src/modules/checkout/`
**Target Areas:**
- **`sales`:** Carts, payments, completed sales snapshots, returns, voids. *Verify that completed sales triggers block UPDATE/DELETE. Check for race conditions in cart saving vs. checkout execution.*
- **`shifts`:** Cash drawer totals, float, expected cash. *Check for edge cases on crossing midnight, duplicate shift opens.*
- **UI Checkout:** `CheckoutPage.tsx`. *Verify 60/40 split layout constraints, focus stealing in `ScanBox`, debounced API spam.*

## Part 6: Inventory & Purchasing (Back-office Operations)
**Scope:** `crates/inventory/`, `crates/purchasing/`
**Target Areas:**
- **`inventory`:** Stock ledger, batches, expiry (FEFO). *Check for race conditions on stock updates, negative stock propagation, ledger aggregate invariant (sum of rows = total stock).*
- **`purchasing`:** Suppliers, POs, payables. *Check state machine on POs (Draft -> Placed -> Received), supplier debt aggregation bugs.*

## Part 7: Customers & Reports (Data & Insights)
**Scope:** `crates/customers/`, `crates/reports/`, `crates/io/`
**Target Areas:**
- **`customers`:** Points & credit ledger. *Ensure negative credit limits are respected, point calculation truncations.*
- **`reports`:** Massive read-only SQL over `_v_` views. *Check for SQL injection (if dynamic strings are used), index usage (table scans), date boundary conditions (timezone offsets).*
- **`io`:** CSV/XLSX processing. *Check OOM (Out of Memory) vulnerabilities on large files, CSV escaping on malicious input.*

## Part 8: Hardware & Printing
**Scope:** `crates/hardware/`, `crates/printing/`
**Target Areas:**
- **`hardware`:** Serial, Network, Windows Spooler. *Check for thread panics on disconnected devices, Windows FFI (`unsafe`) memory leaks, timeout handling on offline network printers.*
- **`printing`:** Layouts, Raster imaging. *Check for out-of-bounds pixel drawing, memory spikes when generating long receipts, TSPL/ESC-POS byte alignment.*

---

## Instructions for AI Agents
When assigning an AI agent to one of these parts, provide them with:
1. The path to this document and their specific assigned Part.
2. The overarching rules (`docs/03 Architecture/Module Rules.md` and `docs/05 Modules/Module - <module>.md`).
3. Instructions to only read files within their specified scope and report any logic flaws, security vulnerabilities, or violations of architectural constraints (like cross-module DB joins).
