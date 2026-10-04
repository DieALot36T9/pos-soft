//! The worked examples from `docs/03 Architecture/Money Tax and Rounding.md`.
//! Each test is named after its example number (E1 to E16).
//!
//! Some examples belong to other modules: E7 (tax components) to `tax`,
//! E12 and E13 (change) to `sales`, E16 (points) to `customers`. Here we test
//! the `core` building blocks those modules use, with the same numbers.

use pos_core::{
    Bps, Money, Qty, RoundingAppliesTo, RoundingMode, RoundingRule, allocate, cash_round,
    moving_average, mul_qty, parse_money, percent_of, round_half_away,
};

/// Turns "123.45" into Money with 2 decimals, so tests read like the vault.
fn money(text: &str) -> Money {
    parse_money(text, 2).expect("test amount must be valid")
}

fn nearest_to_whole_unit() -> RoundingRule {
    RoundingRule {
        increment_minor: money("1.00"),
        mode: RoundingMode::Nearest,
        applies_to: RoundingAppliesTo::AllPayments,
    }
}

#[test]
fn e1_piece_item_base_amount() {
    assert_eq!(mul_qty(money("90.00"), Qty(2000)).unwrap(), money("180.00"));
}

#[test]
fn e2_weighed_item_base_amount() {
    assert_eq!(
        mul_qty(money("120.00"), Qty(1234)).unwrap(),
        money("148.08")
    );
}

#[test]
fn e3_weighed_item_rounds_half_away_from_zero() {
    // 9999 × 333 / 1000 = 3329.667 minor units, which rounds to 3330.
    assert_eq!(mul_qty(money("99.99"), Qty(333)).unwrap(), money("33.30"));
}

#[test]
fn e4_inclusive_tax_building_block() {
    // tax = round(amount × r / (10000 + r)), used by the tax module.
    let tax = round_half_away(11_500 * 1_500, 10_000 + 1_500).unwrap();
    assert_eq!(Money(tax), money("15.00"));
    assert_eq!(
        money("115.00").checked_sub(Money(tax)).unwrap(),
        money("100.00")
    );
}

#[test]
fn e5_inclusive_tax_building_block_with_rounding() {
    let tax = round_half_away(10_000 * 1_500, 10_000 + 1_500).unwrap();
    assert_eq!(Money(tax), money("13.04"));
    assert_eq!(
        money("100.00").checked_sub(Money(tax)).unwrap(),
        money("86.96")
    );
}

#[test]
fn e6_exclusive_tax_is_a_percent_of_the_amount() {
    let tax = percent_of(money("99.99"), Bps(750)).unwrap();
    assert_eq!(tax, money("7.50"));
    assert_eq!(money("99.99").checked_add(tax).unwrap(), money("107.49"));
}

#[test]
fn e7_tax_components_building_block() {
    // Each component gets floor(tax × comp / rate); the last one gets the rest.
    let tax = money("13.05");
    let first = Money(tax.0 * 900 / 1800);
    let last = tax.checked_sub(first).unwrap();
    assert_eq!(first, money("6.52"));
    assert_eq!(last, money("6.53"));
}

#[test]
fn e8_invoice_discount_split_in_proportion() {
    let shares = allocate(money("10.00"), &[money("30.00"), money("70.00")]).unwrap();
    assert_eq!(shares, vec![money("3.00"), money("7.00")]);
}

#[test]
fn e9_invoice_discount_remainder_goes_to_the_earliest_line() {
    let lines = [money("10.00"), money("10.00"), money("10.00")];
    let shares = allocate(money("10.00"), &lines).unwrap();
    assert_eq!(shares, vec![money("3.34"), money("3.33"), money("3.33")]);
}

#[test]
fn e10_cash_rounding_down_to_nearest_whole_unit() {
    let due = cash_round(money("123.45"), &nearest_to_whole_unit()).unwrap();
    assert_eq!(due, money("123.00"));
    assert_eq!(due.checked_sub(money("123.45")).unwrap(), money("-0.45"));
}

#[test]
fn e11_cash_rounding_half_goes_up() {
    let due = cash_round(money("123.50"), &nearest_to_whole_unit()).unwrap();
    assert_eq!(due, money("124.00"));
    assert_eq!(due.checked_sub(money("123.50")).unwrap(), money("0.50"));
}

#[test]
fn e12_change_building_block() {
    let change = money("500.00").checked_sub(money("470.00")).unwrap();
    assert_eq!(change, money("30.00"));
}

#[test]
fn e13_split_payment_change_building_block() {
    let left_for_cash = money("1000.00").checked_sub(money("400.00")).unwrap();
    let change = money("700.00").checked_sub(left_for_cash).unwrap();
    assert_eq!(change, money("100.00"));
}

#[test]
fn e14_partial_returns_add_up_to_the_line() {
    // Return 1 of 3, then 1 of 3, then the last one gets exactly what remains.
    let gross = money("100.00");
    let first = Money(round_half_away(i128::from(gross.0), 3).unwrap());
    let second = Money(round_half_away(i128::from(gross.0), 3).unwrap());
    let last = Money::sum(&[
        gross,
        first.checked_neg().unwrap(),
        second.checked_neg().unwrap(),
    ])
    .unwrap();
    assert_eq!(
        (first, second, last),
        (money("33.33"), money("33.33"), money("33.34"))
    );
}

#[test]
fn e15_moving_average_cost() {
    let cost = moving_average(Qty(10_000), money("50.00"), Qty(20_000), money("56.00")).unwrap();
    assert_eq!(cost, money("54.00"));
}

#[test]
fn e16_points_building_block() {
    // earned = floor(eligible / spend unit) × points per unit
    let eligible = money("1250.00");
    let spend_unit = money("100.00");
    assert_eq!(eligible.0 / spend_unit.0, 12);
}
