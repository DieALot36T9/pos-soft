//! First expiry, first out (INV-05): which batches a stock-out takes from.
//! A pure rule with no database, so it's easy to test.

use pos_core::{Id, Qty};

use crate::InventoryError;
use crate::convert::negate_qty;

/// Splits a stock-out over batches. `batches` are in FEFO order (earliest expiry
/// first) with the stock each holds. Each batch gives at most what it holds. Anything
/// left over gets no batch (ADR-020), so the product can go below zero.
/// Example: 6 out of batches holding [2, 3, 5] → [−2, −3, −1].
pub(crate) fn split_fefo(
    qty_out: Qty,
    batches: &[(Id, Qty)],
) -> Result<Vec<(Option<Id>, Qty)>, InventoryError> {
    let mut still_needed = negate_qty(qty_out)?.0;
    let mut parts = Vec::new();
    for (batch_id, on_hand) in batches {
        if still_needed <= 0 {
            break;
        }
        if on_hand.0 <= 0 {
            continue;
        }
        let taken = still_needed.min(on_hand.0);
        parts.push((Some(batch_id.clone()), Qty(-taken)));
        still_needed -= taken;
    }
    if still_needed > 0 {
        parts.push((None, Qty(-still_needed)));
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batches(stock: &[i64]) -> Vec<(Id, Qty)> {
        stock.iter().map(|milli| (Id::new(), Qty(*milli))).collect()
    }

    #[test]
    fn takes_whole_batches_in_order_then_part_of_the_next() {
        let held = batches(&[2_000, 3_000, 5_000]);
        let parts = split_fefo(Qty(-6_000), &held).unwrap();
        assert_eq!(
            parts,
            vec![
                (Some(held[0].0.clone()), Qty(-2_000)),
                (Some(held[1].0.clone()), Qty(-3_000)),
                (Some(held[2].0.clone()), Qty(-1_000)),
            ]
        );
    }

    #[test]
    fn skips_empty_and_negative_batches() {
        let held = batches(&[0, -1_000, 4_000]);
        let parts = split_fefo(Qty(-1_500), &held).unwrap();
        assert_eq!(parts, vec![(Some(held[2].0.clone()), Qty(-1_500))]);
    }

    #[test]
    fn the_rest_goes_without_a_batch() {
        let held = batches(&[1_000]);
        let parts = split_fefo(Qty(-2_500), &held).unwrap();
        assert_eq!(
            parts,
            vec![(Some(held[0].0.clone()), Qty(-1_000)), (None, Qty(-1_500))]
        );
    }

    #[test]
    fn no_batches_means_one_row_without_a_batch() {
        assert_eq!(split_fefo(Qty(-750), &[]).unwrap(), vec![(None, Qty(-750))]);
    }

    #[test]
    fn the_parts_always_add_up_to_the_stock_out() {
        let held = batches(&[300, 0, 1_200, 50]);
        for wanted in [1, 299, 300, 1_550, 9_999] {
            let parts = split_fefo(Qty(-wanted), &held).unwrap();
            let total: i64 = parts.iter().map(|(_, qty)| qty.0).sum();
            assert_eq!(total, -wanted);
        }
    }
}
