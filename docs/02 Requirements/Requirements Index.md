---
tags: [requirements]
---
# Requirements Index

Each story has a **stable ID** (`AREA-NN`). Tests name the ID they prove, e.g. `fn chk_08_weight_embedded_label_adds_weighed_line()`.

| Area | Prefix | Page | Main modules |
|---|---|---|---|
| Setup and settings | SET | [[Req - Setup and Settings]] | settings, tax |
| Users and security | USR | [[Req - Users and Security]] | auth, audit |
| Checkout | CHK | [[Req - Checkout]] | sales, catalog, barcode, pricing |
| Payments | PAY | [[Req - Payments]] | sales, shifts, customers |
| Returns and voids | RET | [[Req - Returns and Voids]] | sales, inventory, customers |
| Shifts and cash | SHF | [[Req - Shifts and Cash]] | shifts |
| Catalog | CAT | [[Req - Catalog]] | catalog, barcode, printing |
| Pricing and promotions | PRC | [[Req - Pricing and Promotions]] | pricing |
| Tax | TAX | [[Req - Tax]] | tax |
| Inventory | INV | [[Req - Inventory]] | inventory |
| Purchasing | PUR | [[Req - Purchasing]] | purchasing, inventory |
| Customers and loyalty | CUS | [[Req - Customers and Loyalty]] | customers |
| Reports | RPT | [[Req - Reports]] | reports, io |
| Import and export | IO | [[Req - Import and Export]] | io |
| Hardware and printing | HW | [[Req - Hardware and Printing]] | printing, hardware |
| Backup and logging | BAK | [[Req - Backup and Logging]] | db, app |
| Quality attributes | NFR | [[Non-Functional Requirements]] | all |
| Permissions | — | [[Roles and Permissions]] | auth, app |

## Story format
```
### CHK-01 Short title
As a <role>, I want <capability>, so that <benefit>.
- Given <state>
- When <action>
- Then <observable result>
```
Use the template `Templates/Template - User Story`.
