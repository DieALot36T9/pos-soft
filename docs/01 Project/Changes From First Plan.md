---
tags: [project]
---
# Changes from the first plan

The first plan was a single message. This vault replaces it. Here's what changed and why.

## Decisions from the product owner
| Change | Reason |
|---|---|
| No Core/Soon/Later split. **Everything ships in v1.0** | Product owner's decision |
| Tax is fully configurable (label, rates, inclusive/exclusive, components) | The app is universal |
| Currency, cash rounding, payment methods and denominations are configurable | Same reason. Nothing is specific to one country |
| Windows is the main target; Linux and macOS are still built in CI | Shops run Windows 10 with 16 GB. The original requirement for three operating systems is kept |
| **English only**, but ready for more languages | Bangla isn't needed. Text is kept in one place so languages can be added later ([[ADR-022 Ready for more languages later]]) |
| **Testing on a Mac for now** | The product owner only has a Mac at the moment. Windows is still the shop target, checked in CI and before UAT ([[ADR-017 Windows first all three OS built]]) |
| **Flat, colour-coded UI with tooltips everywhere** | Easy to learn by looking; nothing fancy ([[ADR-021 Flat colour-coded UI with tooltips]]) |
| **Plain language** in docs, code and comments | Everything must be easy to understand ([[Writing Style]]) |
| Milestones replace "2-week sprints" | The developer is Claude, so ordering matters more than calendar time |

## Stack changes
| Was | Now | Reason |
|---|---|---|
| Svelte 5 + shadcn-svelte | **React 19 + TypeScript + Tailwind v4 + shadcn/ui** | This is the UI stack I write most reliably. It has the most examples, and Svelte 5's newer syntax is easy to confuse with Svelte 4. See [[ADR-002 React TypeScript Tailwind for UI]] |
| Hand-written TypeScript types for commands | **ts-rs** generates TS types from Rust structs | The UI and Rust can't drift apart silently ([[ADR-014 Shared types with ts-rs]]) |
| "Bangla as an image if the font lacks it" | **Receipts are always printed as pictures** (English only) | Any currency symbol and the logo print on every printer ([[ADR-012 Receipts printed as raster images]]) |
| Money as paisa, weight as grams | Money in **minor units of the configured currency**; quantities in **thousandths** of the product's unit | Universal currencies (0–3 decimals) and every unit (kg, litre, piece) handled by one type |

## Design fixes (contradictions in the first plan)
| Problem | Fix |
|---|---|
| Sales needed shifts, but shifts were scheduled later | Shifts and checkout are built together in M6 |
| Hardware was tested late although it was the top risk | M1 is a hardware test right after foundations |
| Reports were going to read other modules' tables | Each module publishes **read views** (`<module>_v_*`) as a public contract. Reports read only those views |
| Timeline arithmetic didn't add up | Replaced by dependency-ordered milestones |

## Added to the design
| Addition | Why it can't wait |
|---|---|
| **Append-only stock ledger** (levels are a cache) | Stock history is required, and adding a ledger after go-live means rewriting inventory |
| **Audit log** | Voids, overrides, drawer opens and price changes are where cash goes missing |
| **Snapshots on sale lines** (name, price, tax, cost) | Old receipts and profit reports must not change when a price changes |
| **Gapless invoice numbers for each till; business-day cutoff** | Tax records; shops trade past midnight |
| **Discounts with limits per role** | Every supershop discounts; limits stop abuse |
| **Returns, exchanges, voids** | They happen every day |
| **Cash rounding** | Small coins aren't used in practice in many countries |
| **Admin override at the till** | The cashier stays logged in; the admin approves one action |
| **Recovery code** | An offline shop has no "forgot password" email |
| **ULID ids + till id** | Lets several tills be added later without migrating ids |
| **Carts saved to the database** | A held or in-progress sale survives a crash |
| **Archive, don't delete** | Old sales refer to products |
| **Backups with `VACUUM INTO`, checked, plus a second folder** | Copying a live database file can produce a broken copy; a USB copy survives a disk failure |
| **Errors are always logged** | The switch only controls detailed logs, so there are always logs for support |
| **FTS5 search** | Fast search over 50k products |
| **Custom roles** | A universal app can't assume every shop has only admins and cashiers |
| **No cross-module foreign keys** | Lets each module migrate independently ([[ADR-005 No cross-module foreign keys]]) |
