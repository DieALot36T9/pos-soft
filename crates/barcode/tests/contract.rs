//! Contract tests: the public API behaves as the module page says
//! (`docs/05 Modules/Module - barcode.md`). Each test names the requirement it covers.
//! This module has no tables, so there are no migrations or views to test.

use barcode::{
    ScaleKind, ScaleRule, Scanned, Symbology, default_rules, ean13_check_digit, internal_ean13,
    is_valid_gtin, parse, rules_overlap, validate_rule,
};
use pos_core::{AppError, Money, Qty};

fn rule(name: &str, pattern: &str, kind: ScaleKind) -> ScaleRule {
    ScaleRule {
        name: name.to_string(),
        pattern: pattern.to_string(),
        kind,
    }
}

fn validation_code(error: AppError) -> String {
    match error {
        AppError::Validation { code, .. } => code,
        other => panic!("expected a validation error, got {other:?}"),
    }
}

// ---- Check digits (CAT-04) ----

#[test]
fn cat04_known_gtins_are_valid() {
    for code in [
        "4006381333931", // EAN-13
        "5901234123457", // EAN-13
        "96385074",      // EAN-8
        "036000291452",  // UPC-A
        "2100123012343", // CHK-08 weight label
        "2300456150006", // CHK-08 price label
    ] {
        assert!(is_valid_gtin(code), "{code} should be valid");
    }
}

#[test]
fn cat04_changing_any_one_digit_makes_a_gtin_invalid() {
    for code in ["4006381333931", "96385074", "036000291452"] {
        for position in 0..code.len() {
            let mut digits: Vec<u8> = code.bytes().collect();
            // Move this digit up by one (9 becomes 0).
            digits[position] = b'0' + (digits[position] - b'0' + 1) % 10;
            let changed = String::from_utf8(digits).unwrap();
            assert!(!is_valid_gtin(&changed), "{changed} should be invalid");
        }
    }
}

#[test]
fn cat04_codes_that_are_not_gtins_are_invalid() {
    for code in [
        "",
        "123",
        "40063813339312",
        "400638133393A",
        "SKU-0001",
        " 96385074",
    ] {
        assert!(!is_valid_gtin(code), "{code:?} should be invalid");
    }
}

#[test]
fn cat04_ean13_check_digit_of_first_twelve_digits() {
    assert_eq!(ean13_check_digit("400638133393"), Some(1));
    assert_eq!(ean13_check_digit("210012301234"), Some(3));
    assert_eq!(ean13_check_digit("230045615000"), Some(6));
    assert_eq!(ean13_check_digit("40063813339"), None);
    assert_eq!(ean13_check_digit("40063813339X"), None);
}

// ---- Internal barcodes (CAT-11) ----

#[test]
fn cat11_internal_barcode_pads_the_number_and_adds_the_check_digit() {
    let code = internal_ean13("200", 1).unwrap();
    assert_eq!(code, "2000000000015");
    assert!(is_valid_gtin(&code));
    assert_eq!(internal_ean13("200", 123_456_789).unwrap(), "2001234567893");
}

#[test]
fn cat11_internal_barcode_refuses_a_number_that_does_not_fit() {
    let error = internal_ean13("200", 1_000_000_000).unwrap_err();
    assert_eq!(validation_code(error), "internal_barcode_range_full");
}

#[test]
fn cat11_internal_barcode_refuses_a_bad_prefix() {
    for prefix in ["", "2a0", "200000000000"] {
        let error = internal_ean13(prefix, 1).unwrap_err();
        assert_eq!(validation_code(error), "internal_prefix_invalid");
    }
}

#[test]
fn cat11_internal_prefix_200_does_not_overlap_the_default_scale_rules() {
    assert_eq!(rules_overlap(&default_rules(), "200"), None);
}

#[test]
fn cat11_internal_prefix_inside_a_scale_prefix_overlaps() {
    let rules = default_rules();
    assert_eq!(rules_overlap(&rules, "21"), Some("Weight".to_string()));
    assert_eq!(rules_overlap(&rules, "2300"), Some("Price".to_string()));
    assert_eq!(rules_overlap(&rules, "2"), Some("Weight".to_string()));
}

#[test]
fn cat11_scale_rule_wider_than_the_internal_prefix_overlaps() {
    let rules = vec![rule("Loose", "2......{NNDDD}", ScaleKind::Weight)];
    assert_eq!(rules_overlap(&rules, "200"), Some("Loose".to_string()));
}

#[test]
fn chk08_two_scale_rules_for_the_same_codes_overlap() {
    let rules = vec![
        rule("Weight", "21.....{NNDDD}", ScaleKind::Weight),
        rule("Weight again", "21.....{NNNDD}", ScaleKind::Weight),
    ];
    assert_eq!(
        rules_overlap(&rules, "200"),
        Some("Weight again".to_string())
    );
}

#[test]
fn chk08_ean8_rules_never_overlap_ean13_codes() {
    let rules = vec![rule("Small weight", "2..{NNDD}", ScaleKind::Weight)];
    assert_eq!(rules_overlap(&rules, "200"), None);
}

// ---- Scale-label rules (CHK-08) ----

#[test]
fn chk08_default_rules_match_odoo() {
    let rules = default_rules();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].pattern, "21.....{NNDDD}");
    assert_eq!(rules[0].kind, ScaleKind::Weight);
    assert_eq!(rules[1].pattern, "23.....{NNNDD}");
    assert_eq!(rules[1].kind, ScaleKind::Price);
    for scale_rule in &rules {
        assert_eq!(validate_rule(scale_rule), Ok(()));
    }
}

#[test]
fn chk08_weight_label_gives_item_code_and_quantity() {
    let scanned = parse("2100123012343", &default_rules(), 2);
    assert_eq!(
        scanned,
        Scanned::Weighed {
            item_code: "00123".to_string(),
            qty: Qty(1234),
        }
    );
}

#[test]
fn chk08_price_label_gives_item_code_and_exact_price() {
    let scanned = parse("2300456150006", &default_rules(), 2);
    assert_eq!(
        scanned,
        Scanned::Priced {
            item_code: "00456".to_string(),
            price: Money(15000),
        }
    );
}

#[test]
fn chk08_price_label_follows_the_currency_decimals() {
    // 150.00 on the label: 150 in a currency without decimals, 150.000 with three.
    let rules = default_rules();
    let no_decimals = parse("2300456150006", &rules, 0);
    let three_decimals = parse("2300456150006", &rules, 3);
    assert!(matches!(
        no_decimals,
        Scanned::Priced {
            price: Money(150),
            ..
        }
    ));
    assert!(matches!(
        three_decimals,
        Scanned::Priced {
            price: Money(150_000),
            ..
        }
    ));
}

#[test]
fn chk08_label_with_a_wrong_check_digit_is_treated_as_a_plain_code() {
    let scanned = parse("2100123012344", &default_rules(), 2);
    assert_eq!(scanned, Scanned::Plain("2100123012344".to_string()));
}

#[test]
fn chk08_codes_that_match_no_rule_stay_plain() {
    let rules = default_rules();
    for code in ["4006381333931", "2000000000015", "SKU-0001", "96385074"] {
        assert_eq!(parse(code, &rules, 2), Scanned::Plain(code.to_string()));
    }
}

#[test]
fn chk08_scanner_line_endings_are_ignored() {
    let scanned = parse("2100123012343\r\n", &default_rules(), 2);
    assert!(matches!(scanned, Scanned::Weighed { qty: Qty(1234), .. }));
    assert_eq!(
        parse(" 4006381333931\n", &[], 2),
        Scanned::Plain("4006381333931".to_string())
    );
}

#[test]
fn chk08_custom_rules_work() {
    // Item code of 4 digits, weight with 2 decimals, prefix 22.
    let rules = vec![rule("Deli", "22....{NNNNDD}", ScaleKind::Weight)];
    let first12 = "220042001250";
    let code = format!("{first12}{}", ean13_check_digit(first12).unwrap());
    assert_eq!(
        parse(&code, &rules, 2),
        Scanned::Weighed {
            item_code: "0042".to_string(),
            qty: Qty(12_500),
        }
    );
}

#[test]
fn chk08_ean8_rule_works() {
    let rules = vec![rule("Small price", "2..{NNDD}", ScaleKind::Price)];
    // 2 | 07 | 1250 | check digit: item 07, price 12.50
    let first7 = "2071250";
    let code = format!("{first7}{}", ean8_check(first7));
    assert_eq!(
        parse(&code, &rules, 2),
        Scanned::Priced {
            item_code: "07".to_string(),
            price: Money(1250),
        }
    );
}

#[test]
fn chk08_first_matching_rule_wins_and_broken_rules_are_skipped() {
    let rules = vec![
        rule("Broken", "21....{NN", ScaleKind::Price),
        rule("Weight", "21.....{NNDDD}", ScaleKind::Weight),
        rule("Also weight", "21.....{NNNNN}", ScaleKind::Weight),
    ];
    assert!(matches!(
        parse("2100123012343", &rules, 2),
        Scanned::Weighed { qty: Qty(1234), .. }
    ));
}

#[test]
fn chk08_malformed_patterns_are_rejected() {
    let cases = [
        ("21.....{NNDD}", "scale_rule_pattern_length"),
        ("21.....{NNDDDD}", "scale_rule_pattern_length"),
        ("21....x{NNDDD}", "scale_rule_pattern_character"),
        ("21.....NNDDD", "scale_rule_pattern_character"),
        ("21..{NN}..{DDD}", "scale_rule_value_twice"),
        ("21.....{NNDDD", "scale_rule_pattern_braces"),
        ("21.....{NN.DD}", "scale_rule_pattern_character"),
        ("21.....}NNDDD{", "scale_rule_pattern_braces"),
        ("21...{N{NDDD}}", "scale_rule_pattern_braces"),
        ("21.....{DDNNN}", "scale_rule_value_order"),
        ("21.....{}12345", "scale_rule_value_missing"),
        ("2112345{NNDDD}", "scale_rule_item_code_missing"),
        (".1.....{NNDDD}", "scale_rule_prefix_missing"),
        ("", "scale_rule_value_missing"),
    ];
    for (pattern, expected) in cases {
        let error = validate_rule(&rule("Test", pattern, ScaleKind::Weight)).unwrap_err();
        assert_eq!(validation_code(error), expected, "pattern {pattern:?}");
    }
}

#[test]
fn chk08_rule_without_a_name_is_rejected() {
    let error = validate_rule(&rule("  ", "21.....{NNDDD}", ScaleKind::Weight)).unwrap_err();
    assert_eq!(validation_code(error), "scale_rule_name_missing");
}

#[test]
fn chk08_rule_errors_point_at_the_pattern_field() {
    let error = validate_rule(&rule("Test", "21", ScaleKind::Weight)).unwrap_err();
    assert!(matches!(error, AppError::Validation { field: Some(field), .. } if field == "pattern"));
}

// ---- Pictures (CAT-12) ----

#[test]
fn cat12_svg_is_refused_for_a_code_the_symbology_cannot_hold() {
    let options = barcode::SvgOpts::default();
    let error = barcode::svg("4006381333932", Symbology::Ean13, &options).unwrap_err();
    assert_eq!(validation_code(error), "barcode_not_drawable");
    let error = barcode::svg("café", Symbology::Code128, &options).unwrap_err();
    assert_eq!(validation_code(error), "barcode_not_drawable");
}

#[test]
fn cat12_bitmap_is_refused_when_the_label_is_too_narrow() {
    let error = barcode::bitmap("4006381333931", Symbology::Ean13, 50, 40).unwrap_err();
    assert_eq!(validation_code(error), "barcode_too_wide");
}

/// EAN-8 check digit, worked out here so the test doesn't trust the code it tests.
fn ean8_check(first7: &str) -> u8 {
    let mut sum = 0;
    for (index, byte) in first7.bytes().enumerate() {
        let digit = u32::from(byte - b'0');
        sum += if index % 2 == 0 { digit * 3 } else { digit };
    }
    ((10 - sum % 10) % 10) as u8
}
