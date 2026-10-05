//! Property tests (proptest): the tax rules hold for any amount and any rate.
//! Rules from `docs/03 Architecture/Money Tax and Rounding.md`, "Property tests".

use pos_core::{Bps, Id, Money};
use proptest::prelude::*;
use tax::{RateComponent, RateWithComponents, TaxMode, calc_line, split_components};

// Up to ±10 billion in minor units: far more than any one sale line.
const LARGEST_AMOUNT: i64 = 1_000_000_000_000;

proptest! {
    #[test]
    fn tax_05_inclusive_net_plus_tax_is_the_amount(
        amount in -LARGEST_AMOUNT..=LARGEST_AMOUNT,
        rate in 0u32..=10_000,
    ) {
        let line = calc_line(Money(amount), Bps(rate), TaxMode::Inclusive).unwrap();
        prop_assert_eq!(line.net.0 + line.tax.0, amount);
        prop_assert_eq!(line.gross, Money(amount));
    }

    #[test]
    fn tax_05_exclusive_gross_is_amount_plus_tax(
        amount in -LARGEST_AMOUNT..=LARGEST_AMOUNT,
        rate in 0u32..=10_000,
    ) {
        let line = calc_line(Money(amount), Bps(rate), TaxMode::Exclusive).unwrap();
        prop_assert_eq!(line.gross.0, amount + line.tax.0);
        prop_assert_eq!(line.net, Money(amount));
    }

    #[test]
    fn tax_05_tax_has_the_same_sign_as_the_amount(
        amount in -LARGEST_AMOUNT..=LARGEST_AMOUNT,
        rate in 0u32..=10_000,
        inclusive in any::<bool>(),
    ) {
        let mode = if inclusive { TaxMode::Inclusive } else { TaxMode::Exclusive };
        let line = calc_line(Money(amount), Bps(rate), mode).unwrap();
        prop_assert!(line.tax.0.signum() == amount.signum() || line.tax.0 == 0);
        prop_assert!(line.tax.0.abs() <= amount.abs());
    }

    #[test]
    fn tax_03_components_add_up_to_the_tax(
        tax in -LARGEST_AMOUNT..=LARGEST_AMOUNT,
        parts in prop::collection::vec(0u32..=2_500, 1..=4),
    ) {
        let rate = RateWithComponents {
            id: Id::new(),
            name: "Rate".to_string(),
            rate_bps: Bps(parts.iter().sum()),
            active: true,
            sort: 0,
            components: parts
                .iter()
                .enumerate()
                .map(|(index, part)| RateComponent {
                    name: format!("Part {index}"),
                    rate_bps: Bps(*part),
                })
                .collect(),
        };
        let split = split_components(Money(tax), &rate).unwrap();
        prop_assert_eq!(split.len(), parts.len());
        let total: i64 = split.iter().map(|(_, amount)| amount.0).sum();
        prop_assert_eq!(total, tax);
    }
}
