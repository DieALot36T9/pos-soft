//! Tax per rate and per component: the tax report (RPT-06, TAX-09) and the
//! "Tax by rate" part of the daily summary and shift reports. The sums come from
//! `tax_sums`.

use pos_core::{AppError, Bps, Cell, ColumnKind, Money, ReportTable};
use rusqlite::Connection;

use crate::sql::text as text_param;
use crate::table::{column, new_table, range_params, text};
use crate::tax_sums::{RateTotals, rate_and_part_totals};
use crate::{DateRange, ReportsError};

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
    let (rates, parts) = rate_and_part_totals(conn, scope, &params)?;

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
