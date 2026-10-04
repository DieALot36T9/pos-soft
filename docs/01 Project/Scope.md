---
tags: [project, scope]
---
# Scope (v1.0)

**Everything below ships in v1.0.** The **Source** column shows where each item came from: *Owner* means your original list, and *Added* means the design needs it (reasons are in [[Changes From First Plan]]).

## Checkout
| Feature | Source | Module | Requirements |
|---|---|---|---|
| Scan a barcode to add an item (scanner acts as a keyboard) | Owner | sales, catalog, barcode | [[Req - Checkout]] |
| Search by name, code, SKU, category or brand | Owner | catalog | [[Req - Checkout]] |
| Change quantity, remove a line, cancel the whole sale | Owner | sales | [[Req - Checkout]] |
| Weighed items: manual weight or a connected scale | Owner | sales, hardware | [[Req - Checkout]] |
| Scale-label barcodes with weight or price inside (Odoo-style rules) | Owner | barcode | [[Req - Checkout]] |
| Change due | Owner | sales | [[Req - Payments]] |
| Hold/park and resume (survives restarts) | Owner | sales | [[Req - Checkout]] |
| Receipts on 58 mm/80 mm printers, reprint | Owner | printing, hardware | [[Req - Hardware and Printing]] |
| Payments: cash, card, mobile wallet, split, customer credit, loyalty points | Added | sales | [[Req - Payments]] |
| Line and invoice discounts, with limits per role | Added | pricing, auth | [[Req - Pricing and Promotions]] |
| Cash rounding (e.g. to whole currency units) | Added | core, settings | [[Money Tax and Rounding]] |
| Returns, exchanges, voiding a completed sale | Added | sales | [[Req - Returns and Voids]] |
| Gapless invoice numbers and a business day | Added | sales, settings | [[Req - Payments]] |

## Catalog
| Feature | Source | Module | Requirements |
|---|---|---|---|
| Fields: name, barcodes, SKU, category, brand, unit, cost, sale price, tax rate | Owner | catalog | [[Req - Catalog]] |
| Units: piece, kg, gram, litre, pack, plus custom units | Owner | catalog | [[Req - Catalog]] |
| Variants (size, flavour) | Owner | catalog | [[Req - Catalog]] |
| Bundles and combos: kit products and multi-buy promotions (3-for-2) | Owner | catalog, pricing | [[Req - Catalog]], [[Req - Pricing and Promotions]] |
| Generate and print barcode labels | Owner | barcode, printing | [[Req - Catalog]] |
| Shelf price tags | Owner | printing | [[Req - Catalog]] |
| Product images | Owner | catalog | [[Req - Catalog]] |
| Price lists (retail, wholesale, member) | Owner | pricing | [[Req - Pricing and Promotions]] |
| Archive instead of delete; price change history | Added | catalog | [[Req - Catalog]] |

## Inventory
| Feature | Source | Module | Requirements |
|---|---|---|---|
| Live stock that goes down on each sale | Owner | inventory | [[Req - Inventory]] |
| Adjustments with a reason | Owner | inventory | [[Req - Inventory]] |
| Low-stock alerts, reorder level per product | Owner | inventory | [[Req - Inventory]] |
| Batches and expiry, near-expiry alerts, first-expiry-first-out | Owner | inventory | [[Req - Inventory]] |
| Full and partial stock counts with variance | Owner | inventory | [[Req - Inventory]] |
| Stock history (an append-only ledger) | Owner | inventory | [[Req - Inventory]] |

## Purchasing
| Feature | Source | Module | Requirements |
|---|---|---|---|
| Supplier records | Owner | purchasing | [[Req - Purchasing]] |
| Purchase orders | Owner | purchasing | [[Req - Purchasing]] |
| Receive goods: stock goes up and cost price updates | Owner | purchasing, inventory | [[Req - Purchasing]] |
| Returns to suppliers | Owner | purchasing | [[Req - Purchasing]] |
| Money owed to suppliers and payments | Owner | purchasing | [[Req - Purchasing]] |

## Customers
| Feature | Source | Module | Requirements |
|---|---|---|---|
| Customers, with phone number as the main lookup | Owner | customers | [[Req - Customers and Loyalty]] |
| Purchase history | Owner | customers, sales | [[Req - Customers and Loyalty]] |
| Loyalty points: earn and redeem | Owner | customers | [[Req - Customers and Loyalty]] |
| Store credit and customer debt ("baki") | Owner | customers | [[Req - Customers and Loyalty]] |
| Membership tiers and member prices | Owner | pricing, customers | [[Req - Pricing and Promotions]] |

## Reports
| Feature | Source | Module | Requirements |
|---|---|---|---|
| Daily summary: totals, payment methods, tax | Owner | reports | [[Req - Reports]] |
| End-of-shift report (X and Z) | Owner | reports, shifts | [[Req - Shifts and Cash]] |
| Sales by product, category, brand, cashier, hour | Owner | reports | [[Req - Reports]] |
| Profit and margin | Owner | reports | [[Req - Reports]] |
| Stock value, low stock, near expiry | Owner | reports | [[Req - Reports]] |
| Export to Excel and PDF | Owner | io | [[Req - Reports]] |
| Tax report, payables/receivables aging, discounts and overrides report | Added | reports | [[Req - Reports]] |

## Platform
| Feature | Source | Module | Requirements |
|---|---|---|---|
| Fully offline, local database | Owner | db | [[Non-Functional Requirements]] |
| Automatic local backups, plus a second backup folder | Owner | db | [[Req - Backup and Logging]] |
| Admin password for protected actions; anyone can do everything else | Owner | auth | [[Roles and Permissions]] |
| Admin override at the till (one-time approval) | Added | auth | [[Req - Users and Security]] |
| Custom roles | Added | auth | [[Roles and Permissions]] |
| Audit log | Added | audit | [[Req - Users and Security]] |
| First-run setup wizard | Owner | settings | [[Req - Setup and Settings]] |
| Logging switch | Owner | app | [[Req - Backup and Logging]] |
| CSV/Excel import and export | Owner | io | [[Req - Import and Export]] |
| English UI and receipts, ready for more languages later | Owner | ui, printing | [[ADR-022 Ready for more languages later]] |
| Flat, colour-coded screens with a tooltip on every button and field | Owner | ui | [[Design System]] |
| Builds for Windows, Linux and macOS | Owner | CI | [[ADR-017 Windows first all three OS built]] |

## Out of scope
These are left out on purpose. Several are designed for so they can be added later without rework.
| Item | Why | Ready for later? |
|---|---|---|
| Several tills sharing data | Not on the owner's list; needs a server/sync design | Yes: ULID ids and a till id on every record ([[ADR-009 ULID ids and till identity]]) |
| Cloud sync, online store, accounting general ledger | Online features; not requested | No |
| Card terminal integration (EMV) | Varies by bank and country; card payments are recorded manually | Partly: the `card` payment kind stores a reference |
| Government fiscal devices and e-invoicing (e.g. EFD/SDC) | Different in each country | Yes: a `FiscalDevice` trait slot in [[Module - hardware]] |
| Compound taxes (tax on tax, e.g. supplementary duty) | Rare and country-specific; adds complexity | Partly: tax components exist; compound calculation does not |
| Buying in cartons and selling in pieces (unit conversion) | Not on the list; a pack can be its own product | No |
| Customer-facing display | Not requested | No |
| Database encryption at rest | Use operating-system disk encryption (BitLocker) | — |
