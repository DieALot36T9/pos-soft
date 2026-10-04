---
tags: [project, risks]
---
# Risks

| # | Risk | Likelihood | Impact | Mitigation | Owner |
|---|---|---|---|---|---|
| R1 | **No hardware is available yet**, and real printers, scanners and scales behave differently by model and OS | High | High | Build behind fake devices (receipt preview window, fake scale, scan simulator). Print only pictures (one code path) using standard ESC/POS commands. Keep printing through the system print dialog as a fallback. First real-hardware test in **M13**, early enough to fix problems before UAT. Record known-good models in [[Printer Setup]] | Dev + PO |
| R2 | Currency symbols or the logo print wrongly on some printers | Medium | Medium | Receipts are printed as pictures ([[ADR-012 Receipts printed as raster images]]). Checked on a real printer in M13 | Dev |
| R3 | Power loss or crash corrupts data or loses sales | Medium | Critical | SQLite WAL with `synchronous=FULL`, one transaction per workflow, carts saved on every change, backups checked after writing, integrity check at startup, kill-the-process tests | Dev |
| R4 | Tax or rounding mistakes in money | Medium | Critical | Integers only, a single tax function, worked examples as unit tests, property tests (sum of lines = total; return + remainder = original) | Dev |
| R5 | Module boundaries erode | Medium | Medium | Crates can't import each other (enforced by `Cargo.toml` and a CI script), contract tests, the [[Module Rules]] review checklist | Dev |
| R6 | The scope is large for one release ("everything day one") | High | Medium | Dependency-ordered milestones, alpha builds from M6, and the PO accepting each milestone | Dev + PO |
| R7 | AI-written code drifts in style or quietly breaks invariants | Medium | High | Strict Rust and TypeScript, clippy `-D warnings`, the invariant tests in [[Test Strategy]], small files, a project `CLAUDE.md` with the rules, and this vault as memory | Dev |
| R8 | Scale protocols differ between brands | High | Medium | Configurable protocol profiles plus a generic "read a line, parse a number" mode. Manual weight entry always works | Dev |
| R9 | Unsigned installer triggers Windows SmartScreen / macOS warnings | High | Low | Fine during development and testing (click through the warning). Signing is set up **later, once the product works**, before the pilot (M15) | PO |
| R10 | WebView2 missing on older Windows 10 | Low | Medium | The installer includes the WebView2 bootstrapper (Tauri `webviewInstallMode`) | Dev |
| R11 | A shop needs a tax rule the model can't express (compound tax) | Medium | Medium | Tax components exist. Compound tax is listed as out of scope in [[Scope]], and the tax module is isolated so it can be extended | Dev |
| R12 | Admin forgets the password at an offline shop | Medium | High | A recovery code is shown and printed at setup ([[Req - Users and Security]]) | Dev |
| R13 | Database grows large over the years | Low | Medium | Indexes planned per table, performance tests with 5M lines, `PRAGMA optimize` when the app closes. Archiving is only needed far beyond v1 | Dev |
| R14 | Only a Mac is available for testing now, so Windows-only problems are found late | Medium | High | CI builds and tests on Windows on every pull request. A full check on a real Windows 10 PC with the shop's hardware in M13, before UAT ([[ADR-017 Windows first all three OS built]]) | Dev + PO |
| R16 | No barcode scanner to test with | Low | Low | Scanners type like a keyboard, so typing a code and pressing Enter tests the same path. The scan simulator types codes very fast, like a real scanner. A real scanner is checked in M13 | Dev |
| R15 | USB receipt printers are awkward on macOS (often no Mac driver) | Low | Low | For Mac testing, use a network or USB-serial printer if possible. Otherwise use a raw CUPS queue when a printer is available, and add a direct-USB connection only if needed | Dev |
