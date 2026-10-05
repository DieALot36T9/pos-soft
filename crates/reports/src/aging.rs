//! Supplier and customer aging (RPT-07).
//!
//! Each ledger row that adds to what is owed is a debt with a due date. Every row that
//! takes money off (payments, returns, negative adjustments) pays off the oldest debts
//! first. What is left is put in a bucket by how many days past due it is. Money paid
//! beyond all debts shows as a negative "current" amount. Same rule as the purchasing
//! module's own aging.

use std::collections::BTreeMap;

use pos_core::{AppError, BusinessDate, Cell, ColumnKind, Money, ReportParams, ReportTable};
use rusqlite::Connection;

use crate::ReportsError;
use crate::sql::query;
use crate::table::{column, new_table, text};

/// Current (not yet due), 1–30, 31–60, 61–90 and over 90 days past due.
pub(crate) type Buckets = [Money; 5];

/// One ledger row: when it is due, and the signed amount (+ owed, − paid).
pub(crate) type Entry = (BusinessDate, Money);

/// Puts what is still owed into the five buckets, as of `today`.
pub(crate) fn age(entries: &[Entry], today: BusinessDate) -> Result<Buckets, AppError> {
    let mut debts: Vec<Entry> = entries
        .iter()
        .copied()
        .filter(|entry| entry.1.0 > 0)
        .collect();
    debts.sort_by_key(|entry| entry.0);
    let mut credit = Money::zero();
    for entry in entries.iter().filter(|entry| entry.1.0 < 0) {
        credit = credit.checked_sub(entry.1)?;
    }
    let mut buckets = [Money::zero(); 5];
    for (due, amount) in debts {
        let paid = if credit.0 < amount.0 { credit } else { amount };
        credit = credit.checked_sub(paid)?;
        let left = amount.checked_sub(paid)?;
        let index = bucket_index((today.0 - due.0).num_days());
        buckets[index] = buckets[index].checked_add(left)?;
    }
    buckets[0] = buckets[0].checked_sub(credit)?;
    Ok(buckets)
}

/// 0 or less days late → current; then 1–30, 31–60, 61–90, over 90.
fn bucket_index(days_late: i64) -> usize {
    match days_late {
        i64::MIN..=0 => 0,
        1..=30 => 1,
        31..=60 => 2,
        61..=90 => 3,
        _ => 4,
    }
}

/// Supplier aging: what the shop owes each supplier, by days past the invoice due date.
/// A row without a due date counts from its business date.
pub fn payables_aging(conn: &Connection, today: BusinessDate) -> Result<ReportTable, ReportsError> {
    let names = query(
        conn,
        "SELECT supplier_id, name FROM purchasing_v_payables",
        &[],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let rows = query(
        conn,
        "SELECT supplier_id, COALESCE(due_date, business_date), amount_minor FROM purchasing_v_ledger",
        &[],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    aging_table("Supplier aging", "Supplier", names, rows, today)
}

/// Customer aging: what each customer owes the shop on credit, by days since the debt
/// was made (customers have no payment terms).
pub fn receivables_aging(
    conn: &Connection,
    today: BusinessDate,
) -> Result<ReportTable, ReportsError> {
    let names = query(
        conn,
        "SELECT id, name FROM customers_v_customers",
        &[],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let rows = query(
        conn,
        "SELECT customer_id, business_date, amount_minor FROM customers_v_credit_ledger",
        &[],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    aging_table("Customer aging", "Customer", names, rows, today)
}

/// One row per supplier or customer with something owed either way, sorted by name.
fn aging_table(
    title: &str,
    who: &str,
    names: Vec<(String, String)>,
    rows: Vec<(String, String, i64)>,
    today: BusinessDate,
) -> Result<ReportTable, ReportsError> {
    let names: BTreeMap<String, String> = names.into_iter().collect();
    let mut entries: BTreeMap<String, Vec<Entry>> = BTreeMap::new();
    for (owner, due, amount) in rows {
        let due = BusinessDate::parse(&due)?;
        entries.entry(owner).or_default().push((due, Money(amount)));
    }
    let mut lines: Vec<(String, Buckets)> = Vec::new();
    for (owner, owner_entries) in entries {
        let buckets = age(&owner_entries, today)?;
        if buckets.iter().any(|amount| amount.0 != 0) {
            lines.push((names.get(&owner).cloned().unwrap_or(owner), buckets));
        }
    }
    lines.sort_by(|a, b| a.0.cmp(&b.0));

    let params = ReportParams {
        to: Some(today),
        ..ReportParams::default()
    };
    let mut table = new_table(
        title,
        params,
        vec![
            column("name", who, ColumnKind::Text),
            column("current", "Current", ColumnKind::Money),
            column("days_1_30", "1–30 days", ColumnKind::Money),
            column("days_31_60", "31–60 days", ColumnKind::Money),
            column("days_61_90", "61–90 days", ColumnKind::Money),
            column("days_over_90", "Over 90 days", ColumnKind::Money),
            column("total", "Total", ColumnKind::Money),
        ],
    );
    let mut sums = [Money::zero(); 6];
    for (name, buckets) in lines {
        let total = Money::sum(&buckets)?;
        let mut row = vec![text(&name)];
        for (index, amount) in buckets.iter().chain([&total]).enumerate() {
            sums[index] = sums[index].checked_add(*amount)?;
            row.push(Cell::Money(*amount));
        }
        table.rows.push(row);
    }
    let mut totals = vec![text("Total")];
    totals.extend(sums.map(Cell::Money));
    table.totals = Some(totals);
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> BusinessDate {
        BusinessDate::parse(text).unwrap()
    }

    #[test]
    fn rpt_07_bucket_edges_are_0_30_60_90_days() {
        assert_eq!(bucket_index(-5), 0);
        assert_eq!(bucket_index(0), 0);
        assert_eq!(bucket_index(1), 1);
        assert_eq!(bucket_index(30), 1);
        assert_eq!(bucket_index(31), 2);
        assert_eq!(bucket_index(60), 2);
        assert_eq!(bucket_index(61), 3);
        assert_eq!(bucket_index(90), 3);
        assert_eq!(bucket_index(91), 4);
    }

    #[test]
    fn rpt_07_payments_pay_off_the_oldest_debts_first() {
        let today = date("2026-03-05");
        let entries = [
            (date("2026-03-01"), Money(300)),
            (date("2026-01-10"), Money(500)),
            (date("2026-03-02"), Money(-600)),
        ];
        // 500 (54 days) is paid, then 100 of the 300 (4 days).
        assert_eq!(
            age(&entries, today).unwrap(),
            [Money(0), Money(200), Money(0), Money(0), Money(0)]
        );
    }

    #[test]
    fn rpt_07_overpayment_shows_as_negative_current() {
        let today = date("2026-03-05");
        let entries = [
            (date("2025-11-01"), Money(1_000)),
            (date("2025-11-05"), Money(-1_500)),
        ];
        assert_eq!(
            age(&entries, today).unwrap(),
            [Money(-500), Money(0), Money(0), Money(0), Money(0)]
        );
    }
}
