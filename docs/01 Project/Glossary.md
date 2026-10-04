---
tags: [project, glossary]
---
# Glossary

Use these words consistently in code, UI text and docs. When code and this page disagree, fix one of them.

| Term | Meaning | Code name |
|---|---|---|
| **Till** | One installation of the app on one computer, with its own code (e.g. `T1`) | `till_id`, `till_code` |
| **Shift** | One cashier's working period on a till, opened with a cash float and closed with a cash count | `shift` |
| **Business day / business date** | The trading day a record belongs to. Sales between midnight and the cutoff hour (default 04:00) belong to the previous day | `business_date` |
| **Cart** | A sale in progress. It is saved to the database on every change | `cart` |
| **Held cart** | A cart parked so another customer can be served | `cart.status = held` |
| **Sale** | A completed, paid cart with an invoice number. It is never edited afterwards | `sale` |
| **Invoice number** | Gapless number for each till, e.g. `T1-000123` | `invoice_no` |
| **Line** | One product row in a cart or sale | `line` |
| **Cancel** (a cart) | Throw away a sale in progress. No invoice number is used | `cancel_cart` |
| **Void** (a sale) | Reverse a completed sale completely. It keeps its invoice number and is marked VOID | `void_sale` |
| **Return** | A customer brings items back. Recorded as its own document with a return number | `return` |
| **Exchange** | A return and a new sale settled together | `exchange` |
| **Receipt** | The printed slip given to the customer. Never used to mean goods arriving | `receipt` |
| **GRN** (goods received note) | A record of goods arriving from a supplier | `grn` |
| **PO** (purchase order) | An order sent to a supplier | `purchase_order` |
| **Minor units** | The smallest unit of the currency (paisa, cent). All money is stored as whole minor units | `*_minor` |
| **Milli-quantity** | Quantity × 1000. 1 piece = 1000, 1.234 kg = 1234 | `*_milli` |
| **Basis points (bps)** | Hundredths of a percent. 15% = 1500 bps, 7.5% = 750 bps | `*_bps` |
| **Tax-inclusive** | Shelf prices already include tax (common in South Asia and Europe) | `TaxMode::Inclusive` |
| **Tax-exclusive** | Tax is added on top of shelf prices (common in North America) | `TaxMode::Exclusive` |
| **Tax rate** | A named rate, e.g. "VAT 15%". It can be split into components for display (e.g. CGST and SGST) | `tax_rate` |
| **Cash rounding** | Rounding the amount due to a practical increment, e.g. a whole taka | `rounding` |
| **Price list** | A named set of prices (Wholesale, Member) that overrides the retail price | `price_list` |
| **Tier** | Customer membership level. It links to a price list and/or a discount | `tier` |
| **Promotion** | An automatic price rule: % off, amount off, fixed price, buy X get Y, multi-buy | `promotion` |
| **Kit** | A product sold as one item but made of component products (a gift box) | `ProductKind::Kit` |
| **Variant** | A sellable version of a parent product, e.g. "Shampoo 200 ml Mango" | `ProductKind::Variant` |
| **Batch** | Stock of a product received together, with an optional expiry date | `batch` |
| **FEFO** | First expiry, first out: sales take stock from the batch that expires soonest | `fefo` |
| **Ledger** | An append-only table of changes (stock, points, credit, payables). Balances are sums of a ledger | `*_ledger` |
| **Stock level** | The current on-hand quantity. It is a cache of the ledger total | `inventory_levels` |
| **Adjustment** | A manual stock change with a reason (damage, theft, expiry, count error, opening stock) | `adjustment` |
| **Stock count** | Physically counting stock and posting the differences | `stock_count` |
| **Baki / on account / customer credit** | A sale paid later by the customer. Increases the customer's debt | `credit` |
| **Store credit** | Money the shop owes the customer (from a return). A negative customer balance | `credit` (negative) |
| **Payables** | Money the shop owes suppliers | `payables` |
| **Override** | An admin approving one restricted action on a cashier's screen | `override` |
| **X report** | A mid-shift report that does not close the shift | `x_report` |
| **Z report** | The end-of-shift report printed when the shift closes | `z_report` |
| **Scale label / embedded barcode** | A barcode printed by a weighing scale that contains a weight or price | `scale_label` |
| **Float** | Cash put in the drawer when a shift opens | `opening_float` |
| **Module** | One Rust crate plus its UI folder that owns one part of the business | — |
| **Workflow** | A use case in the `app` crate that coordinates several modules in one transaction | `workflows::*` |

## Technical words, in plain English
Developer pages use a few technical words. This is what they mean.

| Word | Plain meaning |
|---|---|
| **Crate** | A Rust package. Each module of this app is one crate |
| **Module** | One self-contained part of the app (e.g. sales, catalog) with its own code, data tables and screens |
| **Workflow** | A piece of code in the `app` crate that makes several modules work together on one job (e.g. completing a sale) |
| **Transaction** | A group of database changes that are saved together or not at all. Nothing is ever half-saved |
| **Migration** | A small numbered SQL file that creates or changes database tables when the app is upgraded |
| **View** (database) | A saved, read-only query. Modules share their data with reports through views |
| **SQLite** | The database built into the app. It's one file on disk; no server is needed |
| **WAL** | A SQLite setting that lets the app read and write at the same time and protects data during power cuts |
| **FTS5** | SQLite's built-in fast text search, used for searching products |
| **ULID** | A unique ID for each record, e.g. `01HX3K…`. Unique even across several tills, and sorts by time |
| **IPC / bridge / invoke** | How the screen (web part) asks the Rust part to do something |
| **Tauri** | The tool that turns our Rust code and web screens into a desktop app |
| **React / TypeScript / Tailwind** | The tools used to build the screens |
| **ESC/POS** | The command language receipt printers understand |
| **TSPL** | The command language many label printers understand |
| **Raster / picture printing** | Printing the receipt as a black-and-white picture instead of as text |
| **Spooler / CUPS** | The parts of Windows (spooler) and Mac/Linux (CUPS) that manage printers |
| **Serial port / COM port** | An old-style connection still used by scales and some printers, often through a USB adapter |
| **Unit test** | A small automatic check of one function |
| **Contract test** | An automatic check that a module still offers what other parts depend on |
| **Property test** | An automatic check that tries thousands of random inputs to make sure a rule always holds |
| **Golden image** | An approved picture of a receipt. Tests compare new printouts against it |
| **Invariant** | Something that must always be true, e.g. "stock level = sum of all stock changes" |
| **CI** | The automatic build-and-test run on GitHub after every change, on Windows, Mac and Linux |
| **UAT** | User acceptance testing: real users trying the app on real hardware before release |
| **ADR** | Architecture decision record: a short note explaining an important decision |
| **Mock / fake** | A pretend version of something (printer, scale, Rust side) used for testing |
| **Tooltip** | The small box that floats next to a button when you hover over it, explaining what it does |
