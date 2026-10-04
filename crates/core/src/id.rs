//! Row ids: ULIDs stored as 26-character text (ADR-009).

use std::fmt;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use ulid::Ulid;

use crate::AppError;

/// A unique id for any row. It sorts by creation time and never clashes across tills.
/// Example: "01HX5Z3J8Q9W2E4R6T8Y0V1K2M".
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(try_from = "String", into = "String")]
#[ts(export, type = "string")]
pub struct Id(String);

impl Id {
    /// Makes a new id.
    pub fn new() -> Id {
        Id(Ulid::generate().to_string())
    }

    /// Reads an id from text, for example from the database or the UI.
    /// Lower-case text is accepted and stored in capitals.
    pub fn parse(text: &str) -> Result<Id, AppError> {
        Ulid::from_string(text)
            .map(|ulid| Id(ulid.to_string()))
            .map_err(|_| AppError::validation("id_invalid"))
    }

    /// The id as text, ready to store in a `TEXT` column.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Id {
    /// A new id, the same as `Id::new()`.
    fn default() -> Id {
        Id::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<String> for Id {
    type Error = AppError;

    fn try_from(text: String) -> Result<Id, AppError> {
        Id::parse(&text)
    }
}

impl From<Id> for String {
    fn from(id: Id) -> String {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ids_are_26_characters_and_different() {
        let first = Id::new();
        let second = Id::new();
        assert_eq!(first.as_str().len(), 26);
        assert_ne!(first, second);
    }

    #[test]
    fn parse_reads_back_what_new_made() {
        let id = Id::new();
        assert_eq!(Id::parse(id.as_str()).unwrap(), id);
        assert_eq!(Id::parse(&id.as_str().to_lowercase()).unwrap(), id);
    }

    #[test]
    fn parse_refuses_text_that_is_not_an_id() {
        assert_eq!(
            Id::parse("not-an-id"),
            Err(AppError::validation("id_invalid"))
        );
    }

    #[test]
    fn ids_from_the_ui_are_checked() {
        let bad: Result<Id, _> = serde_json::from_str(r#""hello""#);
        assert!(bad.is_err());
    }
}
