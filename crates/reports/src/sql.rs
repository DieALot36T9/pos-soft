//! Running a read-only query with named parameters.

use rusqlite::types::Value;
use rusqlite::{Connection, Row};

use crate::ReportsError;

/// One named parameter. Example: `(":from", text("2026-03-01"))`.
pub(crate) type Param = (&'static str, Value);

/// A text parameter.
pub(crate) fn text(value: &str) -> Value {
    Value::Text(value.to_string())
}

/// A text parameter that may be missing. Missing becomes SQL NULL ("no filter").
pub(crate) fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, text)
}

/// Runs `sql` and reads every row with `read`.
/// Only the parameters the SQL actually names are bound, so report code can pass one
/// shared list (date, till, shift, …) to several queries that each use part of it.
pub(crate) fn query<T>(
    conn: &Connection,
    sql: &str,
    params: &[Param],
    mut read: impl FnMut(&Row) -> rusqlite::Result<T>,
) -> Result<Vec<T>, ReportsError> {
    let mut statement = conn.prepare(sql)?;
    for (name, value) in params {
        if let Some(index) = statement.parameter_index(name)? {
            statement.raw_bind_parameter(index, value)?;
        }
    }
    let mut rows = statement.raw_query();
    let mut found = Vec::new();
    while let Some(row) = rows.next()? {
        found.push(read(row)?);
    }
    Ok(found)
}

/// Runs a query that gives exactly one row, such as `SELECT COUNT(*) …`.
pub(crate) fn query_one<T>(
    conn: &Connection,
    sql: &str,
    params: &[Param],
    read: impl FnMut(&Row) -> rusqlite::Result<T>,
) -> Result<T, ReportsError> {
    let mut found = query(conn, sql, params, read)?;
    if found.is_empty() {
        return Err(ReportsError::Database(rusqlite::Error::QueryReturnedNoRows));
    }
    Ok(found.swap_remove(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_named_parameters_are_bound() {
        let conn = Connection::open_in_memory().unwrap();
        let params = [(":a", Value::Integer(2)), (":unused", Value::Integer(9))];
        let doubled = query_one(&conn, "SELECT :a * 2", &params, |row| row.get::<_, i64>(0));
        assert_eq!(doubled.unwrap(), 4);
    }
}
