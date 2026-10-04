---
tags: [requirements]
modules: [catalog, barcode, printing, inventory]
---
# Requirements: Catalog (CAT)

Modules: [[Module - catalog]], [[Module - barcode]]. Screens: [[Back Office Screens#Products]].

### CAT-01 Product fields
| Field | Rules |
|---|---|
| Name | Required, ≤ 120 characters |
| SKU | Required, unique. Auto-generated if left blank (`P000001`) |
| Barcodes | 0 or more, each unique across all products |
| Category | Required (a default "Uncategorised" category exists) |
| Brand | Optional |
| Unit | Required ([[#CAT-02 Units]]) |
| Cost price | ≥ 0, money |
| Sale price | ≥ 0, money (retail price) |
| Tax rate | Required (default from settings) |
| Weighed | Yes/no (unit must allow decimals) |
| Scale item code | Optional, unique; used by scale-label barcodes |
| Track stock | Default yes (no for services and kits) |
| Track expiry | Default no |
| Reorder level and quantity | Stored by inventory, edited on the same form |
| Preferred supplier | Optional |
| Image | Optional |
| Description / notes | Optional |

### CAT-02 Units
- Built in: piece (0 decimals), kg (3), g (0), litre (3), ml (0), pack (0)
- Admins can add units (e.g. "dozen", "box") with 0–3 decimals
- A unit in use can't be deleted

### CAT-03 Categories and brands
- Categories form a tree (unlimited depth; the UI shows 3 levels comfortably). Brands are a flat list
- Archiving a category that still has active products is refused

### CAT-04 Barcodes
- Given barcode `890…` belongs to "Milk 1L"
- When it's added to another product
- Then it's refused: "Already used by Milk 1L (SKU P000123)"
- EAN-13/EAN-8/UPC-A check digits are validated when the code looks like one of those (all digits, length 8/12/13). Other codes (Code128) are accepted as given

### CAT-05 Search
- Full-text search over name, SKU, barcodes, category and brand, with prefix matching ("mil" finds "Milk")
- Exact barcode/SKU matches are always ranked first

### CAT-06 Variants
- Create a **parent** product with attributes (e.g. Size: 200 ml, 400 ml; Flavour: Mango, Lemon)
- "Generate variants" creates every combination as a variant product with its own SKU, barcode, price and stock (fields copied from the parent, then editable)
- The parent isn't sellable and holds no stock

### CAT-07 Kits
- A kit lists components with quantities. Kits have their own price and tax rate and no stock of their own
- Available kit quantity = the smallest of (component stock ÷ component quantity)

### CAT-08 Archive instead of delete
- Archiving hides a product from sale and search but keeps it in history and reports
- Permanent delete is only allowed for products that were never used anywhere (no sales, stock, POs)

### CAT-09 Price history
- Every change to the sale or cost price is recorded with the old and new value, time and user. It's shown on the product page

### CAT-10 Images
- PNG/JPG/WebP up to 10 MB is resized to 512 px plus a 96 px thumbnail, stored as files. The database keeps paths

### CAT-11 Internal barcodes
- "Generate barcode" creates an EAN-13 in the shop's internal range (default prefix `200`, configurable), never overlapping the scale-label prefixes

### CAT-12 Barcode labels
- Choose products and the number of labels (defaults: 1, or the quantity just received from a GRN)
- Templates: label size, fields (name, price, barcode, unit price, packed/expiry date)
- Output: A4 sheets with a chosen layout (e.g. 3×8) through the system printer, or a thermal label printer
- Printed labels must scan correctly ([[Req - Hardware and Printing]])

### CAT-13 Shelf tags
- Templates show name, big price, unit price (per kg / per litre), barcode, and the promotion price with dates if a promotion is active
- Print selected products, a category, or "prices changed since <date>"
