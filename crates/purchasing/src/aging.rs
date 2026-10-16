//! PUR-05 Aging report: how late the money owed to each supplier is.
//! Buckets count days past the due date: current (not late), 1–30, 31–60, 61–90, 90+.

use chrono::NaiveDate;
use pos_core::{Id, Money};
use rusqlite::{Connection, named_params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::PurchasingError;
use crate::sql::{read_date, read_id, read_optional_date};

/// One supplier's balance split by how late it is. The buckets add up to `balance`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgingRow {
    pub supplier_id: Id,
    pub name: String,
    /// Not due yet (or due today). A credit balance shows here as a negative amount.
    pub current: Money,
    pub days_1_30: Money,
    pub days_31_60: Money,
    pub days_61_90: Money,
    pub days_over_90: Money,
    pub balance: Money,
}

/// The aging report on `today`, for every supplier who doesn't have a zero balance.
/// Suppliers are listed by name.
pub fn aging(connection: &Connection, today: NaiveDate) -> Result<Vec<AgingRow>, PurchasingError> {
    let mut statement = connection.prepare(
        "SELECT s.id, s.name FROM purchasing_suppliers s
         JOIN purchasing_balances b ON b.supplier_id = s.id
         WHERE b.balance_minor <> 0
         ORDER BY s.name COLLATE NOCASE, s.id",
    )?;
    let rows = statement.query_map([], |row| Ok((read_id(row, "id")?, row.get("name")?)))?;
    let mut report = Vec::new();
    for row in rows {
        let (supplier_id, name): (Id, String) = row?;
        let entries = load_entries(connection, &supplier_id)?;
        let buckets = age_entries(today, &entries)?;
        report.push(AgingRow {
            supplier_id,
            name,
            current: buckets[0],
            days_1_30: buckets[1],
            days_31_60: buckets[2],
            days_61_90: buckets[3],
            days_over_90: buckets[4],
            balance: Money::sum(&buckets)?,
        });
    }
    Ok(report)
}

/// One ledger row for aging: when it is due and how much (signed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AgingEntry {
    pub due_date: NaiveDate,
    pub amount: Money,
}

/// Splits a supplier's ledger into the five buckets: [current, 1–30, 31–60, 61–90, 90+].
/// Payments, returns and negative adjustments pay off the oldest amounts first
/// (by due date), which is how suppliers expect to be paid.
pub(crate) fn age_entries(
    today: NaiveDate,
    entries: &[AgingEntry],
) -> Result<[Money; 5], PurchasingError> {
    let mut owed: Vec<AgingEntry> = entries.iter().copied().filter(|e| e.amount.0 > 0).collect();
    owed.sort_by_key(|entry| entry.due_date);
    let mut credit = Money::zero();
    for entry in entries.iter().filter(|e| e.amount.0 < 0) {
        credit = credit.checked_sub(entry.amount)?;
    }

    let mut buckets = [Money::zero(); 5];
    for entry in owed {
        let paid_off = entry.amount.min(credit);
        credit = credit.checked_sub(paid_off)?;
        let still_owed = entry.amount.checked_sub(paid_off)?;
        let bucket = bucket_for(today, entry.due_date);
        buckets[bucket] = buckets[bucket].checked_add(still_owed)?;
    }
    // More paid than owed: the shop is in credit with this supplier.
    buckets[0] = buckets[0].checked_sub(credit)?;
    Ok(buckets)
}

/// Which bucket an amount due on `due_date` falls in on `today`.
fn bucket_for(today: NaiveDate, due_date: NaiveDate) -> usize {
    let days_late = (today - due_date).num_days();
    match days_late {
        ..=0 => 0,
        1..=30 => 1,
        31..=60 => 2,
        61..=90 => 3,
        _ => 4,
    }
}

fn load_entries(
    connection: &Connection,
    supplier: &Id,
) -> Result<Vec<AgingEntry>, PurchasingError> {
    let mut statement = connection.prepare(
        "SELECT amount_minor, due_date, business_date FROM purchasing_ledger
         WHERE supplier_id = :supplier",
    )?;
    let rows = statement.query_map(named_params! { ":supplier": supplier.as_str() }, |row| {
        // Rows without a due date are due on the day they were recorded.
        let due_date = match read_optional_date(row, "due_date")? {
            Some(date) => date,
            None => read_date(row, "business_date")?,
        };
        Ok(AgingEntry {
            due_date,
            amount: Money(row.get("amount_minor")?),
        })
    })?;
    let mut entries = Vec::new();
    for entry in rows {
        entries.push(entry?);
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(text: &str) -> Result<NaiveDate, chrono::format::ParseError> {
        NaiveDate::parse_from_str(text, "%Y-%m-%d")
    }

    fn entry(due: &str, amount: i64) -> Result<AgingEntry, chrono::format::ParseError> {
        Ok(AgingEntry {
            due_date: day(due)?,
            amount: Money(amount),
        })
    }

    #[test]
    fn pur_05_bucket_edges_are_0_30_60_90_days() -> Result<(), Box<dyn std::error::Error>> {
        let today = day("2026-03-31")?;
        assert_eq!(bucket_for(today, day("2026-04-05")?), 0);
        assert_eq!(bucket_for(today, day("2026-03-31")?), 0);
        assert_eq!(bucket_for(today, day("2026-03-30")?), 1);
        assert_eq!(bucket_for(today, day("2026-03-01")?), 1); // 30 days
        assert_eq!(bucket_for(today, day("2026-02-28")?), 2); // 31 days
        assert_eq!(bucket_for(today, day("2026-01-30")?), 2); // 60 days
        assert_eq!(bucket_for(today, day("2026-01-29")?), 3); // 61 days
        assert_eq!(bucket_for(today, day("2025-12-31")?), 3); // 90 days
        assert_eq!(bucket_for(today, day("2025-12-30")?), 4); // 91 days
        Ok(())
    }

    #[test]
    fn pur_05_credits_pay_off_the_oldest_amounts_first() -> Result<(), Box<dyn std::error::Error>> {
        let entries = [
            entry("2026-03-25", 1_000)?,
            entry("2026-01-01", 2_000)?,
            entry("2026-03-31", -2_500)?,
        ];
        let buckets = age_entries(day("2026-03-31")?, &entries)?;
        assert_eq!(
            buckets,
            [Money(0), Money(500), Money(0), Money(0), Money(0)]
        );
        Ok(())
    }

    #[test]
    fn pur_05_overpayment_shows_as_negative_current() -> Result<(), Box<dyn std::error::Error>> {
        let entries = [entry("2026-01-01", 1_000)?, entry("2026-01-02", -1_500)?];
        let buckets = age_entries(day("2026-03-31")?, &entries)?;
        assert_eq!(
            buckets,
            [Money(-500), Money(0), Money(0), Money(0), Money(0)]
        );
        Ok(())
    }
}
