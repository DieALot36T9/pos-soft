---
tags: [module]
crate: pricing
milestone: M8 (engine skeleton in M6)
---
# Module: pricing

**Purpose:** decide the unit price and discounts of every cart line: price lists, tiers, promotions, manual discounts, invoice discount split. **The engine is a pure function.**

**Requirements:** [[Req - Pricing and Promotions]]. **Design:** [[ADR-019 Single best promotion per line]], [[Money Tax and Rounding]].
**Depends on:** `core`, `rusqlite`, `serde`, `thiserror`, `chrono` (only to find the local weekday for a promotion's days). Dev: `proptest`, `serde_json`.

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
Every function returns `Result<_, PricingError>`. `PricingError` turns into `core::AppError`: `Invalid` → validation (with a field and a code), `NameTaken` → conflict, `NotFound` → not found, database errors → internal.
```rust
// Engine input and output (cart.rs). All plain data with serde derives.
pub struct PricingProduct { pub id: Id, pub retail_price: Money, pub category_path: Vec<Id> /* top category first */, pub brand_id: Option<Id> }
pub struct LocalTime { pub at: Timestamp, pub offset_minutes: i32 }   // app: clock.now() + clock.local_offset_minutes()
pub enum ManualDiscount { Percent(Bps), Amount(Money) }               // JSON: {"kind":"percent","value":500}
pub struct TierRule { pub id: Id, pub discount: Bps }
pub struct PricingContext { pub now: LocalTime, pub tier: Option<TierRule>,
                            pub list_prices: HashMap<Id, Money>, pub promotions: Vec<Promotion> }
pub struct PriceLineIn { pub key: usize, pub product: PricingProduct, pub qty: Qty, pub embedded_price: Option<Money>,
                         pub price_override: Option<Money>, pub manual: Option<ManualDiscount> }
pub struct AppliedPromo { pub promotion_id: Id, pub name: String, pub rule: PromoRule, pub discount: Money }
pub struct PriceLineOut { pub key: usize, pub unit_price: Money, pub base: Money, pub promo: Option<AppliedPromo>,
                          pub tier_discount: Money, pub manual_discount: Money, pub invoice_share: Money, pub amount: Money }
impl PriceLineOut { pub fn promo_discount(&self) -> Money; }          // 0 if no promotion

// Loading (I/O, load.rs)
pub fn load_context(conn, products: &[PricingProduct], tier: Option<&Id>, price_list: Option<&Id>, now: LocalTime) -> Result<PricingContext>;

// Engine (pure, no I/O, engine.rs)
pub fn price_cart(ctx: &PricingContext, lines: &[PriceLineIn], invoice_discount: Option<ManualDiscount>) -> Result<Vec<PriceLineOut>>;
pub fn manual_discount_bps(lines: &[PriceLineOut]) -> Bps;   // for checking role limits (PRC-07)

// Promotions (promotion.rs, promotion_store.rs)
pub enum PromoRule { PercentOff { percent: Bps }, AmountOff { amount_per_unit: Money }, FixedPrice { price_per_unit: Money },
                     BuyXGetY { buy: u32, get: u32 }, MultiBuy { bundle_qty: u32, bundle_price: Money } }  // JSON tag "kind"
pub enum PromoTarget { Product(Id), Category(Id), Brand(Id) }       // JSON {"type":"category","id":"…"}
pub const EVERY_DAY: u8 = 0b111_1111;                                 // days_mask: bit 0 = Monday … bit 6 = Sunday
pub struct Promotion { id, name, rule, starts_at, ends_at, days_mask, tier_id, targets /* sorted */, active, created_at }
pub struct PromotionInput { name, rule, starts_at, ends_at, days_mask, tier_id, targets }
pub fn create_promotion(tx, &PromotionInput, now: Timestamp) -> Result<Id>;
pub fn update_promotion(tx, &Id, &PromotionInput) -> Result<()>;     // replaces settings and targets
pub fn set_promotion_active(tx, &Id, active: bool) -> Result<()>;    // the "active switch"
pub fn promotion(conn, &Id) -> Result<Promotion>;
pub fn promotions(conn) -> Result<Vec<Promotion>>;                   // all, oldest first

// Price lists (price_lists.rs)
pub struct PriceList { id, name, active, created_at }   pub struct PriceListUpdate { name, active }
pub struct PriceListItem { product_id: Id, price: Money }
pub fn create_price_list(tx, name: &str, now: Timestamp) -> Result<Id>;
pub fn update_price_list(tx, &Id, &PriceListUpdate) -> Result<()>;  // rename, switch on/off
pub fn set_price_list_items(tx, &Id, &[PriceListItem]) -> Result<()>;  // adds or replaces prices
pub fn remove_price_list_items(tx, &Id, product_ids: &[Id]) -> Result<()>;
pub fn price_lists(conn) -> Result<Vec<PriceList>>;                  // by name
pub fn price_list_items(conn, &Id) -> Result<Vec<PriceListItem>>;    // by product id

// Tiers (tiers.rs)
pub struct Tier { id, name, price_list_id, discount: Bps, sort, archived_at }   pub struct TierInput { name, price_list_id, discount, sort }
pub fn create_tier(tx, &TierInput) -> Result<Id>;
pub fn update_tier(tx, &Id, &TierInput) -> Result<()>;
pub fn archive_tier(tx, &Id, now: Timestamp) -> Result<()>;
pub fn tier(conn, &Id) -> Result<Tier>;
pub fn tiers(conn) -> Result<Vec<Tier>>;                             // archived too, by sort then name

pub fn migrations() -> ModuleMigrations;                             // 001_init
```

### How `price_cart` works
The steps follow "Line calculation" in [[Money Tax and Rounding]]. Lines come back in the order they went in.
1. **Unit price:** the override (CHK-13), else the price list price (PRC-02), else retail (PRC-01). **Base:** `round(unit price × qty)`, or the scale sticker price exactly (CHK-08).
2. **Promotion:** lines of the same product at the same unit price are grouped, and every running promotion is worked out on the **combined quantity** (PRC-05). The one that saves the most wins; on a tie the oldest wins (ADR-019). Its saving is split over the group's lines in proportion to each line's base with `core::allocate` (largest remainder, ties to the earlier line), so the shares add up exactly.
3. **Tier discount:** `round(base × tier %)`, only on lines with no promotion (PRC-03).
4. **Manual line discount:** % of what's left, or an amount; never more than what's left (CHK-12).
5. **Invoice discount:** % of the subtotal after line discounts, or an amount; never more than the subtotal; split with `core::allocate` by what each line still costs.
6. `amount = base − promo − tier − manual − invoice share`.

Rules worth knowing:
- **A promotion never raises a price.** Each saving is kept between zero and the group's base. A fixed price or a multi-buy that isn't cheaper than the line's normal price (retail or price list) saves nothing, so it's ignored and the tier discount can apply instead.
- **Running** means: switched on, `starts_at ≤ now < ends_at` (either may be empty), the **local** weekday is in `days_mask`, and if the promotion has a tier, the cart has that tier.
- Buy X get Y and multi-buy count **whole units** only (2.9 kg counts as 2). Amount off and fixed price work per unit, so per kg for weighed items.
- **Scale sticker lines and overridden lines get no automatic discounts** (no promotion, no tier discount), and don't count toward a promotion's quantity. The cashier's manual discount and the invoice discount still apply to them, because those are checked against the role's limit like any other discount.
- Negative quantities, prices or discount amounts are refused with a validation error (`qty_negative`, `price_negative`, `discount_negative`). With inputs of zero or more, no line amount goes below zero.
- `manual_discount_bps` gives the **largest** manual discount as a %, rounded up: each line's manual discount against what that line cost before it, and the invoice discount against the subtotal before it. `app` compares it with the role's `max_discount_bps` (PRC-07, CHK-12).

### How `load_context` works
- A price list chosen for the cart (CHK-17) wins over the tier's own price list. A switched-off price list gives no prices (retail applies). An unknown price list or tier id is a `NotFound` error.
- An archived tier gives no prices and no discount, even if a customer still has it.
- Promotions: it reads every switched-on promotion that hasn't ended, then keeps those that cover at least one of the products. The engine checks the start, weekday and tier itself, so `price_cart` stays pure.

### Checks when saving
| Code | When |
|---|---|
| `price_list_name_missing`, `tier_name_missing`, `promotion_name_missing` | Name is empty or only spaces |
| `price_list_name_taken`, `tier_name_taken` (conflict) | Another price list / tier has this name |
| `price_negative` | A price list price below zero |
| `tier_discount_invalid` | Tier discount above 100% |
| `promotion_percent_invalid` | Percent off is 0 or above 100% |
| `promotion_amount_invalid` | Amount off is 0 or less |
| `promotion_price_invalid` | Fixed price below zero |
| `promotion_buy_get_invalid` | Buy or get is 0 |
| `promotion_bundle_invalid` | Bundle of fewer than 2, or a bundle price below zero |
| `promotion_days_invalid` | No days chosen, or bits above Sunday |
| `promotion_dates_invalid` | Start is not before the end |
| `promotion_targets_missing` | No product, category or brand chosen |

## Tests
Each test name starts with the requirement it covers.
- `tests/prc_06_examples.rs`: every row of PRC-06.
- `tests/engine_promotions.rs`: amount off, fixed price below retail and above the price list price, a multi-buy that isn't cheaper, day-of-week mask in local time, not started yet, end time, switched off, tier-only promotion, category inheritance (parent category promotion applies to a child), brand, combined quantity across two lines, tie goes to the oldest.
- `tests/engine_rules.rs`: retail (PRC-01), price list fallback (PRC-02), Gold tier (PRC-03), manual after promotion (PRC-05), discounts can't make a line negative (CHK-12), an override disables the promotion and tier and doesn't count toward a promotion (CHK-13), scale sticker lines (CHK-08), invoice discount split E8/E9 (CHK-12), `manual_discount_bps` (PRC-07), negative inputs refused.
- `tests/properties.rs` (proptest): `amount ≥ 0`, each discount ≥ 0, `amount = base − discounts` and `Σ discounts ≤ Σ base` for random carts, promotions, tiers and discounts.
- `tests/contract.rs`: migrations build every table and the view; `pricing_v_promotions` columns; price list save/replace/remove and checks; `load_context` with price lists (PRC-02, CHK-17), tiers (PRC-03), archived tiers and unknown ids.
- `tests/contract_promotions.rs`: every promotion kind saved and read back, update replaces targets, every save check, not found, and `load_context` bringing only switched-on promotions for the cart's products (CHK-16).
- Unit tests: saving maths for part units, error conversion, local weekday, JSON shape of `ManualDiscount`.

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
- A promotion never raises a price above what the line would cost without it (retail or the price list price).
- Lines priced by a scale sticker get no automatic discounts (promotion, tier). Manual and invoice discounts still apply.
- `price_cart` returns a `Result`: money helpers in `core` report overflow as an error, and negative inputs are refused.

**Done when:** the [[Definition of Done]] is met, and this page matches the code.

## Open items
*Anything not finished yet. The next session working on this module starts here.*
- **Built in task B-pricing:** tables, the engine, loading and admin functions, with tests. Not done here, by design: `app` commands and wiring (C-M6, C-M8) and screens (C-M8).
- **TypeScript types (done in C-M8):** the structs the screens use derive `ts_rs::TS` and are exported to `ui/src/generated/`.
- **Connected in C-M8:** `app` saves price lists, their items, tiers and promotions from the "Prices and promotions" screen, passes the customer's tier and the cart's price list to `load_context`, and checks `manual_discount_bps` against the role's limit. The PRC-06 examples also run end to end in `crates/app/tests/prc_06_end_to_end.rs`.
- **For `app` (C-M6):** map `catalog::price_info` into `PricingProduct` with `category_path` ordered from the top category down. Build `LocalTime` from `clock.now()` and `clock.local_offset_minutes()`. Check `manual_discount_bps` against the role's limit and ask for an override above it.
- **Product owner to confirm** (decided here, see the PR): (1) a promotion saving on a product split over several lines is shared in proportion to each line's base, not filled line by line; (2) scale sticker and overridden lines still accept manual and invoice discounts; (3) `manual_discount_bps` uses the largest line or invoice percentage.
- **Speed:** `load_context` reads every switched-on, not-ended promotion and filters in Rust. That is fine for hundreds of promotions. If a shop has thousands, look them up through the `pricing_targets_lookup` index instead.
- **Coverage:** Test Strategy asks for 100% of public functions in `pricing`; every public function is called by a test, but `cargo llvm-cov` wasn't run in this session.
- Migration `001_init` is not released yet, so the "earlier release" migration test starts with the next migration.
