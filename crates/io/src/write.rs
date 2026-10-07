//! Writing .csv files (with the `csv` crate) and .xlsx files (with
//! `rust_xlsxwriter`) for exports (IO-03).

use std::path::Path;

use rust_xlsxwriter::{ExcelDateTime, Format, Workbook, Worksheet};

use crate::error::{Result, not_saved};
use crate::guard::guard;
use crate::{Table, XlsxCell, XlsxSheet};

/// Writes a CSV file Excel opens correctly: UTF-8 with a BOM (so ৳ shows up),
/// commas, and line ends of carriage return + line feed (RFC 4180).
/// Cells Excel could run as formulas get a leading apostrophe.
pub fn write_csv(path: &Path, table: &Table) -> Result<()> {
    let mut bytes = b"\xef\xbb\xbf".to_vec();
    {
        let mut writer = csv::WriterBuilder::new()
            .flexible(true)
            .terminator(csv::Terminator::CRLF)
            .from_writer(&mut bytes);
        let all_rows = std::iter::once(&table.headers).chain(table.rows.iter());
        for row in all_rows {
            let guarded: Vec<String> = row.iter().map(|cell| guard(cell)).collect();
            writer.write_record(&guarded).map_err(not_saved)?;
        }
        writer.flush().map_err(not_saved)?;
    }
    std::fs::write(path, bytes).map_err(not_saved)
}

/// Writes an .xlsx file with one tab per sheet. The header row is bold.
/// Money, quantities and percentages are real numbers with a number format.
pub fn write_xlsx(path: &Path, sheets: &[XlsxSheet]) -> Result<()> {
    let mut workbook = Workbook::new();
    for sheet in sheets {
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(safe_sheet_name(&sheet.name))
            .map_err(not_saved)?;
        let headers: Vec<XlsxCell> = sheet.headers.iter().cloned().map(XlsxCell::Text).collect();
        write_row(worksheet, 0, &headers, &bold())?;
        for (index, row) in sheet.rows.iter().enumerate() {
            write_row(worksheet, row_number(index + 1)?, row, &Format::new())?;
        }
    }
    workbook.save(path).map_err(not_saved)
}

/// Writes one row of typed cells starting at column A.
/// `base` is added to every cell's format (for example bold for headers).
pub(crate) fn write_row(
    worksheet: &mut Worksheet,
    row: u32,
    cells: &[XlsxCell],
    base: &Format,
) -> Result<()> {
    for (index, cell) in cells.iter().enumerate() {
        let column = u16::try_from(index).map_err(not_saved)?;
        write_cell(worksheet, row, column, cell, base)?;
    }
    Ok(())
}

/// Writes one typed cell. Text is always stored as text, never as a formula.
fn write_cell(
    worksheet: &mut Worksheet,
    row: u32,
    column: u16,
    cell: &XlsxCell,
    base: &Format,
) -> Result<()> {
    match cell {
        XlsxCell::Empty => return Ok(()),
        XlsxCell::Text(text) => worksheet.write_string_with_format(row, column, text, base),
        XlsxCell::Int(number) => {
            let value = number_from_text(&number.to_string())?;
            worksheet.write_number_with_format(row, column, value, base)
        }
        XlsxCell::Money {
            amount,
            minor_digits,
        } => {
            let value = number_from_text(&decimal_text(amount.0, *minor_digits))?;
            let format = base
                .clone()
                .set_num_format(money_number_format(*minor_digits));
            worksheet.write_number_with_format(row, column, value, &format)
        }
        XlsxCell::Qty(quantity) => {
            let value = number_from_text(&decimal_text(quantity.0, 3))?;
            let format = base.clone().set_num_format("#,##0.000");
            worksheet.write_number_with_format(row, column, value, &format)
        }
        XlsxCell::Percent(rate) => {
            // Excel stores 12.5% as 0.125, which is the rate in basis points / 10000.
            let value = number_from_text(&decimal_text(i64::from(rate.0), 4))?;
            let format = base.clone().set_num_format("0.00%");
            worksheet.write_number_with_format(row, column, value, &format)
        }
        XlsxCell::Date(date) => {
            let excel_date = excel_date(*date)?;
            let format = base.clone().set_num_format("yyyy-mm-dd");
            worksheet.write_datetime_with_format(row, column, excel_date, &format)
        }
    }
    .map_err(not_saved)?;
    Ok(())
}

/// The bold format used for header and total rows.
pub(crate) fn bold() -> Format {
    Format::new().set_bold()
}

/// Turns a list position into an Excel row number.
pub(crate) fn row_number(index: usize) -> Result<u32> {
    u32::try_from(index).map_err(not_saved)
}

/// Writes whole minor units as a decimal number. Example: (-1250, 2) → "-12.50".
/// Done with text, not floating-point maths, so no digits are lost. Exports use it
/// for money and quantities in CSV files (IO-04).
pub fn decimal_text(value: i64, decimals: u8) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let digits = value.unsigned_abs().to_string();
    let decimals = usize::from(decimals);
    if decimals == 0 {
        return format!("{sign}{digits}");
    }
    let padded = format!("{digits:0>width$}", width = decimals + 1);
    let (whole, fraction) = padded.split_at(padded.len() - decimals);
    format!("{sign}{whole}.{fraction}")
}

/// Excel stores every number as a floating-point number. Reading it from the
/// exact decimal text gives the closest one, with no maths in between.
fn number_from_text(text: &str) -> Result<f64> {
    text.parse::<f64>().map_err(not_saved)
}

/// The Excel number format for money. Example: 2 decimals → "#,##0.00".
fn money_number_format(minor_digits: u8) -> String {
    if minor_digits == 0 {
        "#,##0".to_string()
    } else {
        format!("#,##0.{}", "0".repeat(usize::from(minor_digits)))
    }
}

/// Turns a date into Excel's date type.
fn excel_date(date: chrono::NaiveDate) -> Result<ExcelDateTime> {
    use chrono::Datelike;
    let year = u16::try_from(date.year()).map_err(not_saved)?;
    let month = u8::try_from(date.month()).map_err(not_saved)?;
    let day = u8::try_from(date.day()).map_err(not_saved)?;
    ExcelDateTime::from_ymd(year, month, day).map_err(not_saved)
}

/// Makes a name Excel accepts for a tab: none of [ ] : * ? / \, no apostrophe
/// at either end, 31 characters or fewer, never empty.
/// Example: "Sales [1/2]" → "Sales (1-2)".
pub(crate) fn safe_sheet_name(name: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|character| match character {
            '[' => '(',
            ']' => ')',
            ':' | '*' | '?' | '/' | '\\' => '-',
            other => other,
        })
        .collect();
    let trimmed = replaced.trim().trim_matches('\'');
    let short: String = trimmed.chars().take(31).collect();
    if short.is_empty() {
        "Sheet1".to_string()
    } else {
        short
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_04_money_is_written_with_the_currency_decimals() {
        assert_eq!(decimal_text(1250, 2), "12.50");
        assert_eq!(decimal_text(-5, 2), "-0.05");
        assert_eq!(decimal_text(7, 0), "7");
        assert_eq!(decimal_text(1234, 3), "1.234");
        assert_eq!(decimal_text(i64::MIN, 2), "-92233720368547758.08");
        assert_eq!(money_number_format(2), "#,##0.00");
        assert_eq!(money_number_format(0), "#,##0");
    }

    #[test]
    fn io_03_sheet_names_are_made_safe() {
        assert_eq!(safe_sheet_name("Sales [1/2]"), "Sales (1-2)");
        assert_eq!(safe_sheet_name("'Stock'"), "Stock");
        assert_eq!(safe_sheet_name(""), "Sheet1");
        assert_eq!(safe_sheet_name(&"x".repeat(40)).len(), 31);
    }
}
