//! The pricing engine: a pure function from cart lines to priced lines.
//! No database and no clock: everything comes in through `PricingContext`.
//! The steps follow "Line calculation" in Money Tax and Rounding.

use pos_core::{Bps, Id, Money, Qty, allocate, mul_qty, percent_of};

use crate::error::invalid;
use crate::promotion::saving;
use crate::{
    AppliedPromo, ManualDiscount, PriceLineIn, PriceLineOut, PricingContext, PricingError,
    Promotion,
};

/// Prices every cart line: unit price, promotion, tier discount, manual discount and its
/// share of the invoice discount. Lines come back in the order they went in.
pub fn price_cart(
    ctx: &PricingContext,
    lines: &[PriceLineIn],
    invoice_discount: Option<ManualDiscount>,
) -> Result<Vec<PriceLineOut>, PricingError> {
    check_inputs(lines, invoice_discount)?;
    let mut priced = Vec::with_capacity(lines.len());
    for line in lines {
        priced.push(start_line(ctx, line)?);
    }
    apply_promotions(ctx, lines, &mut priced)?;
    apply_tier_discount(ctx, lines, &mut priced)?;
    apply_manual_discounts(lines, &mut priced)?;
    apply_invoice_discount(invoice_discount, &mut priced)?;
    for line in &mut priced {
        line.amount = remaining(line)?.checked_sub(line.invoice_share)?;
    }
    Ok(priced)
}

/// The largest manual discount in the cart as a percentage, to compare with the role's
/// limit (PRC-07). Each line's discount counts against what the line cost before it. The
/// invoice discount counts against the subtotal before it. Rounded up, so a discount just
/// over the limit never passes as equal to it. Example: 15.01 off 300.00 is 5.01%.
pub fn manual_discount_bps(lines: &[PriceLineOut]) -> Bps {
    let mut highest = 0;
    let mut subtotal_before_invoice: i128 = 0;
    let mut invoice_total: i128 = 0;
    for line in lines {
        let before_manual = i128::from(line.base.0)
            - i128::from(line.promo_discount().0)
            - i128::from(line.tier_discount.0);
        let manual = i128::from(line.manual_discount.0);
        highest = highest.max(percent_rounded_up(manual, before_manual));
        subtotal_before_invoice += before_manual - manual;
        invoice_total += i128::from(line.invoice_share.0);
    }
    highest = highest.max(percent_rounded_up(invoice_total, subtotal_before_invoice));
    Bps(highest)
}

/// `part` as hundredths of a percent of `whole`, rounded up. Zero if there's nothing to measure.
fn percent_rounded_up(part: i128, whole: i128) -> u32 {
    if part <= 0 || whole <= 0 {
        return 0;
    }
    let rate = (part * 10_000 + whole - 1) / whole;
    u32::try_from(rate).unwrap_or(u32::MAX)
}

/// Refuses negative quantities, prices and discounts. The engine promises that no line
/// goes below zero, and that only holds for inputs of zero or more.
fn check_inputs(
    lines: &[PriceLineIn],
    invoice_discount: Option<ManualDiscount>,
) -> Result<(), PricingError> {
    for line in lines {
        if line.qty.0 < 0 {
            return Err(invalid("qty", "qty_negative"));
        }
        let prices = [
            Some(line.product.retail_price),
            line.embedded_price,
            line.price_override,
        ];
        if prices.iter().flatten().any(|price| price.0 < 0) {
            return Err(invalid("price", "price_negative"));
        }
        if let Some(ManualDiscount::Amount(amount)) = line.manual
            && amount.0 < 0
        {
            return Err(invalid("discount", "discount_negative"));
        }
    }
    if let Some(ManualDiscount::Amount(amount)) = invoice_discount
        && amount.0 < 0
    {
        return Err(invalid("invoice_discount", "discount_negative"));
    }
    Ok(())
}

/// Step 1: the unit price and base amount, with no discounts yet.
fn start_line(ctx: &PricingContext, line: &PriceLineIn) -> Result<PriceLineOut, PricingError> {
    let list_price = ctx.list_prices.get(&line.product.id).copied();
    // Rule CHK-13: an override wins. Then the price list (PRC-02), then retail (PRC-01).
    let unit_price = line
        .price_override
        .or(list_price)
        .unwrap_or(line.product.retail_price);
    // Rule CHK-08: a scale sticker's price is used exactly as printed.
    let base = match line.embedded_price {
        Some(sticker_price) => sticker_price,
        None => mul_qty(unit_price, line.qty)?,
    };
    Ok(PriceLineOut {
        key: line.key,
        unit_price,
        base,
        promo: None,
        tier_discount: Money::zero(),
        manual_discount: Money::zero(),
        invoice_share: Money::zero(),
        amount: base,
    })
}

/// True if the shop's automatic discounts (promotions, tier) may touch this line.
/// Scale sticker prices and overridden prices are fixed, so they get neither.
fn gets_automatic_discounts(line: &PriceLineIn) -> bool {
    line.embedded_price.is_none() && line.price_override.is_none()
}

/// Lines of the same product at the same unit price, which share one promotion (PRC-05).
struct ProductGroup {
    product_id: Id,
    unit_price: Money,
    line_indexes: Vec<usize>,
}

/// Step 2: for each product, find the promotion that saves the most on the combined
/// quantity, and split its saving over that product's lines (ADR-019).
fn apply_promotions(
    ctx: &PricingContext,
    lines: &[PriceLineIn],
    priced: &mut [PriceLineOut],
) -> Result<(), PricingError> {
    let weekday_bit = ctx.now.weekday_bit()?;
    let mut running: Vec<&Promotion> = ctx
        .promotions
        .iter()
        .filter(|promo| promo.runs_now(ctx.now.at, weekday_bit, ctx.tier.as_ref()))
        .collect();
    // Ties go to the promotion made first, so look at the oldest first.
    running.sort_by(|a, b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));

    for group in product_groups(lines, priced) {
        let first_line = &lines[group.line_indexes[0]];
        let mut total_qty: i64 = 0;
        let mut bases = Vec::with_capacity(group.line_indexes.len());
        for index in &group.line_indexes {
            total_qty = total_qty
                .checked_add(lines[*index].qty.0)
                .ok_or_else(|| invalid("qty", "qty_too_large"))?;
            bases.push(priced[*index].base);
        }
        let group_base = Money::sum(&bases)?;

        let mut best: Option<(&Promotion, Money)> = None;
        for promo in &running {
            if !promo.covers(&first_line.product) {
                continue;
            }
            let promo_saving = saving(&promo.rule, group.unit_price, Qty(total_qty), group_base)?;
            let best_saving = best.map_or(Money::zero(), |(_, amount)| amount);
            // Strictly more, so an equal saving keeps the older promotion.
            if promo_saving > best_saving {
                best = Some((promo, promo_saving));
            }
        }

        if let Some((promo, total_saving)) = best {
            // Each line gets a share in proportion to its base; leftovers go to earlier lines.
            let shares = allocate(total_saving, &bases)?;
            for (index, share) in group.line_indexes.iter().zip(shares) {
                priced[*index].promo = Some(AppliedPromo {
                    promotion_id: promo.id.clone(),
                    name: promo.name.clone(),
                    rule: promo.rule.clone(),
                    discount: share,
                });
            }
        }
    }
    Ok(())
}

/// Groups the lines that may get a promotion by product and unit price, in cart order.
fn product_groups(lines: &[PriceLineIn], priced: &[PriceLineOut]) -> Vec<ProductGroup> {
    let mut groups: Vec<ProductGroup> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !gets_automatic_discounts(line) {
            continue;
        }
        let unit_price = priced[index].unit_price;
        let existing = groups
            .iter_mut()
            .find(|group| group.product_id == line.product.id && group.unit_price == unit_price);
        match existing {
            Some(group) => group.line_indexes.push(index),
            None => groups.push(ProductGroup {
                product_id: line.product.id.clone(),
                unit_price,
                line_indexes: vec![index],
            }),
        }
    }
    groups
}

/// Step 3: the tier discount, only on lines with no promotion (PRC-03, PRC-05).
fn apply_tier_discount(
    ctx: &PricingContext,
    lines: &[PriceLineIn],
    priced: &mut [PriceLineOut],
) -> Result<(), PricingError> {
    let Some(tier) = &ctx.tier else {
        return Ok(());
    };
    for (line, out) in lines.iter().zip(priced.iter_mut()) {
        if gets_automatic_discounts(line) && out.promo.is_none() {
            out.tier_discount = percent_of(out.base, tier.discount)?.min(out.base);
        }
    }
    Ok(())
}

/// Step 4: the cashier's line discount, taken from what's left after the promotion and
/// tier discount. It can't take the line below zero (CHK-12).
fn apply_manual_discounts(
    lines: &[PriceLineIn],
    priced: &mut [PriceLineOut],
) -> Result<(), PricingError> {
    for (line, out) in lines.iter().zip(priced.iter_mut()) {
        if let Some(discount) = line.manual {
            out.manual_discount = discount_on(remaining(out)?, discount)?;
        }
    }
    Ok(())
}

/// Step 5: the invoice discount, split over the lines in proportion to what each line
/// still costs, so the shares add up exactly (core `allocate`, largest remainder).
fn apply_invoice_discount(
    invoice_discount: Option<ManualDiscount>,
    priced: &mut [PriceLineOut],
) -> Result<(), PricingError> {
    let Some(discount) = invoice_discount else {
        return Ok(());
    };
    let mut line_amounts = Vec::with_capacity(priced.len());
    for out in priced.iter() {
        line_amounts.push(remaining(out)?);
    }
    let subtotal = Money::sum(&line_amounts)?;
    let total_discount = discount_on(subtotal, discount)?;
    let shares = allocate(total_discount, &line_amounts)?;
    for (out, share) in priced.iter_mut().zip(shares) {
        out.invoice_share = share;
    }
    Ok(())
}

/// A percentage or amount off `amount`, never more than `amount` itself.
fn discount_on(amount: Money, discount: ManualDiscount) -> Result<Money, PricingError> {
    let wanted = match discount {
        ManualDiscount::Percent(rate) => percent_of(amount, rate)?,
        ManualDiscount::Amount(off) => off,
    };
    Ok(wanted.min(amount).max(Money::zero()))
}

/// What the line costs after its promotion, tier and manual discounts.
fn remaining(out: &PriceLineOut) -> Result<Money, PricingError> {
    let left = out
        .base
        .checked_sub(out.promo_discount())?
        .checked_sub(out.tier_discount)?
        .checked_sub(out.manual_discount)?;
    Ok(left)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_rounded_up_never_rounds_down() {
        assert_eq!(percent_rounded_up(1, 30_000), 1);
        assert_eq!(percent_rounded_up(1500, 30_000), 500);
        assert_eq!(percent_rounded_up(1501, 30_000), 501);
        assert_eq!(percent_rounded_up(5, 0), 0);
    }

    #[test]
    fn discount_on_stops_at_the_amount() {
        assert_eq!(
            discount_on(Money(500), ManualDiscount::Percent(Bps(12_000))).unwrap(),
            Money(500)
        );
        assert_eq!(
            discount_on(Money(500), ManualDiscount::Amount(Money(800))).unwrap(),
            Money(500)
        );
    }
}
