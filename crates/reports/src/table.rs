//! Small helpers to build a `ReportTable`.

use pos_core::{
    AppError, Bps, BusinessDate, Cell, Column, ColumnKind, Money, ReportFilter, ReportParams,
    ReportTable, round_half_away,
};

use crate::DateRange;

/// A column with a stable key, a heading and a kind.
pub(crate) fn column(key: &str, title: &str, kind: ColumnKind) -> Column {
    Column {
        key: key.to_string(),
        title: title.to_string(),
        kind,
    }
}

/// An empty report with a title, its parameters and its columns.
pub(crate) fn new_table(title: &str, params: ReportParams, columns: Vec<Column>) -> ReportTable {
    ReportTable {
        title: title.to_string(),
        params,
        columns,
        rows: Vec::new(),
        totals: None,
        sections: Vec::new(),
    }
}

/// Parameters for a report over a date range.
pub(crate) fn range_params(range: DateRange) -> ReportParams {
    ReportParams {
        from: Some(range.from),
        to: Some(range.to),
        filters: Vec::new(),
    }
}

/// Parameters for a report about one day.
pub(crate) fn day_params(date: BusinessDate) -> ReportParams {
    ReportParams {
        from: Some(date),
        to: Some(date),
        filters: Vec::new(),
    }
}

pub(crate) fn filter(name: &str, value: &str) -> ReportFilter {
    ReportFilter {
        name: name.to_string(),
        value: value.to_string(),
    }
}

pub(crate) fn text(value: &str) -> Cell {
    Cell::Text(value.to_string())
}

/// A text cell, or an empty one when there is no text.
pub(crate) fn optional_text(value: Option<String>) -> Cell {
    value.map_or(Cell::Empty, Cell::Text)
}

/// `part` as a share of `whole`, rounded to 0.01%. Example: 25.00 of 200.00 → 12.50%.
/// Empty when it can't be shown as a percentage: a negative part, or a whole of 0 or less.
pub(crate) fn share(part: Money, whole: Money) -> Result<Cell, AppError> {
    if part.0 < 0 || whole.0 <= 0 {
        return Ok(Cell::Empty);
    }
    let bps = round_half_away(i128::from(part.0) * 10_000, i128::from(whole.0))?;
    let bps = u32::try_from(bps)
        .map_err(|_| AppError::internal(format!("share {} of {} is too large", part.0, whole.0)))?;
    Ok(Cell::Percent(Bps(bps)))
}

/// `a + b` for plain i64 sums (quantities, counts), with overflow as an error.
pub(crate) fn add(a: i64, b: i64) -> Result<i64, AppError> {
    a.checked_add(b)
        .ok_or_else(|| AppError::internal(format!("reports: {a} + {b} overflowed")))
}

/// `a − b` for plain i64 sums, with overflow as an error.
pub(crate) fn sub(a: i64, b: i64) -> Result<i64, AppError> {
    a.checked_sub(b)
        .ok_or_else(|| AppError::internal(format!("reports: {a} - {b} overflowed")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_rounds_half_away_from_zero() {
        assert_eq!(
            share(Money(2_500), Money(20_000)).unwrap(),
            Cell::Percent(Bps(1_250))
        );
        // 1/3 = 33.333…% → 33.33%
        assert_eq!(
            share(Money(1), Money(3)).unwrap(),
            Cell::Percent(Bps(3_333))
        );
        // 1/8000 = 0.0125% → 0.01%; 3/8000 = 0.0375% → 0.04%
        assert_eq!(
            share(Money(3), Money(8_000)).unwrap(),
            Cell::Percent(Bps(4))
        );
    }

    #[test]
    fn share_is_empty_when_it_cannot_be_a_percentage() {
        assert_eq!(share(Money(-5), Money(100)).unwrap(), Cell::Empty);
        assert_eq!(share(Money(5), Money(0)).unwrap(), Cell::Empty);
    }
}
