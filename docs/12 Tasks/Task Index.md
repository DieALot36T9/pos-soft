---
tags: [task]
---
# Task Index

Every piece of work, as a brief that a **fresh cloud session** can do alone. How to run them: [[Building with Cloud Sessions]].

## Phases A to C
A0 → A1, each alone → all B tasks in parallel → C tasks one after another, in milestone order.

| Brief | What | Needs merged first | When |
|---|---|---|---|
| [[Task - A0 Foundations]] | Workspace, all crates, core, app shell, CI, cloud setup | — | ✅ done |
| [[Task - A1 Reuse Alignment]] | Swap hand-written parts for standard tools (cargo-deny, eslint-plugin-boundaries, i18next, num-format, single-instance), clear CI warning | A0 | ✅ done |
| [[Task - B-db]] | `db` crate | A0 | parallel |
| [[Task - B-settings]] | `settings` crate | A0 | ✅ done |
| [[Task - B-auth]] | `auth` crate | A0 | parallel |
| [[Task - B-audit]] | `audit` crate | A0 | parallel |
| [[Task - B-tax]] | `tax` crate | A0 | parallel |
| [[Task - B-catalog]] | `catalog` crate | A0 | parallel |
| [[Task - B-pricing]] | `pricing` crate | A0 | parallel |
| [[Task - B-barcode]] | `barcode` crate | A0 | parallel |
| [[Task - B-inventory]] | `inventory` crate | A0 | parallel |
| [[Task - B-sales]] | `sales` crate | A0 | parallel |
| [[Task - B-shifts]] | `shifts` crate | A0 | parallel |
| [[Task - B-customers]] | `customers` crate | A0 | parallel |
| [[Task - B-purchasing]] | `purchasing` crate | A0 | parallel |
| [[Task - B-reports]] | `reports` crate | A0 | after sales, inventory, catalog, shifts, customers, purchasing, audit |
| [[Task - B-io]] | `io` crate | A0 | parallel |
| [[Task - B-printing]] | `printing` crate | A0 | parallel |
| [[Task - B-hardware]] | `hardware` crate | A0 | parallel |
| [[Task - C-M1]] | Receipt pictures and fake hardware | B-printing, B-hardware, B-barcode | in order |
| [[Task - C-M2]] | Settings, setup wizard and backups | C-M1, B-settings, B-tax, B-auth, B-db | in order |
| [[Task - C-M3]] | Login, roles, admin approval and audit | C-M2, B-auth, B-audit | in order |
| [[Task - C-M4]] | Products | C-M3, B-catalog, B-barcode | in order |
| [[Task - C-M5]] | Stock | C-M4, B-inventory | in order |
| [[Task - C-M6]] | Shifts and checkout | C-M5, B-sales, B-shifts, B-pricing, B-tax | in order |
| [[Task - C-M7]] | Receipts and devices at the till | C-M6, B-reports | in order |
| [[Task - C-M8]] | Prices, promotions and customers | C-M7, B-pricing, B-customers | in order |
| [[Task - C-M9]] | Returns and voids | C-M8 | in order |
| [[Task - C-M10]] | Purchasing | C-M9, B-purchasing | in order |
| [[Task - C-M11]] | Reports | C-M10, B-reports, B-io | in order |
| [[Task - C-M12]] | Import/export, labels and shelf tags | C-M11, B-io | in order |

## Phase D: with people and hardware
| Step | What | Needs | Who |
|---|---|---|---|
| D-M13 | Hardening + Windows PC + first real hardware | C-M12 | Product owner + Windows PC + devices |
| D-M14 | UAT: a simulated trading day ([[UAT Script]]) | D-M13 | Product owner + a cashier |
| D-M15 | Release 1.0 and pilot (signing set up here) | D-M14 | Product owner |

## Model and effort
**Every task uses Opus 5.5.** Only the effort level changes, and **medium is the lowest** used. Each brief shows both at the top.
- **High:** B-pricing, B-inventory, B-sales, B-printing, C-M6, C-M9. Tricky rules where a subtle mistake costs money or data.
- **Medium:** everything else.
If a medium session gets stuck or its tests keep failing, start a **fresh** session at high rather than pushing the same one.

## Status
Each brief has a `status` at the top: `ready` → `in progress` → `in review` → `done` (or `waiting` while its dependencies aren't merged). The session updates it in its pull request.
