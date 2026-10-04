---
tags: [module]
crate: pricing
milestone: M8 (engine skeleton in M6)
---
# Module: pricing

**Purpose:** decide the unit price and discounts of every cart line: price lists, tiers, promotions, manual discounts, invoice discount split. **The engine is a pure function.**

**Requirements:** [[Req - Pricing and Promotions]]. **Design:** [[ADR-019 Single best promotion per line]], [[Money Tax and Rounding]].
**Depends on:** `core`, `rusqlite`.

## Tables
```sql
CREATE TABLE pricing_price_lists (
  id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, active INTEGER NOT NULL DEFAULT 1, created_at INTEGER NOT NULL
);
CREATE TABLE pricing_price_list_items (
  price_list_id TEXT NOT NULL REFERENCES pricing_price_lists(id),
  product_id    TEXT NOT NULL,                       -- catalog (no FK)
  price_minor   INTEGER NOT NULL CHECK (price_minor >= 0),
  PRIMARY KEY (price_list_id, product_id)
);
CREATE TABLE pricing_tiers (
  id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE,
  price_list_id TEXT REFERENCES pricing_price_lists(id),
  discount_bps  INTEGER NOT NULL DEFAULT 0, sort INTEGER NOT NULL DEFAULT 0, archived_at INTEGER
);
CREATE TABLE pricing_promotions (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,
  kind          TEXT NOT NULL CHECK (kind IN ('percent_off','amount_off','fixed_price','buy_x_get_y','multi_buy')),
  percent_bps   INTEGER,          -- percent_off
  amount_minor  INTEGER,          -- amount_off (per unit) / fixed_price (per unit) / multi_buy (bundle price)
  buy_qty       INTEGER,          -- buy_x_get_y: buy X (whole units)
  get_qty       INTEGER,          -- buy_x_get_y: get Y free
  bundle_qty    INTEGER,          -- multi_buy
  starts_at     INTEGER, ends_at INTEGER,
  days_mask     INTEGER NOT NULL DEFAULT 127,   -- bit 0 = Monday
  tier_id       TEXT REFERENCES pricing_tiers(id),
  active        INTEGER NOT NULL DEFAULT 1,
  created_at    INTEGER NOT NULL
);
CREATE TABLE pricing_promotion_targets (
  promotion_id TEXT NOT NULL REFERENCES pricing_promotions(id),
  target_type  TEXT NOT NULL CHECK (target_type IN ('product','category','brand')),
  target_id    TEXT NOT NULL,
  PRIMARY KEY (promotion_id, target_type, target_id)
);
CREATE INDEX pricing_targets_lookup ON pricing_promotion_targets(target_type, target_id);
CREATE VIEW pricing_v_promotions AS SELECT id, name, kind, starts_at, ends_at, active FROM pricing_promotions;
```

## Public API
```rust
// Loading (I/O)
pub fn load_context(conn, products: &[PricingProduct], tier: Option<&Id>, price_list: Option<&Id>, now: LocalTime) -> Result<PricingContext>;

// Engine (pure, no I/O)
pub struct PriceLineIn { pub key: usize, pub product: PricingProduct /* id, retail price, category path, brand */,
                         pub qty: Qty, pub embedded_price: Option<Money>, pub price_override: Option<Money>,
                         pub manual: Option<ManualDiscount> }
pub struct PriceLineOut { pub key: usize, pub unit_price: Money, pub base: Money, pub promo: Option<AppliedPromo>,
                          pub tier_discount: Money, pub manual_discount: Money, pub invoice_share: Money, pub amount: Money }
pub fn price_cart(ctx: &PricingContext, lines: &[PriceLineIn], invoice_discount: Option<ManualDiscount>) -> Vec<PriceLineOut>;
pub fn manual_discount_bps(lines: &[PriceLineOut]) -> Bps;   // for checking role limits

// Admin CRUD
pub fn create_price_list / set_price_list_items / create_tier / update_tier / create_promotion / update_promotion / deactivate_promotion …
pub fn migrations() -> ModuleMigrations;
```
Notes:
- Promotions on the **same product across several cart lines** use the combined quantity; the discount is assigned to the lines in order.
- Price-embedded scale lines never get promotions or tier discounts (their price is fixed by the label).
- Price overrides disable promotions and tier discounts on that line.

## Tests
- Every row of PRC-06, plus: day-of-week mask, tier-only promotion, category inheritance (parent category promotion applies to a child), combined quantity across two lines, a fixed price above retail is ignored, an override disables the promotion.
- proptest: `amount ≥ 0` and `Σ discounts ≤ Σ base` for random carts.

## Working on this module
*Everything needed to work on this module on its own. Read this page, [[Module Rules]] and [[Module - core]]. Nothing else is required. The same instructions are in `crates/pricing/CLAUDE.md`, which points back here.*

| | |
|---|---|
| **Rust folder** | `crates/pricing/` |
| **Screen folder** | `ui/src/modules/pricing/` |
| **Requirements** | [[Req - Pricing and Promotions]] PRC-01 to PRC-08; [[Req - Checkout]] CHK-12, CHK-16, CHK-17 |
| **You may change** | Files in the folders above, this page, and new migration files |
| **Don't touch** | Other modules' folders and tables. If you need another module's data, take it as a function argument; `app` passes it in |
| **Gets from `app`** | Each product's retail price, category path and brand (mapped by `app` from catalog), the cart's tier and price list, and the local time |
| **Used by** (re-run their tests when the public API changes) | Cart pricing, completing a sale, shelf tags (promotion prices) |
| **Test it on its own** | `cargo nextest run -p pricing` · screens: `pnpm test src/modules/<folder>` |

**Watch out for**
- `price_cart` must stay **pure**: no database, no clock. Everything comes in through `PricingContext`.
- Only the single best promotion per line ([[ADR-019 Single best promotion per line]]).
- A promotion never raises a price above retail.
- Lines priced by a scale sticker get no discounts.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.
