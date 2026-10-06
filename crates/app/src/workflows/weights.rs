//! Weights for weighed items (CHK-06, CHK-07): typed by hand or read from the scale,
//! turned into a quantity in the product's own unit.

use hardware::{Reading, WeightUnit};
use pos_core::{AppError, Qty, parse_money, round_half_away};

/// Thousandths: a quantity of 1.250 kg is 1250 (ADR-007).
const QTY_DIGITS: u8 = 3;

/// Reads a weight typed in the weight dialog. A number alone is in the product's unit;
/// "g" or "kg" after it says the unit. Example for a product sold by kg: "1.25" and
/// "1250 g" are both 1.250 kg (CHK-06).
pub fn parse_weight(text: &str, unit_symbol: &str) -> Result<Qty, AppError> {
    let typed = text.trim().to_lowercase();
    let (number, typed_unit) = if let Some(rest) = typed.strip_suffix("kg") {
        (rest, Some(WeightUnit::Kg))
    } else if let Some(rest) = typed.strip_suffix('g') {
        (rest, Some(WeightUnit::G))
    } else {
        (typed.as_str(), None)
    };
    let value =
        parse_money(number.trim(), QTY_DIGITS).map_err(|_| weight_error("weight_invalid"))?;
    let qty = match typed_unit {
        None => Qty(value.0),
        Some(unit) => convert(value.0, unit, unit_symbol)?,
    };
    positive(qty)
}

/// Turns a scale reading into a quantity in the product's unit. Only a stable weight
/// above zero may be sold (CHK-07).
pub fn weight_from_reading(reading: &Reading, unit_symbol: &str) -> Result<Qty, AppError> {
    if !reading.stable {
        return Err(weight_error("weight_not_stable"));
    }
    let qty = convert(reading.value_milli, reading.unit, unit_symbol)?;
    positive(qty)
}

/// Thousandths of `from` as thousandths of the product's unit (kg or g only).
fn convert(value_milli: i64, from: WeightUnit, unit_symbol: &str) -> Result<Qty, AppError> {
    let to = match unit_symbol.trim().to_lowercase().as_str() {
        "kg" => WeightUnit::Kg,
        "g" => WeightUnit::G,
        _ => return Err(weight_error("weight_unit_wrong")),
    };
    let converted = match (from, to) {
        (WeightUnit::Kg, WeightUnit::Kg) | (WeightUnit::G, WeightUnit::G) => value_milli,
        (WeightUnit::G, WeightUnit::Kg) => round_half_away(i128::from(value_milli), 1000)?,
        (WeightUnit::Kg, WeightUnit::G) => value_milli
            .checked_mul(1000)
            .ok_or_else(|| weight_error("weight_invalid"))?,
        _ => return Err(weight_error("weight_unit_wrong")),
    };
    Ok(Qty(converted))
}

fn positive(qty: Qty) -> Result<Qty, AppError> {
    if qty.0 <= 0 {
        return Err(weight_error("weight_not_positive"));
    }
    Ok(qty)
}

fn weight_error(code: &str) -> AppError {
    AppError::Validation {
        field: Some("weight".to_string()),
        code: code.to_string(),
        params: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(value_milli: i64, unit: WeightUnit, stable: bool) -> Reading {
        Reading {
            value_milli,
            unit,
            stable,
            raw: String::new(),
        }
    }

    #[test]
    fn chk_06_typed_weights_in_kg_or_g() {
        assert_eq!(parse_weight("1.25", "kg").unwrap(), Qty(1250));
        assert_eq!(parse_weight("1250 g", "kg").unwrap(), Qty(1250));
        assert_eq!(parse_weight("1.25kg", "kg").unwrap(), Qty(1250));
        assert_eq!(parse_weight(" 0.5 KG ", "g").unwrap(), Qty(500_000));
    }

    #[test]
    fn chk_06_bad_weights_are_refused() {
        for (text, code) in [
            ("abc", "weight_invalid"),
            ("0", "weight_not_positive"),
            ("-1", "weight_not_positive"),
            ("2 kg", "weight_unit_wrong"),
        ] {
            let unit = if code == "weight_unit_wrong" {
                "pc"
            } else {
                "kg"
            };
            match parse_weight(text, unit) {
                Err(AppError::Validation { code: found, .. }) => assert_eq!(found, code, "{text}"),
                other => panic!("{text}: {other:?}"),
            }
        }
    }

    #[test]
    fn chk_07_only_a_stable_weight_above_zero_counts() {
        assert_eq!(
            weight_from_reading(&reading(1234, WeightUnit::Kg, true), "kg").unwrap(),
            Qty(1234)
        );
        assert_eq!(
            weight_from_reading(&reading(1_234_000, WeightUnit::G, true), "kg").unwrap(),
            Qty(1234)
        );
        assert!(weight_from_reading(&reading(1234, WeightUnit::Kg, false), "kg").is_err());
        assert!(weight_from_reading(&reading(0, WeightUnit::Kg, true), "kg").is_err());
        assert!(weight_from_reading(&reading(1000, WeightUnit::Lb, true), "kg").is_err());
    }
}
