//! Splitting an amount across lines so the parts add up exactly.

use crate::{AppError, Money};

/// Splits `total` across lines in proportion to `weights`, so the shares add up exactly.
/// Leftover minor units go to the largest remainders; ties go to the earliest line.
/// Example: 10.00 over three lines of 10.00 gives 3.34, 3.33, 3.33.
/// If every weight is zero, the total is split evenly.
pub fn allocate(total: Money, weights: &[Money]) -> Result<Vec<Money>, AppError> {
    if weights.is_empty() {
        if total == Money::zero() {
            return Ok(Vec::new());
        }
        return Err(AppError::internal(
            "allocate: no lines to split the amount over",
        ));
    }
    let mut weight_values: Vec<i128> = Vec::with_capacity(weights.len());
    for weight in weights {
        if weight.0 < 0 {
            return Err(AppError::internal("allocate: a weight is negative"));
        }
        weight_values.push(i128::from(weight.0));
    }
    let mut weight_total: i128 = weight_values.iter().sum();
    if weight_total == 0 {
        // Nothing to go by, so every line counts the same.
        weight_values = vec![1; weights.len()];
        weight_total = i128::try_from(weights.len()).unwrap_or(i128::MAX);
    }

    // Work with the size of the total and put the sign back at the end.
    let size = i128::from(total.0).abs();
    let mut shares: Vec<i128> = Vec::with_capacity(weights.len());
    let mut remainders: Vec<(usize, i128)> = Vec::with_capacity(weights.len());
    for (index, weight) in weight_values.iter().enumerate() {
        let exact = size * weight;
        shares.push(exact / weight_total);
        remainders.push((index, exact % weight_total));
    }

    // Hand out the minor units that are left, largest remainder first, earliest line on ties.
    let given: i128 = shares.iter().sum();
    let left_over = usize::try_from(size - given).unwrap_or(0);
    remainders.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (index, _) in remainders.iter().take(left_over) {
        shares[*index] += 1;
    }

    let mut result = Vec::with_capacity(shares.len());
    for share in shares {
        let signed = if total.0 < 0 { -share } else { share };
        let amount = i64::try_from(signed)
            .map_err(|_| AppError::internal("allocate: a share is too large"))?;
        result.push(Money(amount));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_total_splits_the_same_way() {
        let shares = allocate(Money(-1000), &[Money(1000), Money(1000), Money(1000)]).unwrap();
        assert_eq!(shares, vec![Money(-334), Money(-333), Money(-333)]);
    }

    #[test]
    fn zero_weights_split_evenly() {
        let shares = allocate(Money(100), &[Money(0), Money(0), Money(0)]).unwrap();
        assert_eq!(shares, vec![Money(34), Money(33), Money(33)]);
    }

    #[test]
    fn larger_remainder_wins_over_earlier_line() {
        // Exact shares: 0.6 and 1.4 → 0 and 1 by floor, the one left goes to the 0.6.
        let shares = allocate(Money(2), &[Money(3), Money(7)]).unwrap();
        assert_eq!(shares, vec![Money(1), Money(1)]);
    }

    #[test]
    fn no_lines_and_nothing_to_split_is_fine() {
        assert_eq!(allocate(Money(0), &[]).unwrap(), Vec::<Money>::new());
    }

    #[test]
    fn no_lines_with_an_amount_is_an_error() {
        assert!(allocate(Money(5), &[]).is_err());
    }

    #[test]
    fn negative_weight_is_an_error() {
        assert!(allocate(Money(5), &[Money(-1), Money(2)]).is_err());
    }
}
