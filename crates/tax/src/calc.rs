//! The tax calculation. Pure functions: no database.
//! Rules: `docs/03 Architecture/Money Tax and Rounding.md`, "Line calculation" steps 7 and 8.

use pos_core::{AppError, Bps, Money, TaxMode, percent_of, round_half_away};

use crate::{LineTax, RateWithComponents, Result};

/// Works out the net, tax and gross of one sale line, after its discounts (rule TAX-05).
/// Tax is rounded per line, half away from zero. Example: 100.00 inclusive at 15% → tax 13.04.
pub fn calc_line(amount: Money, rate: Bps, mode: TaxMode) -> Result<LineTax> {
    match mode {
        TaxMode::Inclusive => {
            // The amount is net × (1 + r), so the tax inside it is amount × r / (10000 + r).
            let rate_bps = i128::from(rate.0);
            let tax = round_half_away(i128::from(amount.0) * rate_bps, 10_000 + rate_bps)?;
            let tax = Money(tax);
            Ok(LineTax {
                net: amount.checked_sub(tax)?,
                tax,
                gross: amount,
            })
        }
        TaxMode::Exclusive => {
            let tax = percent_of(amount, rate)?;
            Ok(LineTax {
                net: amount,
                tax,
                gross: amount.checked_add(tax)?,
            })
        }
    }
}

/// Splits a line's tax across the rate's named parts, for receipts and reports (rule TAX-03).
/// Each part gets its share rounded towards zero, and the last part gets what is left,
/// so the parts always add up to the tax. A rate with no parts gives one part named after it.
/// Example: 13.05 tax on "GST 18%" = CGST 9% + SGST 9% → CGST 6.52, SGST 6.53.
pub fn split_components(tax: Money, rate: &RateWithComponents) -> Result<Vec<(String, Money)>> {
    let Some((last, others)) = rate.components.split_last() else {
        return Ok(vec![(rate.name.clone(), tax)]);
    };
    let mut parts = Vec::with_capacity(rate.components.len());
    let mut given_out = Money::zero();
    for component in others {
        let share = share_of(tax, component.rate_bps, rate.rate_bps)?;
        given_out = given_out.checked_add(share)?;
        parts.push((component.name.clone(), share));
    }
    parts.push((last.name.clone(), tax.checked_sub(given_out)?));
    Ok(parts)
}

/// tax × part / whole, rounded towards zero. Rounding towards zero (not down) means a
/// refund splits into the same sizes as the sale, only negative.
fn share_of(tax: Money, part: Bps, whole: Bps) -> Result<Money> {
    if whole.0 == 0 {
        return Ok(Money::zero());
    }
    // Rust's integer division already goes towards zero.
    let share = i128::from(tax.0) * i128::from(part.0) / i128::from(whole.0);
    let share = i64::try_from(share)
        .map_err(|_| AppError::internal(format!("tax share {share} is too large")))?;
    Ok(Money(share))
}
