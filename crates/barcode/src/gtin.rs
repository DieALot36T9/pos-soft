//! Check digits for shop barcodes (GTINs) and the shop's own internal barcodes.
//! The check digit follows the GS1 General Specifications (mod 10, weights 3 and 1).

use pos_core::AppError;

/// The last digit of an EAN-13, worked out from the first 12. `None` if the input isn't 12 digits.
pub fn ean13_check_digit(first12: &str) -> Option<u8> {
    if first12.len() != 12 {
        return None;
    }
    gtin_check_digit(first12)
}

/// True if the code is an EAN-8, UPC-A or EAN-13 with the right check digit.
pub fn is_valid_gtin(code: &str) -> bool {
    if !matches!(code.len(), 8 | 12 | 13) {
        return false;
    }
    let (body, last) = code.split_at(code.len() - 1);
    match (gtin_check_digit(body), digit_value(last)) {
        (Some(expected), Some(found)) => expected == found,
        _ => false,
    }
}

/// Rule CAT-11: makes the shop's own EAN-13 from its prefix and a running number.
/// Example: prefix `200` and number 1 give `2000000000015`.
pub fn internal_ean13(prefix: &str, number: u64) -> Result<String, AppError> {
    let prefix_is_digits = !prefix.is_empty() && prefix.bytes().all(|byte| byte.is_ascii_digit());
    if !prefix_is_digits || prefix.len() > 11 {
        return Err(AppError::validation("internal_prefix_invalid"));
    }
    let number_width = 12 - prefix.len();
    let number_text = format!("{number:0number_width$}");
    if number_text.len() > number_width {
        return Err(AppError::validation("internal_barcode_range_full"));
    }
    let first12 = format!("{prefix}{number_text}");
    match ean13_check_digit(&first12) {
        Some(check) => Ok(format!("{first12}{check}")),
        None => Err(AppError::internal(format!(
            "bad internal barcode {first12}"
        ))),
    }
}

/// The GS1 check digit for any run of digits (the code without its last digit).
/// From the right, digits are multiplied by 3, 1, 3, 1, … and the check digit
/// brings the sum up to the next multiple of 10.
fn gtin_check_digit(body: &str) -> Option<u8> {
    let mut sum: u32 = 0;
    for (position_from_right, byte) in body.bytes().rev().enumerate() {
        let digit = u32::from(digit_value_of_byte(byte)?);
        let weight = if position_from_right % 2 == 0 { 3 } else { 1 };
        sum += digit * weight;
    }
    // The result is always 0 to 9, so it fits in a u8.
    Some(((10 - sum % 10) % 10) as u8)
}

fn digit_value(text: &str) -> Option<u8> {
    match text.as_bytes() {
        [byte] => digit_value_of_byte(*byte),
        _ => None,
    }
}

fn digit_value_of_byte(byte: u8) -> Option<u8> {
    if byte.is_ascii_digit() {
        Some(byte - b'0')
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cat04_check_digit_matches_gs1_examples() {
        // Examples from the GS1 check digit calculator.
        assert_eq!(gtin_check_digit("629104150021"), Some(3));
        assert_eq!(gtin_check_digit("9638507"), Some(4));
        assert_eq!(gtin_check_digit("03600029145"), Some(2));
    }

    #[test]
    fn cat04_check_digit_refuses_letters() {
        assert_eq!(gtin_check_digit("96385O7"), None);
    }

    #[test]
    fn cat11_one_digit_prefix_leaves_eleven_digits_for_the_number() {
        assert_eq!(
            internal_ean13("2", 99_999_999_999).unwrap(),
            "2999999999991"
        );
        assert!(internal_ean13("2", 100_000_000_000).is_err());
    }
}
