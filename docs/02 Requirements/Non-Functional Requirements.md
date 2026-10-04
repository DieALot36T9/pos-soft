---
tags: [requirements, nfr]
---
# Non-Functional Requirements

**Reference machine:** Windows 10 64-bit, 4-core CPU from 2016 or newer, 16 GB RAM, SATA SSD.
Until a Windows PC is available, demo testing happens on the product owner's Mac. All budgets are measured again on Windows in M13.
**Reference data:** 50,000 products, 80,000 barcodes, 5,000 customers, 5,000,000 sale lines (about 3 years at 1,500 sales a day).

## Performance (measured in M13; any regression fails CI benchmarks)
| ID | Requirement | Budget |
|---|---|---|
| NFR-P1 | Barcode scan → line visible in the cart | **≤ 100 ms** (target 30 ms) |
| NFR-P2 | Product search keystroke → results shown | ≤ 150 ms |
| NFR-P3 | Complete sale (save + stock + shift + ledgers) | ≤ 200 ms, not counting printing |
| NFR-P4 | Receipt printing starts after completing the sale | ≤ 1 s |
| NFR-P5 | Cold start to login screen | ≤ 3 s |
| NFR-P6 | Daily summary report | ≤ 1 s |
| NFR-P7 | Any report over one year of data | ≤ 3 s |
| NFR-P8 | Import 50,000 products | ≤ 60 s, with progress shown |
| NFR-P9 | Product list scrolling with 50k rows | 60 fps (virtualized list) |
| NFR-P10 | Memory use during normal work | ≤ 400 MB total |
| NFR-P11 | Installer size (Windows) | ≤ 25 MB |

## Reliability and data safety
| ID | Requirement |
|---|---|
| NFR-R1 | Once "Sale complete" is shown, the sale survives an immediate power cut (`synchronous=FULL`) |
| NFR-R2 | Each workflow is one transaction: either every change is saved or none is |
| NFR-R3 | Carts are saved on every change. After a crash, the in-progress cart is restored |
| NFR-R4 | Hardware failures (printer, scale, drawer) never block or roll back saving a sale |
| NFR-R5 | The database integrity check runs at startup. If it fails, the app offers to restore the latest good backup |
| NFR-R6 | Automatic backups are checked after they're written ([[Req - Backup and Logging]]) |
| NFR-R7 | The database schema upgrade takes a backup first, and refuses to run on a database from a newer app version |
| NFR-R8 | Only one copy of the app can run on a computer (a second launch brings the first window to the front), so two copies can never write to the database at once (`tauri-plugin-single-instance`) |

## Offline and portability
| ID | Requirement |
|---|---|
| NFR-O1 | No feature makes network calls, except to user-configured LAN printers |
| NFR-O2 | Windows 10/11 x64 is the main target. Linux (AppImage/deb) and macOS (dmg) builds pass CI tests |
| NFR-O3 | All files live in the OS app-data folder. Backups go to configurable folders |

## Usability
| ID | Requirement |
|---|---|
| NFR-U1 | A full checkout can be done with the keyboard alone ([[Keyboard Shortcuts]]) |
| NFR-U2 | Touch targets are at least 44×44 px, so the app works on touch screens |
| NFR-U3 | Every error message says what happened and what to do next, in plain English ([[Writing Style]]) |
| NFR-U4 | The UI and receipts are in English. All text lives in one place per module, so a language can be added later without code changes ([[ADR-022 Ready for more languages later]]) |
| NFR-U5 | Numbers, dates and money are formatted using the shop's settings |
| NFR-U6 | All text and numbers have **WCAG AAA contrast (7 : 1 or more)**, checked by an automatic test. There's one fixed look (no dark/light mode), and the computer's dark-mode setting is ignored. Keyboard focus is always visible |
| NFR-U7 | Flat look: no shadows, gradients or animations. One fixed colour code across the whole app ([[Design System]]) |
| NFR-U8 | Every button, icon, status light and field shows a floating tooltip on hover (after 0.4 s) that explains it in one plain sentence, with its shortcut key |
| NFR-U9 | Colour is never the only signal: every coloured item also has an icon and a word |

## Security
| ID | Requirement |
|---|---|
| NFR-S1 | Passwords and PINs are hashed with argon2id. They're never logged or shown |
| NFR-S2 | Every restricted command checks permission in Rust ([[Security Model]]) |
| NFR-S3 | Every restricted action, override, void, price change and drawer opening is audited |
| NFR-S4 | Tauri capabilities are minimal: no shell access, file access limited to the app's folders and paths the user picks |

## Maintainability
| ID | Requirement |
|---|---|
| NFR-M1 | Module crates can't depend on each other (checked in CI) |
| NFR-M2 | Line coverage of 80% or more in module crates; 100% of public money, tax and pricing functions |
| NFR-M3 | Source files have 400 lines or fewer (guideline; going over needs a reason in review) |
| NFR-M4 | No `unsafe` outside `hardware` (Windows spooler FFI only) |
| NFR-M5 | `clippy -D warnings`, `rustfmt`, `eslint`, `prettier` and `tsc --noEmit` are clean |

## Capacity
| ID | Requirement |
|---|---|
| NFR-C1 | Performance budgets hold at the reference data size |
| NFR-C2 | The database works up to 10 GB without design changes |
