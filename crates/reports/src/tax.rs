//! Tax per rate and per component: the tax report (RPT-06, TAX-09) and the
//! "Tax by rate" part of the daily summary and shift reports.

use std::collections::BTreeMap;

use pos_core::{AppError, Bps, Cell, ColumnKind, Money, ReportTable};
use rusqlite::Connection;

use crate::sql::{Param, query, text as text_param};
use crate::table::{column, new_table, range_params, sub, text};
use crate::{DateRange, ReportsError};

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

/// Adds up sale lines and return lines per tax rate, sorted by rate name.
/// `scope` is a condition on alias `s`, which both `sales_v_lines` and
/// `sales_v_return_lines` satisfy (business_date, till_id, shift_id).
pub(crate) fn rate_totals(
    conn: &Connection,
    scope: &str,
    params: &[Param],
) -> Result<Vec<RateTotals>, ReportsError> {
    let mut rates: BTreeMap<String, RateTotals> = BTreeMap::new();
    let sales_sql = format!(
        "SELECT s.tax_rate_id, MAX(s.tax_name), MAX(s.tax_bps), SUM(s.net_minor), SUM(s.tax_minor)
         FROM sales_v_lines s WHERE s.status = 'completed' AND {scope} GROUP BY s.tax_rate_id"
    );
    for (rate_id, name, bps, net, tax) in query(conn, &sales_sql, params, |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
        ))
    })? {
        rates.insert(
            rate_id,
            RateTotals {
                rate_id: String::new(),
                name,
                bps,
                sales_net: Money(net),
                sales_tax: Money(tax),
                returns_net: Money::zero(),
                returns_tax: Money::zero(),
            },
        );
    }
    // A return's rate name comes from the latest sale line of that rate.
    let returns_sql = format!(
        "SELECT s.tax_rate_id, MAX(s.tax_bps), SUM(s.net_minor), SUM(s.tax_minor),
           (SELECT l.tax_name FROM sales_v_lines l WHERE l.tax_rate_id = s.tax_rate_id
            ORDER BY l.completed_at DESC LIMIT 1)
         FROM sales_v_return_lines s WHERE {scope} GROUP BY s.tax_rate_id"
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
        let rate = rates.entry(rate_id).or_insert_with(|| RateTotals {
            rate_id: String::new(),
            name: name.unwrap_or_else(|| "Tax rate".to_string()),
            bps,
            sales_net: Money::zero(),
            sales_tax: Money::zero(),
            returns_net: Money::zero(),
            returns_tax: Money::zero(),
        });
        rate.returns_net = Money(net);
        rate.returns_tax = Money(tax);
    }
    let mut list: Vec<RateTotals> = Vec::new();
    for (rate_id, mut rate) in rates {
        rate.rate_id = rate_id;
        list.push(rate);
    }
    list.sort_by(|a, b| a.name.cmp(&b.name).then(a.rate_id.cmp(&b.rate_id)));
    Ok(list)
}

/// The "Tax by rate" section: net and tax after returns, per rate.
pub(crate) fn tax_by_rate_section(rates: &[RateTotals]) -> Result<ReportTable, AppError> {
    let mut table = new_table(
        "Tax by rate",
        Default::default(),
        vec![
            column("rate", "Rate", ColumnKind::Text),
            column("rate_percent", "Rate %", ColumnKind::Percent),
            column("net", "Net", ColumnKind::Money),
            column("tax", "Tax", ColumnKind::Money),
        ],
    );
    let mut total_net = Money::zero();
    let mut total_tax = Money::zero();
    for rate in rates {
        let net = rate.sales_net.checked_sub(rate.returns_net)?;
        let tax = rate.sales_tax.checked_sub(rate.returns_tax)?;
        total_net = total_net.checked_add(net)?;
        total_tax = total_tax.checked_add(tax)?;
        table.rows.push(vec![
            text(&rate.name),
            Cell::Percent(Bps(rate.bps)),
            Cell::Money(net),
            Cell::Money(tax),
        ]);
    }
    table.totals = Some(vec![
        text("Total"),
        Cell::Empty,
        Cell::Money(total_net),
        Cell::Money(total_tax),
    ]);
    Ok(table)
}

/// Tax report (RPT-06, TAX-09): per rate and per component, net sales, tax collected,
/// returns, tax refunded and net tax, for a date range.
pub fn tax_report(conn: &Connection, range: DateRange) -> Result<ReportTable, ReportsError> {
    let range = range.check()?;
    let params = [
        (":from", text_param(&range.from.to_string())),
        (":to", text_param(&range.to.to_string())),
    ];
    let scope = "s.business_date BETWEEN :from AND :to";
    let rates = rate_totals(conn, scope, &params)?;
    let parts = component_totals(conn, scope, &params)?;

    let mut table = new_table("Tax report", range_params(range), tax_columns());
    let mut sums = [Money::zero(); 5];
    for rate in &rates {
        let net_tax = rate.sales_tax.checked_sub(rate.returns_tax)?;
        let amounts = [
            rate.sales_net,
            rate.sales_tax,
            rate.returns_net,
            rate.returns_tax,
            net_tax,
        ];
        for (sum, amount) in sums.iter_mut().zip(amounts) {
            *sum = sum.checked_add(amount)?;
        }
        let mut row = vec![text(&rate.name), Cell::Empty, Cell::Percent(Bps(rate.bps))];
        row.extend(amounts.map(Cell::Money));
        table.rows.push(row);
        for ((rate_id, part_name, part_bps), (collected, refunded)) in &parts {
            if *rate_id == rate.rate_id {
                table.rows.push(vec![
                    text(&rate.name),
                    text(part_name),
                    Cell::Percent(Bps(*part_bps)),
                    Cell::Empty,
                    Cell::Money(*collected),
                    Cell::Empty,
                    Cell::Money(*refunded),
                    Cell::Money(collected.checked_sub(*refunded)?),
                ]);
            }
        }
    }
    let mut totals = vec![text("Total"), Cell::Empty, Cell::Empty];
    totals.extend(sums.map(Cell::Money));
    table.totals = Some(totals);
    Ok(table)
}

fn tax_columns() -> Vec<pos_core::Column> {
    vec![
        column("rate", "Rate", ColumnKind::Text),
        column("part", "Part", ColumnKind::Text),
        column("rate_percent", "Rate %", ColumnKind::Percent),
        column("net_sales", "Net sales", ColumnKind::Money),
        column("tax_collected", "Tax collected", ColumnKind::Money),
        column("returns", "Returns", ColumnKind::Money),
        column("tax_refunded", "Tax refunded", ColumnKind::Money),
        column("net_tax", "Net tax", ColumnKind::Money),
    ]
}

/// (rate id, part name, part bps) → (tax collected, tax refunded). Sorted by part name.
type PartTotals = BTreeMap<(String, String, u32), (Money, Money)>;

/// Tax per component (TAX-03). Sales keep their parts on each line. A return line
/// splits its refunded tax with the parts of the sale line it came from (or, for a
/// return without a receipt, the latest sale line of the same rate).
fn component_totals(
    conn: &Connection,
    scope: &str,
    params: &[Param],
) -> Result<PartTotals, ReportsError> {
    let mut parts: PartTotals = BTreeMap::new();
    let sales_sql = format!(
        "SELECT s.tax_rate_id, json_extract(j.value, '$.name'), json_extract(j.value, '$.bps'),
                SUM(json_extract(j.value, '$.tax_minor'))
         FROM sales_v_lines s, json_each(s.tax_components_json) j
         WHERE s.status = 'completed' AND {scope}
         GROUP BY 1, 2, 3"
    );
    for (rate_id, name, bps, tax) in query(conn, &sales_sql, params, |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })? {
        parts.insert((rate_id, name, bps), (Money(tax), Money::zero()));
    }
    let returns_sql = format!(
        "SELECT s.id, s.tax_rate_id, s.tax_minor, json_extract(j.value, '$.name'),
                json_extract(j.value, '$.bps')
         FROM sales_v_return_lines s, json_each(COALESCE(
           (SELECT l.tax_components_json FROM sales_v_lines l WHERE l.id = s.sale_line_id),
           (SELECT l.tax_components_json FROM sales_v_lines l WHERE l.tax_rate_id = s.tax_rate_id
            ORDER BY l.completed_at DESC LIMIT 1),
           '[]')) j
         WHERE {scope}
         ORDER BY s.id, j.key"
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
    Ok(parts)
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
