//! Gapless document numbers for each till (PAY-08, RET-06).
//! A number is taken inside the transaction that saves the document. If that
//! transaction is rolled back, the number goes back too, so there are never gaps.

use pos_core::Id;
use rusqlite::{Transaction, named_params};

use crate::SalesError;

/// Which kind of document a number is for. Each has its own sequence per till.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocKind {
    Sale,
    Return,
}

impl DocKind {
    fn as_str(self) -> &'static str {
        match self {
            DocKind::Sale => "sale",
            DocKind::Return => "return",
        }
    }
}

/// Takes the next number for this till and document kind. The first one is 1.
pub(crate) fn take_next_number(
    tx: &Transaction,
    till_id: &Id,
    doc: DocKind,
) -> Result<i64, SalesError> {
    let params = named_params! { ":till": till_id.as_str(), ":doc": doc.as_str() };
    tx.execute(
        "INSERT OR IGNORE INTO sales_sequences (till_id, doc, next) VALUES (:till, :doc, 1)",
        params,
    )?;
    let number: i64 = tx.query_row(
        "SELECT next FROM sales_sequences WHERE till_id = :till AND doc = :doc",
        params,
        |row| row.get(0),
    )?;
    tx.execute(
        "UPDATE sales_sequences SET next = next + 1 WHERE till_id = :till AND doc = :doc",
        params,
    )?;
    Ok(number)
}

/// The printed number. Sales: "T1-000123". Returns: "T1-R000045". At least six digits.
pub(crate) fn format_number(till_code: &str, doc: DocKind, number: i64) -> String {
    match doc {
        DocKind::Sale => format!("{till_code}-{number:06}"),
        DocKind::Return => format!("{till_code}-R{number:06}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pay_08_invoice_numbers_have_the_till_code_and_six_digits() {
        assert_eq!(format_number("T1", DocKind::Sale, 123), "T1-000123");
        assert_eq!(format_number("T1", DocKind::Sale, 1_234_567), "T1-1234567");
    }

    #[test]
    fn ret_06_return_numbers_have_an_r() {
        assert_eq!(format_number("T1", DocKind::Return, 45), "T1-R000045");
    }
}
