//! Finding customers at the till as the cashier types (rule CUS-02).

use pos_core::Money;
use rusqlite::{Connection, Row, named_params};

use crate::rows::{parse_id, parse_optional_id};
use crate::{CustomerHit, CustomersError};

const HIT_COLUMNS: &str = "c.id, c.phone, c.name, c.tier_id, b.points, b.credit_minor";

/// Finds customers by part of their phone number or the start of words in their name.
/// Archived customers are left out. Example: "01711", "3456" or "rah udd".
pub fn lookup(
    connection: &Connection,
    typed: &str,
    limit: u32,
) -> Result<Vec<CustomerHit>, CustomersError> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    if looks_like_phone(typed) {
        by_phone(connection, typed, limit)
    } else {
        by_name(connection, typed, limit)
    }
}

/// Only digits and the marks people type in phone numbers, with at least one digit.
fn looks_like_phone(typed: &str) -> bool {
    let allowed = |c: char| c.is_ascii_digit() || " +-().".contains(c);
    typed.chars().all(allowed) && typed.chars().any(|c| c.is_ascii_digit())
}

fn by_phone(
    connection: &Connection,
    typed: &str,
    limit: u32,
) -> Result<Vec<CustomerHit>, CustomersError> {
    let digits: String = typed.chars().filter(char::is_ascii_digit).collect();
    // Stored numbers start with the country code, typed ones often with a local 0:
    // "01711…" is stored as "+8801711…". Without leading zeros, "1711…" matches both.
    let without_leading_zeros = digits.trim_start_matches('0');
    if without_leading_zeros.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        "SELECT {HIT_COLUMNS}
         FROM customers_customers c JOIN customers_balances b ON b.customer_id = c.id
         WHERE c.archived_at IS NULL AND c.phone LIKE :pattern
         ORDER BY c.name, c.id LIMIT :limit"
    );
    // The pattern holds only digits, so it has no LIKE wildcards of its own.
    let pattern = format!("%{without_leading_zeros}%");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        named_params! { ":pattern": pattern, ":limit": limit },
        raw_hit,
    )?;
    collect_hits(rows)
}

fn by_name(
    connection: &Connection,
    typed: &str,
    limit: u32,
) -> Result<Vec<CustomerHit>, CustomersError> {
    let Some(query) = name_query(typed) else {
        return Ok(Vec::new());
    };
    let sql = format!(
        "WITH found AS (
           SELECT customer_id, rank FROM customers_search WHERE customers_search MATCH :query
         )
         SELECT {HIT_COLUMNS}
         FROM found
         JOIN customers_customers c ON c.id = found.customer_id
         JOIN customers_balances b ON b.customer_id = c.id
         WHERE c.archived_at IS NULL
         ORDER BY found.rank, c.name LIMIT :limit"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(named_params! { ":query": query, ":limit": limit }, raw_hit)?;
    collect_hits(rows)
}

/// Turns typed words into an FTS5 search where every word must start a word in the name.
/// Each word is quoted, so search marks such as `*` or `"` are treated as plain text.
/// Example: `rah udd` becomes `"rah"* "udd"*`.
fn name_query(typed: &str) -> Option<String> {
    let mut parts = Vec::new();
    for word in typed.split_whitespace() {
        if !word.chars().any(char::is_alphanumeric) {
            continue;
        }
        parts.push(format!("\"{}\"*", word.replace('"', "\"\"")));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

type RawHit = (String, String, String, Option<String>, i64, i64);

fn raw_hit(row: &Row) -> rusqlite::Result<RawHit> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn collect_hits(
    rows: impl Iterator<Item = rusqlite::Result<RawHit>>,
) -> Result<Vec<CustomerHit>, CustomersError> {
    let mut hits = Vec::new();
    for row in rows {
        let (id, phone, name, tier_id, points, credit) = row?;
        hits.push(CustomerHit {
            id: parse_id(&id)?,
            phone,
            name,
            tier_id: parse_optional_id(tier_id)?,
            points,
            credit: Money(credit),
        });
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cus_02_typed_words_become_prefix_searches() {
        assert_eq!(name_query("rah  udd").as_deref(), Some("\"rah\"* \"udd\"*"));
        assert_eq!(name_query("o\"neil").as_deref(), Some("\"o\"\"neil\"*"));
        assert_eq!(name_query(" - * "), None);
    }

    #[test]
    fn cus_02_phone_like_input_is_spotted() {
        assert!(looks_like_phone("+880 1711-123456"));
        assert!(looks_like_phone("(017) 11"));
        assert!(!looks_like_phone("rahim 017"));
        assert!(!looks_like_phone("- -"));
    }
}
