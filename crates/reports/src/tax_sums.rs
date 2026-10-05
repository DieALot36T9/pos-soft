//! Adding up tax per rate and per component (TAX-03, TAX-09), for `tax`.
//!
//! Speed (RPT-11): a year has about 1.6 million sale lines. They are read once and added
//! up here, not with SQL `GROUP BY` (which sorts every line first). Lines with the same
//! list of tax parts are counted together, so each different list is split only once.

use std::collections::{BTreeMap, HashMap};

use pos_core::{AppError, Money};
use rusqlite::Connection;

use crate::ReportsError;
use crate::sql::{Param, query, text as text_param};
use crate::table::sub;

/// Sales and returns of one tax rate.
pub(crate) struct RateTotals {
    pub(crate) rate_id: String,
    pub(crate) name: String,
    pub(crate) bps: u32,
    pub(crate) sales_net: Money,
    pub(crate) sales_tax: Money,
    pub(crate) returns_net: Money,
    pub(crate) returns_tax: Money,
}

impl RateTotals {
    fn new(rate_id: &str, name: String, bps: u32) -> RateTotals {
        RateTotals {
            rate_id: rate_id.to_string(),
            name,
            bps,
            sales_net: Money::zero(),
            sales_tax: Money::zero(),
            returns_net: Money::zero(),
            returns_tax: Money::zero(),
        }
    }
}

/// (rate id, part name, part bps) → (tax collected, tax refunded). Sorted by part name.
pub(crate) type PartTotals = BTreeMap<(String, String, u32), (Money, Money)>;

/// (rate id, list of parts as saved on the line) → number of lines.
type PartLists = HashMap<(String, String), i64>;

/// Sale lines and return lines added up per tax rate, sorted by rate name.
/// `scope` is a condition on alias `s`, which both `sales_v_lines` and
/// `sales_v_return_lines` satisfy (business_date, till_id, shift_id).
pub(crate) fn rate_totals(
    conn: &Connection,
    scope: &str,
    params: &[Param],
) -> Result<Vec<RateTotals>, ReportsError> {
    let (rates, _) = sale_sums(conn, scope, params, false)?;
    finish_rates(conn, scope, params, rates)
}

/// Rate totals and component totals, from one pass over the sale lines.
pub(crate) fn rate_and_part_totals(
    conn: &Connection,
    scope: &str,
    params: &[Param],
) -> Result<(Vec<RateTotals>, PartTotals), ReportsError> {
    let (rates, lists) = sale_sums(conn, scope, params, true)?;
    let mut parts = sale_parts(conn, &lists)?;
    add_return_parts(conn, scope, params, &mut parts)?;
    Ok((finish_rates(conn, scope, params, rates)?, parts))
}

/// One pass over the sale lines: net and tax per rate and, when asked, how many lines
/// have each list of tax parts.
fn sale_sums(
    conn: &Connection,
    scope: &str,
    params: &[Param],
    with_parts: bool,
) -> Result<(HashMap<String, RateTotals>, PartLists), ReportsError> {
    let parts_column = if with_parts {
        "s.tax_components_json"
    } else {
        "'[]'"
    };
    let sql = format!(
        "SELECT s.tax_rate_id, s.tax_name, s.tax_bps, s.net_minor, s.tax_minor, {parts_column}
         FROM sales_v_lines s WHERE s.status = 'completed' AND {scope}"
    );
    let mut rates: HashMap<String, RateTotals> = HashMap::new();
    let mut lists: PartLists = HashMap::new();
    query(conn, &sql, params, |row| {
        let rate_id = row.get_ref(0)?.as_str()?;
        let rate = match rates.get_mut(rate_id) {
            Some(rate) => rate,
            None => rates.entry(rate_id.to_string()).or_insert(RateTotals::new(
                rate_id,
                row.get(1)?,
                row.get(2)?,
            )),
        };
        rate.sales_net = Money(checked(rate.sales_net.0, row.get(3)?)?);
        rate.sales_tax = Money(checked(rate.sales_tax.0, row.get(4)?)?);
        if with_parts {
            let key = (rate_id.to_string(), row.get::<_, String>(5)?);
            *lists.entry(key).or_insert(0) += 1;
        }
        Ok(())
    })?;
    Ok((rates, lists))
}

/// Adds the returns per rate and sorts the rates by name.
fn finish_rates(
    conn: &Connection,
    scope: &str,
    params: &[Param],
    mut rates: HashMap<String, RateTotals>,
) -> Result<Vec<RateTotals>, ReportsError> {
    // A return's rate name comes from the sale line it returns (or, without a receipt,
    // the latest sale of the same product at that rate). Both lookups use an index.
    let returns_sql = format!(
        "SELECT s.tax_rate_id, MAX(s.tax_bps), SUM(s.net_minor), SUM(s.tax_minor),
           MAX({SALE_LINE_OF_RETURN})
         FROM sales_v_return_lines s WHERE {scope} GROUP BY s.tax_rate_id",
        SALE_LINE_OF_RETURN = sale_line_value("tax_name")
    );
    for (rate_id, bps, net, tax, name) in query(conn, &returns_sql, params, |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })? {
        let name = name.unwrap_or_else(|| "Tax rate".to_string());
        let rate = rates
            .entry(rate_id.clone())
            .or_insert_with(|| RateTotals::new(&rate_id, name, bps));
        rate.returns_net = Money(net);
        rate.returns_tax = Money(tax);
    }
    let mut list: Vec<RateTotals> = rates.into_values().collect();
    list.sort_by(|a, b| a.name.cmp(&b.name).then(a.rate_id.cmp(&b.rate_id)));
    Ok(list)
}

/// Splits each different list of parts once, and multiplies by its number of lines.
fn sale_parts(conn: &Connection, lists: &PartLists) -> Result<PartTotals, ReportsError> {
    let mut parts: PartTotals = BTreeMap::new();
    for ((rate_id, list), lines) in lists {
        let found: Vec<(String, u32, i64)> = query(
            conn,
            "SELECT json_extract(value, '$.name'), json_extract(value, '$.bps'),
                    json_extract(value, '$.tax_minor')
             FROM json_each(:list)",
            &[(":list", text_param(list))],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        for (name, bps, tax) in found {
            let amount = tax.checked_mul(*lines).ok_or_else(|| {
                AppError::internal(format!("tax part {tax} × {lines} lines overflowed"))
            })?;
            let entry = parts
                .entry((rate_id.clone(), name, bps))
                .or_insert((Money::zero(), Money::zero()));
            entry.0 = entry.0.checked_add(Money(amount))?;
        }
    }
    Ok(parts)
}

/// Refunded tax per part. A return line splits its refunded tax with the parts of the
/// sale line it came from (or, without a receipt, the latest sale of the same product
/// at the same rate).
fn add_return_parts(
    conn: &Connection,
    scope: &str,
    params: &[Param],
    parts: &mut PartTotals,
) -> Result<(), ReportsError> {
    let returns_sql = format!(
        "SELECT s.id, s.tax_rate_id, s.tax_minor, json_extract(j.value, '$.name'),
                json_extract(j.value, '$.bps')
         FROM sales_v_return_lines s, json_each(COALESCE(
           {SALE_LINE_OF_RETURN},
           '[]')) j
         WHERE {scope}
         ORDER BY s.id, j.key",
        SALE_LINE_OF_RETURN = sale_line_value("tax_components_json")
    );
    let rows: Vec<(String, String, i64, String, u32)> = query(conn, &returns_sql, params, |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
        ))
    })?;
    // Rows come grouped by return line; split each line's tax over its parts.
    let mut start = 0;
    while start < rows.len() {
        let mut end = start;
        while end < rows.len() && rows[end].0 == rows[start].0 {
            end += 1;
        }
        let line = &rows[start..end];
        let part_bps: Vec<u32> = line.iter().map(|part| part.4).collect();
        let shares = split_tax(Money(line[0].2), &part_bps)?;
        for (part, share) in line.iter().zip(shares) {
            let key = (part.1.clone(), part.3.clone(), part.4);
            let entry = parts.entry(key).or_insert((Money::zero(), Money::zero()));
            entry.1 = entry.1.checked_add(share)?;
        }
        start = end;
    }
    Ok(())
}

/// SQL for one column of the sale line a return line (alias `s`) came from: by its
/// `sale_line_id`, or for a return without a receipt, the latest sale of the same
/// product at the same rate. Example column: "tax_name".
fn sale_line_value(column: &str) -> String {
    format!(
        "COALESCE(
           (SELECT l.{column} FROM sales_v_lines l WHERE l.id = s.sale_line_id),
           (SELECT l.{column} FROM sales_v_lines l
            WHERE l.product_id = s.product_id AND l.tax_rate_id = s.tax_rate_id
            ORDER BY l.completed_at DESC LIMIT 1))"
    )
}

/// Splits a tax amount over parts, the same way the tax module does (TAX-03):
/// each part gets tax × part rate / whole rate, rounded towards zero; the last part
/// gets what is left. Example: 4.57 over 7.5% + 7.5% → 2.28 + 2.29.
pub(crate) fn split_tax(tax: Money, part_bps: &[u32]) -> Result<Vec<Money>, AppError> {
    let whole: i128 = part_bps.iter().map(|bps| i128::from(*bps)).sum();
    let mut shares = Vec::with_capacity(part_bps.len());
    let mut given = 0_i64;
    for (index, bps) in part_bps.iter().enumerate() {
        let share = if index + 1 == part_bps.len() {
            sub(tax.0, given)?
        } else if whole == 0 {
            0
        } else {
            let exact = i128::from(tax.0) * i128::from(*bps) / whole;
            i64::try_from(exact).map_err(|_| AppError::internal("split_tax overflowed"))?
        };
        given = given
            .checked_add(share)
            .ok_or_else(|| AppError::internal("split_tax overflowed"))?;
        shares.push(Money(share));
    }
    Ok(shares)
}

/// `a + b`, as an error on overflow (inside the row reader, which returns database errors).
fn checked(a: i64, b: i64) -> rusqlite::Result<i64> {
    a.checked_add(b)
        .ok_or(rusqlite::Error::IntegralValueOutOfRange(0, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tax_03_split_rounds_towards_zero_and_the_last_part_gets_the_rest() {
        assert_eq!(
            split_tax(Money(457), &[750, 750]).unwrap(),
            vec![Money(228), Money(229)]
        );
        assert_eq!(
            split_tax(Money(1_304), &[750, 750]).unwrap(),
            vec![Money(652), Money(652)]
        );
        assert_eq!(
            split_tax(Money(-457), &[750, 750]).unwrap(),
            vec![Money(-228), Money(-229)]
        );
        assert_eq!(split_tax(Money(100), &[]).unwrap(), Vec::<Money>::new());
    }
}
