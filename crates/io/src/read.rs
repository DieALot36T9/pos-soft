//! Reading .csv and .xlsx files into a `Table` of text cells (IO-01 step 2).
//! The `csv` crate reads CSV (RFC 4180) and `calamine` reads Excel files.

use std::path::Path;

use calamine::{Data, Reader, Xlsx, open_workbook};

use crate::Table;
use crate::error::{IoError, Result, unreadable};
use crate::guard::unguard;

/// The two kinds of file this crate handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileKind {
    Csv,
    Xlsx,
}

/// Works out the kind of file from its name. Example: "Products.XLSX" → Xlsx.
pub(crate) fn file_kind(path: &Path) -> Result<FileKind> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_lowercase();
    match extension.as_str() {
        "csv" => Ok(FileKind::Csv),
        "xlsx" => Ok(FileKind::Xlsx),
        _ => Err(IoError::UnknownFileType { extension }),
    }
}

/// Reads a .csv or .xlsx file. The first row holds the headers.
/// For .xlsx, `sheet` picks a sheet by name; `None` means the first sheet.
/// CSV files may start with a BOM and may use commas, semicolons or tabs.
pub fn read_file(path: &Path, sheet: Option<&str>) -> Result<Table> {
    let all_rows = match file_kind(path)? {
        FileKind::Csv => read_csv_rows(path)?,
        FileKind::Xlsx => read_xlsx_rows(path, sheet)?,
    };
    into_table(all_rows)
}

/// Lists the sheet names of an .xlsx file in order. A .csv file has none.
pub fn sheet_names(path: &Path) -> Result<Vec<String>> {
    match file_kind(path)? {
        FileKind::Csv => Ok(Vec::new()),
        FileKind::Xlsx => {
            let workbook: Xlsx<_> = open_workbook(path).map_err(unreadable)?;
            Ok(workbook.sheet_names())
        }
    }
}

/// Splits the header row from the data rows. Data rows are filled up to the
/// number of headers. Blank rows at the end are dropped, but blank rows in the
/// middle stay so row numbers still match the file.
fn into_table(mut all_rows: Vec<Vec<String>>) -> Result<Table> {
    while all_rows
        .last()
        .is_some_and(|row| row.iter().all(|cell| cell.is_empty()))
    {
        all_rows.pop();
    }
    if all_rows.is_empty() {
        return Err(IoError::FileEmpty);
    }
    let headers: Vec<String> = all_rows
        .remove(0)
        .iter()
        .map(|header| header.trim().to_string())
        .collect();
    for row in &mut all_rows {
        if row.len() < headers.len() {
            row.resize(headers.len(), String::new());
        }
    }
    Ok(Table {
        headers,
        rows: all_rows,
    })
}

/// Reads every row of a CSV file as text.
fn read_csv_rows(path: &Path) -> Result<Vec<Vec<String>>> {
    let bytes = std::fs::read(path).map_err(unreadable)?;
    let without_bom = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    let text = std::str::from_utf8(without_bom).map_err(|_| IoError::FileNotUtf8)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(guess_delimiter(text))
        .from_reader(text.as_bytes());
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(unreadable)?;
        rows.push(
            record
                .iter()
                .map(|cell| unguard(cell).to_string())
                .collect(),
        );
    }
    Ok(rows)
}

/// Picks the separator used in the header line: whichever of comma, semicolon
/// or tab appears most often outside quotes. A tie goes to the comma.
/// Excel in many countries saves "CSV" with semicolons.
fn guess_delimiter(text: &str) -> u8 {
    let first_line = text.lines().next().unwrap_or("");
    let mut inside_quotes = false;
    let (mut commas, mut semicolons, mut tabs) = (0, 0, 0);
    for character in first_line.chars() {
        match character {
            '"' => inside_quotes = !inside_quotes,
            ',' if !inside_quotes => commas += 1,
            ';' if !inside_quotes => semicolons += 1,
            '\t' if !inside_quotes => tabs += 1,
            _ => {}
        }
    }
    if semicolons > commas && semicolons >= tabs {
        b';'
    } else if tabs > commas && tabs > semicolons {
        b'\t'
    } else {
        b','
    }
}

/// Reads every row of one sheet as text. The first used row is the header row.
fn read_xlsx_rows(path: &Path, sheet: Option<&str>) -> Result<Vec<Vec<String>>> {
    let mut workbook: Xlsx<_> = open_workbook(path).map_err(unreadable)?;
    let names = workbook.sheet_names();
    let name = match sheet {
        Some(wanted) if names.iter().any(|name| name == wanted) => wanted.to_string(),
        Some(wanted) => {
            return Err(IoError::SheetNotFound {
                sheet: wanted.to_string(),
            });
        }
        None => names.first().cloned().ok_or(IoError::FileEmpty)?,
    };
    let range = workbook.worksheet_range(&name).map_err(unreadable)?;
    let rows = range
        .rows()
        .map(|row| row.iter().map(xlsx_cell_text).collect())
        .collect();
    Ok(rows)
}

/// Turns one Excel cell into the text a person would type for it.
/// Numbers use the shortest exact form (12.5, 40). Dates become 2026-03-01.
fn xlsx_cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(text) | Data::DateTimeIso(text) | Data::DurationIso(text) => text.clone(),
        Data::Float(number) => number.to_string(),
        Data::Int(number) => number.to_string(),
        Data::Bool(value) => value.to_string(),
        Data::Error(error) => error.to_string(),
        Data::DateTime(date_time) => {
            let as_date_time = if date_time.is_datetime() {
                date_time.as_datetime()
            } else {
                None
            };
            match as_date_time {
                Some(moment) if moment.time() == chrono::NaiveTime::MIN => {
                    moment.date().format("%Y-%m-%d").to_string()
                }
                Some(moment) => moment.format("%Y-%m-%d %H:%M:%S").to_string(),
                None => date_time.as_f64().to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_01_delimiter_is_guessed_from_the_header_line() {
        assert_eq!(guess_delimiter("a,b,c\n1;2;3"), b',');
        assert_eq!(guess_delimiter("a;b;c\n1,2,3"), b';');
        assert_eq!(guess_delimiter("a\tb\tc"), b'\t');
        assert_eq!(guess_delimiter("\"x;y\",b"), b',');
        assert_eq!(guess_delimiter("one column"), b',');
    }

    #[test]
    fn io_01_file_kind_ignores_letter_case() {
        assert_eq!(file_kind(Path::new("a.CSV")).ok(), Some(FileKind::Csv));
        assert_eq!(file_kind(Path::new("a.Xlsx")).ok(), Some(FileKind::Xlsx));
        assert!(file_kind(Path::new("a.xls")).is_err());
        assert!(file_kind(Path::new("noextension")).is_err());
    }
}
