//! Moving average cost when goods are received (ADR-018).

use crate::{AppError, Money, Qty, round_half_away};

/// The new unit cost after receiving goods, as a weighted average of old and new stock.
/// If there was no stock (or negative stock), the new cost is simply the received cost.
/// Example: 10 on hand at 50.00, receive 20 at 56.00 → 54.00.
pub fn moving_average(
    on_hand: Qty,
    old_cost: Money,
    received: Qty,
    received_cost: Money,
) -> Result<Money, AppError> {
    if on_hand.0 <= 0 {
        return Ok(received_cost);
    }
    let total_qty = i128::from(on_hand.0) + i128::from(received.0);
    if total_qty <= 0 {
        return Ok(received_cost);
    }
    let old_value = i128::from(on_hand.0) * i128::from(old_cost.0);
    let new_value = i128::from(received.0) * i128::from(received_cost.0);
    round_half_away(old_value + new_value, total_qty).map(Money)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_stock_takes_the_received_cost() {
        assert_eq!(
            moving_average(Qty(0), Money(5000), Qty(1000), Money(6000)).unwrap(),
            Money(6000)
        );
        assert_eq!(
            moving_average(Qty(-3000), Money(5000), Qty(1000), Money(6000)).unwrap(),
            Money(6000)
        );
    }

    #[test]
    fn average_rounds_half_away_from_zero() {
        // (1 × 1.00 + 2 × 1.01) / 3 = 1.00666 → 1.01
        assert_eq!(
            moving_average(Qty(1000), Money(100), Qty(2000), Money(101)).unwrap(),
            Money(101)
        );
    }
}
