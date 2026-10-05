//! Small helpers for reading columns and checking typed names.

use pos_core::{Id, Money, Timestamp};
use rusqlite::Row;
use rusqlite::types::Type;

use crate::error::{CatalogError, invalid};

/// The longest name allowed for products, units, categories and brands (rule CAT-01).
const MAX_NAME_CHARS: usize = 120;

/// Reads an id column. A badly stored id is a bug, so it becomes a database error.
pub(crate) fn read_id(row: &Row, index: usize) -> rusqlite::Result<Id> {
    let text: String = row.get(index)?;
    Id::parse(&text).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}

/// Reads an id column that may be empty.
pub(crate) fn read_optional_id(row: &Row, index: usize) -> rusqlite::Result<Option<Id>> {
    let text: Option<String> = row.get(index)?;
    match text {
        None => Ok(None),
        Some(_) => read_id(row, index).map(Some),
    }
}

/// Reads a money column (`*_minor`).
pub(crate) fn read_money(row: &Row, index: usize) -> rusqlite::Result<Money> {
    row.get(index).map(Money)
}

/// Reads a timestamp column that may be empty, such as `archived_at`.
pub(crate) fn read_optional_time(row: &Row, index: usize) -> rusqlite::Result<Option<Timestamp>> {
    let value: Option<i64> = row.get(index)?;
    Ok(value.map(Timestamp))
}

/// Trims a typed name and checks it is there and not too long.
pub(crate) fn clean_name(name: &str) -> Result<String, CatalogError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(invalid("name", "name_missing"));
    }
    if trimmed.chars().count() > MAX_NAME_CHARS {
        return Err(invalid("name", "name_too_long"));
    }
    Ok(trimmed.to_string())
}

/// Trims optional text. Blank text counts as "not given".
pub(crate) fn clean_optional(text: Option<&str>) -> Option<String> {
    let trimmed = text?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Turns an optional id into text for a nullable column.
pub(crate) fn optional_id_text(id: Option<&Id>) -> Option<&str> {
    id.map(Id::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_checked() {
        assert_eq!(clean_name("  Milk 1L ").unwrap(), "Milk 1L");
        assert_eq!(clean_name(" "), Err(invalid("name", "name_missing")));
        // 120 letters is fine, even when each letter takes two bytes.
        assert!(clean_name(&"é".repeat(120)).is_ok());
        assert_eq!(
            clean_name(&"a".repeat(121)),
            Err(invalid("name", "name_too_long"))
        );
    }

    #[test]
    fn blank_optional_text_is_none() {
        assert_eq!(clean_optional(Some("  ")), None);
        assert_eq!(clean_optional(Some(" 00123 ")), Some("00123".to_string()));
        assert_eq!(clean_optional(None), None);
    }
}
