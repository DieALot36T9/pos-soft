---
tags: [testing]
---
# Test Data

## In Rust tests: builders, not fixture files
```rust
let shop = TestShop::new()                       // in-memory db, all migrations, FixedClock(2026-03-05 10:00)
    .currency("BDT", 2).tax_inclusive().rate("VAT 15%", 1500)
    .rounding(100, Nearest)
    .product("Milk 1L").barcode("8901234567890").price("90.00").stock(10)
    .weighed("Potato").unit("kg").price("60.00").scale_code("00123")
    .cashier("rahim", "1234").open_shift("rahim", "1000.00")
    .build();
let view = shop.scan("8901234567890")?;
```
- `TestShop` lives in `crates/app/tests/support/` (it needs every module). Module crates have their own small `test_db()` helpers.
- Amounts in builders are strings (`"90.00"`) parsed with the shop currency, so tests read like the requirements.

## Generated datasets (`cargo xtask seed`)
| Size | Contents | Used for |
|---|---|---|
| `demo` | 300 realistic grocery products, 10 categories, 3 promotions, 20 customers, 3 suppliers | Demo data in the setup wizard; manual testing; screenshots |
| `uat` | 2,000 products, barcodes from a fixed list printed on a test sheet, batches with near-expiry items | [[UAT Script]] |
| `reference` | 50,000 products, 80,000 barcodes, 5,000 customers, ~3 years of sales (5M lines), returns, GRNs | Performance budgets (NFR-P) |

Generation is deterministic (fixed random seed), so benchmark results are comparable between runs.

## UI mock data
`ui/src/shared/bridge/mock/fixtures.ts` is generated from the `demo` dataset by `xtask seed --emit-ui-fixtures`, so UI tests and Rust agree on the data.

## Hardware fixtures
- `crates/hardware/tests/fixtures/scales/*.txt`: recorded raw lines per scale protocol.
- `crates/printing/tests/golden/*.png`: approved receipt images.
- A printable **barcode test sheet** (PDF) with EAN-13, EAN-8, UPC-A, Code128 and scale-label codes for scanner testing.
