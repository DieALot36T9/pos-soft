//! Matching file columns to the fields a workflow needs (IO-01 step 3), and
//! collecting cell errors with their row and column (IO-01 step 5).

use crate::Table;

/// One field an import needs, with the header names people commonly use for it.
/// Example: key "sale_price", names ["Price", "Sale price", "MRP"].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSpec {
    /// A stable key the workflow uses. It also counts as a header name.
    pub key: String,
    pub names: Vec<String>,
}

impl FieldSpec {
    /// A field with its key and the header names that mean it.
    pub fn new(key: &str, names: &[&str]) -> FieldSpec {
        FieldSpec {
            key: key.to_string(),
            names: names.iter().map(|name| name.to_string()).collect(),
        }
    }
}

/// For each field, finds the column whose header means that field, or `None`.
/// Case, spaces and punctuation don't matter ("Sale-price" matches "sale price").
/// Each column is used once; fields earlier in the list choose first.
pub fn auto_map(headers: &[String], fields: &[FieldSpec]) -> Vec<Option<usize>> {
    let simple_headers: Vec<String> = headers.iter().map(|header| simplify(header)).collect();
    let mut used = vec![false; headers.len()];
    let mut mapping = Vec::with_capacity(fields.len());
    for field in fields {
        let mut wanted: Vec<String> = field.names.iter().map(|name| simplify(name)).collect();
        wanted.push(simplify(&field.key));
        let found = (0..headers.len())
            .find(|&index| !used[index] && wanted.contains(&simple_headers[index]));
        if let Some(index) = found {
            used[index] = true;
        }
        mapping.push(found);
    }
    mapping
}

/// Lower-case letters and digits only. Example: " Sale-Price " → "saleprice".
fn simplify(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// One bad cell in a file, shown in the import preview.
/// Example: row 812, column "Sale price", code "money_not_a_number", value "12,5O".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellErr {
    /// The row number as the user sees it in the file (headers are row 1).
    pub row: usize,
    /// The column's header.
    pub column: String,
    /// The code the parse function returned. The UI turns it into words.
    pub code: String,
    /// What the cell held.
    pub value: String,
}

impl CellErr {
    /// Runs `check` on every cell of one column and lists the cells it refused.
    pub fn check_column(
        table: &Table,
        column: usize,
        check: impl Fn(&str) -> Result<(), String>,
    ) -> Vec<CellErr> {
        let header = table.headers.get(column).cloned().unwrap_or_default();
        let mut errors = Vec::new();
        for (index, row) in table.rows.iter().enumerate() {
            let value = row.get(column).map(String::as_str).unwrap_or("");
            if let Err(code) = check(value) {
                errors.push(CellErr {
                    row: index + 2,
                    column: header.clone(),
                    code,
                    value: value.to_string(),
                });
            }
        }
        errors
    }
}
