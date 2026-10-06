//! Repricing a cart (Workflows, "Reprice cart"). Every cart change ends here, and the
//! screen gets the whole cart back with every amount worked out in Rust (CHK-18).
//! Nothing about the price is stored on the cart: completing the sale prices it again.

use std::collections::HashMap;

use catalog::{Product, ProductKind, Unit};
use customers::CustomerWithBalances;
use pos_core::{AppError, Clock, Id, Money, Qty, RoundingRule, TaxMode, business_date, cash_round};
use pricing::{LocalTime, ManualDiscount, PriceLineIn, PriceLineOut, PricingProduct};
use rusqlite::Connection;
use sales::{Cart, CartLine, Discount};
use serde::Serialize;
use settings::TaxSettings;
use tax::{LineTax, RateWithComponents};
use ts_rs::TS;

/// A stock problem shown on a cart line as an amber strip (CHK-21). Never blocks here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum StockWarning {
    /// The shop has less in stock than this cart sells.
    NotEnoughStock,
    /// Some of the stock is in a batch past its expiry date.
    ExpiredBatch,
}

/// One cart line as the checkout screen shows it. Every amount comes from Rust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CartLineView {
    pub line_id: Id,
    pub seq: u32,
    pub product_id: Id,
    pub name: String,
    pub sku: String,
    /// Example: "kg", "pc".
    pub unit_symbol: String,
    /// How many decimals a quantity may have. Piece is 0.
    pub unit_decimals: u8,
    /// Sold by weight: the line shows the scale icon.
    pub is_weighed: bool,
    pub qty: Qty,
    /// The price for one unit: an override, the price list price or retail.
    pub unit_price: Money,
    /// unit price × quantity, or the price printed on a scale sticker.
    pub base: Money,
    /// Example: "Buy 2 get 1". Shown in purple under the line (CHK-16).
    pub promotion_name: Option<String>,
    /// Every discount on the line: promotion, tier, manual and its share of the invoice discount.
    pub discount: Money,
    /// What the line costs after its discounts (with tax when prices include tax).
    pub amount: Money,
    /// The price on a scale sticker was used exactly as printed (CHK-08).
    pub sticker_price: bool,
    /// The unit price was typed in with `PriceOverride` (CHK-13).
    pub price_overridden: bool,
    /// The cashier's discount on this line, as typed (CHK-12).
    pub manual_discount: Option<ManualDiscount>,
    pub stock_warning: Option<StockWarning>,
}

/// The totals panel on the right of the checkout screen (CHK-18).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CartTotals {
    /// Pieces count one each; a weighed line counts as one item. Example: 12.
    pub item_count: u32,
    /// Σ base amounts, before any discount.
    pub subtotal: Money,
    /// Σ every discount on every line.
    pub discounts: Money,
    pub net: Money,
    pub tax: Money,
    /// What the customer pays before cash rounding.
    pub gross: Money,
    /// Cash rounding if the rest is paid in cash (SET-06, PAY-06). Not taxed.
    pub rounding: Money,
    /// Total due when paying in cash: gross + rounding. The biggest number on screen.
    pub total: Money,
}

/// The customer attached to the cart (CHK-11), with what the till shows about them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CartCustomer {
    pub id: Id,
    pub name: String,
    /// In E.164 form. Example: "+8801711123456".
    pub phone: String,
    /// Example: "Gold". None when the customer has no tier or it is archived.
    pub tier_name: Option<String>,
    pub points: i64,
    /// Positive: the customer owes the shop. Negative: store credit.
    pub credit: Money,
    pub credit_limit: Money,
}

/// A price list as a choice: the cart's price list, or one the cashier may switch to (CHK-17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PriceListChoice {
    pub id: Id,
    /// Example: "Wholesale".
    pub name: String,
}

/// The whole cart, sent back after every change (Frontend Architecture, "Checkout data flow").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct CartView {
    pub cart_id: Id,
    pub customer: Option<CartCustomer>,
    /// The price list switched on for this cart (CHK-17). None: normal prices.
    pub price_list: Option<PriceListChoice>,
    /// The discount on the whole cart, as typed (CHK-12).
    pub invoice_discount: Option<ManualDiscount>,
    pub lines: Vec<CartLineView>,
    pub totals: CartTotals,
    /// The tax name from settings. Example: "VAT".
    pub tax_label: String,
    /// Inclusive: prices already contain tax, shown as "VAT (incl.)".
    pub tax_mode: TaxMode,
}

/// One cart line with everything pricing, tax and catalog said about it.
pub(crate) struct PricedLine {
    pub line: CartLine,
    pub product: Product,
    pub unit: Unit,
    /// The normal sale price for one unit, for "you saved" and reports.
    pub retail_price: Money,
    pub price: PriceLineOut,
    pub tax: LineTax,
    pub rate: RateWithComponents,
}

/// A cart priced right now: the lines in cart order and the shop's tax setting.
pub(crate) struct PricedCart {
    pub cart: Cart,
    pub lines: Vec<PricedLine>,
    pub tax: TaxSettings,
    /// The attached customer with their balances (CHK-11).
    pub customer: Option<CustomerWithBalances>,
}

impl PricedCart {
    /// Σ line gross: what the customer pays before cash rounding.
    pub fn gross(&self) -> Result<Money, AppError> {
        let amounts: Vec<Money> = self.lines.iter().map(|line| line.tax.gross).collect();
        Money::sum(&amounts)
    }
}

/// Prices every line of the cart with today's prices, promotions and tax rates.
pub(crate) fn price_cart(
    conn: &Connection,
    clock: &dyn Clock,
    cart: Cart,
) -> Result<PricedCart, AppError> {
    let tax: TaxSettings = settings::get(conn)?;
    let customer = cart
        .customer_id
        .as_ref()
        .map(|id| customers::get(conn, id))
        .transpose()?;
    if cart.lines.is_empty() {
        return Ok(PricedCart {
            cart,
            lines: Vec::new(),
            tax,
            customer,
        });
    }
    let mut ids: Vec<Id> = Vec::new();
    for line in &cart.lines {
        if !ids.contains(&line.product_id) {
            ids.push(line.product_id.clone());
        }
    }
    let infos = catalog::price_info(conn, &ids)?;
    let units: HashMap<Id, Unit> = catalog::list_units(conn, true)?
        .into_iter()
        .map(|unit| (unit.id.clone(), unit))
        .collect();
    let mut products = HashMap::new();
    let mut pricing_products = Vec::new();
    for id in &ids {
        let info = infos
            .get(id)
            .ok_or_else(|| AppError::internal(format!("no price details for product {id}")))?;
        // Pricing wants the top category first; catalog gives the product's own first.
        let mut category_path = info.category_path.clone();
        category_path.reverse();
        pricing_products.push(PricingProduct {
            id: id.clone(),
            retail_price: info.sale_price,
            category_path,
            brand_id: info.brand_id.clone(),
        });
        products.insert(id.clone(), catalog::get(conn, id)?);
    }
    let now = LocalTime {
        at: clock.now(),
        offset_minutes: clock.local_offset_minutes(),
    };
    // The customer's tier gives its price list and discount (PRC-03, CHK-11).
    let tier = customer
        .as_ref()
        .and_then(|customer| customer.customer.tier_id.as_ref());
    let context = pricing::load_context(
        conn,
        &pricing_products,
        tier,
        cart.price_list_id.as_ref(),
        now,
    )?;
    let lines_in = pricing_lines(&cart, &pricing_products)?;
    let priced = pricing::price_cart(
        &context,
        &lines_in,
        cart.invoice_discount.map(manual_discount),
    )?;
    let rate_ids: Vec<Id> = products
        .values()
        .map(|product| product.tax_rate_id.clone())
        .collect();
    let rates = tax::get_many(conn, &rate_ids)?;
    let mut lines = Vec::with_capacity(cart.lines.len());
    for (line, price) in cart.lines.iter().zip(priced) {
        let product = products[&line.product_id].clone();
        let rate = rates
            .get(&product.tax_rate_id)
            .cloned()
            .ok_or_else(|| AppError::internal(format!("tax rate of {} is gone", product.id)))?;
        let unit = units
            .get(&product.unit_id)
            .cloned()
            .ok_or_else(|| AppError::internal(format!("unit of {} is gone", product.id)))?;
        let line_tax = tax::calc_line(price.amount, rate.rate_bps, tax.mode)?;
        lines.push(PricedLine {
            line: line.clone(),
            retail_price: infos[&line.product_id].sale_price,
            product,
            unit,
            price,
            tax: line_tax,
            rate,
        });
    }
    Ok(PricedCart {
        cart,
        lines,
        tax,
        customer,
    })
}

/// The cart's lines in the form the pricing engine takes.
fn pricing_lines(
    cart: &Cart,
    pricing_products: &[PricingProduct],
) -> Result<Vec<PriceLineIn>, AppError> {
    let mut lines_in = Vec::with_capacity(cart.lines.len());
    for (key, line) in cart.lines.iter().enumerate() {
        let product = pricing_products
            .iter()
            .find(|each| each.id == line.product_id)
            .cloned()
            .ok_or_else(|| AppError::internal("a cart product went missing"))?;
        lines_in.push(PriceLineIn {
            key,
            product,
            qty: line.qty,
            embedded_price: line.embedded_price,
            price_override: line.price_override.as_ref().map(|each| each.price),
            manual: line.manual_discount.map(manual_discount),
        });
    }
    Ok(lines_in)
}

/// Builds what the screen shows from a priced cart, with the stock warnings (CHK-21).
pub(crate) fn view(
    conn: &Connection,
    clock: &dyn Clock,
    priced: &PricedCart,
) -> Result<CartView, AppError> {
    let rounding_rule: RoundingRule = settings::get(conn)?;
    let warnings = stock_warnings(conn, clock, priced)?;
    let mut lines = Vec::with_capacity(priced.lines.len());
    let mut item_count: u32 = 0;
    let mut sums = Sums::default();
    for priced_line in &priced.lines {
        let price = &priced_line.price;
        let discount = Money::sum(&[
            price.promo_discount(),
            price.tier_discount,
            price.manual_discount,
            price.invoice_share,
        ])?;
        sums.add(price.base, discount, &priced_line.tax)?;
        item_count = item_count.saturating_add(items_on_line(priced_line));
        lines.push(CartLineView {
            line_id: priced_line.line.id.clone(),
            seq: priced_line.line.seq,
            product_id: priced_line.product.id.clone(),
            name: priced_line.product.name.clone(),
            sku: priced_line.product.sku.clone(),
            unit_symbol: priced_line.unit.symbol.clone(),
            unit_decimals: priced_line.unit.decimals,
            is_weighed: priced_line.product.is_weighed,
            qty: priced_line.line.qty,
            unit_price: price.unit_price,
            base: price.base,
            promotion_name: price.promo.as_ref().map(|promo| promo.name.clone()),
            discount,
            amount: price.amount,
            sticker_price: priced_line.line.embedded_price.is_some(),
            price_overridden: priced_line.line.price_override.is_some(),
            manual_discount: priced_line.line.manual_discount.map(manual_discount),
            stock_warning: warnings.get(&priced_line.product.id).copied(),
        });
    }
    let rounded = cash_round(sums.gross, &rounding_rule)?;
    Ok(CartView {
        cart_id: priced.cart.id.clone(),
        customer: priced
            .customer
            .as_ref()
            .map(|customer| cart_customer(conn, customer))
            .transpose()?,
        price_list: priced
            .cart
            .price_list_id
            .as_ref()
            .map(|id| price_list_choice(conn, id))
            .transpose()?,
        invoice_discount: priced.cart.invoice_discount.map(manual_discount),
        lines,
        totals: CartTotals {
            item_count,
            subtotal: sums.base,
            discounts: sums.discount,
            net: sums.net,
            tax: sums.tax,
            gross: sums.gross,
            rounding: rounded.checked_sub(sums.gross)?,
            total: rounded,
        },
        tax_label: priced.tax.label.clone(),
        tax_mode: priced.tax.mode,
    })
}

/// Reprices a cart and builds its view in one step.
pub(crate) fn cart_view(
    conn: &Connection,
    clock: &dyn Clock,
    cart: Cart,
) -> Result<CartView, AppError> {
    let priced = price_cart(conn, clock, cart)?;
    view(conn, clock, &priced)
}

/// The shop's local date today, for "has this batch expired?".
pub(crate) fn local_today(clock: &dyn Clock) -> Result<chrono::NaiveDate, AppError> {
    Ok(business_date(clock.now(), clock.local_offset_minutes(), 0)?.0)
}

/// True when the product keeps stock of its own: kits and variant parents don't (INV-09).
pub(crate) fn keeps_stock(product: &Product) -> bool {
    product.track_stock && matches!(product.kind, ProductKind::Standard | ProductKind::Variant)
}

/// Rule CHK-21: warn when the cart sells more than the stock, or stock has expired.
fn stock_warnings(
    conn: &Connection,
    clock: &dyn Clock,
    priced: &PricedCart,
) -> Result<HashMap<Id, StockWarning>, AppError> {
    let mut wanted: HashMap<Id, i64> = HashMap::new();
    for line in &priced.lines {
        if keeps_stock(&line.product) {
            *wanted.entry(line.product.id.clone()).or_insert(0) += line.line.qty.0;
        }
    }
    let today = local_today(clock)?;
    let mut warnings = HashMap::new();
    for (product_id, qty) in wanted {
        let available = inventory::availability(conn, &product_id, today)?;
        if available.has_expired_batch {
            warnings.insert(product_id, StockWarning::ExpiredBatch);
        } else if available.on_hand.0 < qty {
            warnings.insert(product_id, StockWarning::NotEnoughStock);
        }
    }
    Ok(warnings)
}

/// What the till shows about the attached customer.
fn cart_customer(
    conn: &Connection,
    customer: &CustomerWithBalances,
) -> Result<CartCustomer, AppError> {
    let record = &customer.customer;
    Ok(CartCustomer {
        id: record.id.clone(),
        name: record.name.clone(),
        phone: record.phone.clone(),
        tier_name: running_tier_name(conn, record.tier_id.as_ref())?,
        points: customer.points,
        credit: customer.credit,
        credit_limit: record.credit_limit,
    })
}

/// The tier's name, unless it is archived (an archived tier gives nothing, PRC-03).
pub(crate) fn running_tier_name(
    conn: &Connection,
    tier_id: Option<&Id>,
) -> Result<Option<String>, AppError> {
    let Some(tier_id) = tier_id else {
        return Ok(None);
    };
    let tier = pricing::tier(conn, tier_id)?;
    Ok(tier.archived_at.is_none().then_some(tier.name))
}

fn price_list_choice(conn: &Connection, id: &Id) -> Result<PriceListChoice, AppError> {
    let list = pricing::price_lists(conn)?
        .into_iter()
        .find(|list| list.id == *id)
        .ok_or_else(|| AppError::internal(format!("price list {id} is gone")))?;
    Ok(PriceListChoice {
        id: list.id,
        name: list.name,
    })
}

/// A piece line counts its pieces; a weighed or measured line counts as one item.
fn items_on_line(line: &PricedLine) -> u32 {
    if line.unit.decimals == 0 {
        u32::try_from(line.line.qty.0 / 1000).unwrap_or(u32::MAX)
    } else {
        1
    }
}

/// The cart's stored discount, in the form the pricing engine takes.
pub(crate) fn manual_discount(discount: Discount) -> ManualDiscount {
    match discount {
        Discount::Percent(rate) => ManualDiscount::Percent(rate),
        Discount::Amount(amount) => ManualDiscount::Amount(amount),
    }
}

/// Running totals over the lines.
#[derive(Default)]
struct Sums {
    base: Money,
    discount: Money,
    net: Money,
    tax: Money,
    gross: Money,
}

impl Sums {
    fn add(&mut self, base: Money, discount: Money, line_tax: &LineTax) -> Result<(), AppError> {
        self.base = self.base.checked_add(base)?;
        self.discount = self.discount.checked_add(discount)?;
        self.net = self.net.checked_add(line_tax.net)?;
        self.tax = self.tax.checked_add(line_tax.tax)?;
        self.gross = self.gross.checked_add(line_tax.gross)?;
        Ok(())
    }
}
