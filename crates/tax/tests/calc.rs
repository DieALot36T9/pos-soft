//! The tax calculation: the TAX-05 table and examples E4–E7 from
//! `docs/03 Architecture/Money Tax and Rounding.md`. Each row is one test.

use pos_core::{Bps, Id, Money, TaxMode};
use tax::{LineTax, RateComponent, RateWithComponents, calc_line, split_components};

fn line(net: i64, tax: i64, gross: i64) -> LineTax {
    LineTax {
        net: Money(net),
        tax: Money(tax),
        gross: Money(gross),
    }
}

fn rate_with(name: &str, rate_bps: u32, parts: &[(&str, u32)]) -> RateWithComponents {
    RateWithComponents {
        id: Id::new(),
        name: name.to_string(),
        rate_bps: Bps(rate_bps),
        active: true,
        sort: 0,
        components: parts
            .iter()
            .map(|(part_name, part_bps)| RateComponent {
                name: part_name.to_string(),
                rate_bps: Bps(*part_bps),
            })
            .collect(),
    }
}

fn split(tax: i64, rate: &RateWithComponents) -> Vec<(String, Money)> {
    split_components(Money(tax), rate).expect("test rates must split")
}

#[test]
fn tax_05_row_1_e4_inclusive_115_at_15_percent() {
    let result = calc_line(Money(11500), Bps(1500), TaxMode::Inclusive).unwrap();
    assert_eq!(result, line(10000, 1500, 11500));
}

#[test]
fn tax_05_row_2_e5_inclusive_100_at_15_percent() {
    let result = calc_line(Money(10000), Bps(1500), TaxMode::Inclusive).unwrap();
    assert_eq!(result, line(8696, 1304, 10000));
}

#[test]
fn tax_05_row_3_exclusive_100_at_15_percent() {
    let result = calc_line(Money(10000), Bps(1500), TaxMode::Exclusive).unwrap();
    assert_eq!(result, line(10000, 1500, 11500));
}

#[test]
fn tax_05_row_4_e6_exclusive_99_99_at_7_5_percent() {
    let result = calc_line(Money(9999), Bps(750), TaxMode::Exclusive).unwrap();
    assert_eq!(result, line(9999, 750, 10749));
}

#[test]
fn tax_05_row_5_zero_rate_in_either_mode() {
    for mode in [TaxMode::Inclusive, TaxMode::Exclusive] {
        let result = calc_line(Money(5000), Bps(0), mode).unwrap();
        assert_eq!(result, line(5000, 0, 5000));
    }
}

#[test]
fn tax_05_rounds_half_away_from_zero() {
    // Exclusive 0.10 at 5% = 0.005 tax, which rounds up to 0.01.
    let result = calc_line(Money(10), Bps(500), TaxMode::Exclusive).unwrap();
    assert_eq!(result, line(10, 1, 11));
    // A return line: −0.10 at 5% = −0.005 tax, which rounds to −0.01.
    let result = calc_line(Money(-10), Bps(500), TaxMode::Exclusive).unwrap();
    assert_eq!(result, line(-10, -1, -11));
}

#[test]
fn tax_05_negative_amounts_mirror_sales() {
    let result = calc_line(Money(-10000), Bps(1500), TaxMode::Inclusive).unwrap();
    assert_eq!(result, line(-8696, -1304, -10000));
}

#[test]
fn tax_05_amount_too_large_is_an_error_not_a_wrap() {
    assert!(calc_line(Money(i64::MAX), Bps(1500), TaxMode::Exclusive).is_err());
}

#[test]
fn tax_03_e7_gst_18_split_into_cgst_and_sgst() {
    let gst = rate_with("GST 18%", 1800, &[("CGST", 900), ("SGST", 900)]);
    assert_eq!(
        split(1305, &gst),
        vec![
            ("CGST".to_string(), Money(652)),
            ("SGST".to_string(), Money(653)),
        ]
    );
}

#[test]
fn tax_03_remainder_goes_to_the_last_component() {
    let rate = rate_with("Mixed 10%", 1000, &[("A", 300), ("B", 300), ("C", 400)]);
    // 1.01 × 3/10 = 0.303 → 0.30 each for A and B; C gets 1.01 − 0.60 = 0.41.
    assert_eq!(
        split(101, &rate),
        vec![
            ("A".to_string(), Money(30)),
            ("B".to_string(), Money(30)),
            ("C".to_string(), Money(41)),
        ]
    );
}

#[test]
fn tax_03_refund_tax_splits_like_the_sale() {
    let gst = rate_with("GST 18%", 1800, &[("CGST", 900), ("SGST", 900)]);
    assert_eq!(
        split(-1305, &gst),
        vec![
            ("CGST".to_string(), Money(-652)),
            ("SGST".to_string(), Money(-653)),
        ]
    );
}

#[test]
fn tax_03_rate_without_components_is_one_part_named_after_the_rate() {
    let standard = rate_with("Standard 15%", 1500, &[]);
    assert_eq!(
        split(1500, &standard),
        vec![("Standard 15%".to_string(), Money(1500))]
    );
}

#[test]
fn tax_03_zero_rate_with_components_gives_zero_parts() {
    let exempt = rate_with("Exempt", 0, &[("State", 0), ("City", 0)]);
    assert_eq!(
        split(0, &exempt),
        vec![
            ("State".to_string(), Money(0)),
            ("City".to_string(), Money(0)),
        ]
    );
}

#[test]
fn tax_04_mode_is_sent_as_snake_case_text() {
    // The setup wizard stores the shop's mode; the UI sends it as text.
    assert_eq!(
        serde_json::to_string(&TaxMode::Inclusive).unwrap(),
        r#""inclusive""#
    );
    assert_eq!(
        serde_json::to_string(&TaxMode::Exclusive).unwrap(),
        r#""exclusive""#
    );
}
