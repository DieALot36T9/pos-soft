//! Reading what was typed or scanned into the scan box: a quantity before a star,
//! digits that are a code, scale-label rules, and a price sticker's quantity. Pure
//! functions, no database.

use pos_core::{AppError, Money, Qty, parse_money, round_half_away};

/// Rule CHK-08: a price sticker's quantity is its price ÷ the price per unit, for stock.
/// Example: 150.00 on cheese at 1,200.00/kg is 0.125 kg.
pub(crate) fn qty_for_sticker_price(price: Money, unit_price: Money) -> Result<Qty, AppError> {
    if unit_price.0 <= 0 {
        return Err(AppError::validation("sticker_product_has_no_price"));
    }
    let thousandths = round_half_away(i128::from(price.0) * 1000, i128::from(unit_price.0))?;
    Ok(Qty(thousandths.max(1)))
}

/// "3*8901234567890" → (3 pieces, "8901234567890"). No star → no multiplier.
pub(crate) fn split_multiplier(input: &str) -> Result<(Option<Qty>, &str), AppError> {
    let input = input.trim();
    let Some((count, code)) = input.split_once('*') else {
        return Ok((None, input));
    };
    let qty = parse_money(count.trim(), 3).map_err(|_| field_error("qty", "qty_invalid"))?;
    if qty.0 <= 0 {
        return Err(field_error("qty", "qty_not_positive"));
    }
    Ok((Some(Qty(qty.0)), code.trim()))
}

/// Barcodes and SKUs have no spaces; a name search usually has letters and spaces.
/// Digits only (like "123") are always treated as a code (CHK-02).
pub(crate) fn looks_like_code(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

pub(crate) fn to_barcode_rule(rule: &settings::ScaleRule) -> barcode::ScaleRule {
    barcode::ScaleRule {
        name: rule.name.clone(),
        pattern: rule.pattern.clone(),
        kind: match rule.kind {
            settings::ScaleKind::Weight => barcode::ScaleKind::Weight,
            settings::ScaleKind::Price => barcode::ScaleKind::Price,
        },
    }
}

fn field_error(field: &str, code: &str) -> AppError {
    AppError::Validation {
        field: Some(field.to_string()),
        code: code.to_string(),
        params: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chk_04_a_star_before_the_code_is_the_quantity() {
        assert_eq!(split_multiplier("3*890").unwrap(), (Some(Qty(3000)), "890"));
        assert_eq!(split_multiplier(" 890 ").unwrap(), (None, "890"));
        assert!(split_multiplier("0*890").is_err());
        assert!(split_multiplier("x*890").is_err());
    }

    #[test]
    fn chk_08_sticker_price_gives_the_quantity_for_stock() {
        assert_eq!(
            qty_for_sticker_price(Money(15_000), Money(120_000)).unwrap(),
            Qty(125)
        );
        assert!(qty_for_sticker_price(Money(15_000), Money(0)).is_err());
    }

    #[test]
    fn chk_02_digits_are_codes_and_words_are_searched() {
        assert!(looks_like_code("123"));
        assert!(!looks_like_code("milk"));
        assert!(!looks_like_code("SKU-1"));
    }
}
