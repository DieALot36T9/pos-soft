//! Property tests: rules that must hold for any input, not just the examples.
//! See "Property tests" in `docs/03 Architecture/Money Tax and Rounding.md`.

use pos_core::{
    CurrencyFormat, Grouping, Money, RoundingAppliesTo, RoundingMode, RoundingRule, allocate,
    cash_round, format_money, parse_money, round_half_away,
};
use proptest::prelude::*;

/// A second, differently written rounding, used to check `round_half_away`.
/// It works on the size of the number, then puts the sign back.
fn reference_round(numer: i128, denom: i128) -> i128 {
    let negative = (numer < 0) != (denom < 0);
    let size = (2 * numer.abs() + denom.abs()) / (2 * denom.abs());
    if negative { -size } else { size }
}

fn plain_format(minor_digits: u8) -> CurrencyFormat {
    CurrencyFormat {
        code: "XXX".to_string(),
        symbol: String::new(),
        symbol_before: true,
        minor_digits,
        decimal_sep: ".".to_string(),
        group_sep: String::new(),
        grouping: Grouping::Western,
    }
}

proptest! {
    #[test]
    fn round_half_away_matches_the_reference(
        numer in -1_000_000_000_000i128..1_000_000_000_000,
        denom in prop_oneof![1i128..100_000, -100_000i128..-1],
    ) {
        let rounded = round_half_away(numer, denom).unwrap();
        prop_assert_eq!(i128::from(rounded), reference_round(numer, denom));
    }

    #[test]
    fn allocate_shares_add_up_exactly(
        total in -10_000_000i64..10_000_000,
        weights in prop::collection::vec(0i64..1_000_000, 1..20),
    ) {
        let weights: Vec<Money> = weights.into_iter().map(Money).collect();
        let shares = allocate(Money(total), &weights).unwrap();
        prop_assert_eq!(shares.len(), weights.len());
        prop_assert_eq!(Money::sum(&shares).unwrap(), Money(total));
    }

    #[test]
    fn nearest_cash_rounding_is_never_more_than_half_the_increment(
        amount in -100_000_000i64..100_000_000,
        increment in 1i64..10_000,
    ) {
        let rule = RoundingRule {
            increment_minor: Money(increment),
            mode: RoundingMode::Nearest,
            applies_to: RoundingAppliesTo::AllPayments,
        };
        let rounded = cash_round(Money(amount), &rule).unwrap();
        prop_assert_eq!(rounded.0 % increment, 0);
        prop_assert!(2 * (rounded.0 - amount).abs() <= increment);
    }

    #[test]
    fn parse_reads_back_what_format_wrote(
        minor in -1_000_000_000_000i64..1_000_000_000_000,
        minor_digits in 0u8..=3,
    ) {
        let text = format_money(Money(minor), &plain_format(minor_digits));
        prop_assert_eq!(parse_money(&text, minor_digits), Ok(Money(minor)));
    }
}
