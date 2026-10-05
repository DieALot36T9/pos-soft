//! Exporting a finished report to Excel (rule RPT-09): a header with the shop
//! name, report name, date range and time it was made, then the table with
//! real numbers, its totals and its sections.

use std::path::Path;

use pos_core::{Cell, ReportTable};
use rust_xlsxwriter::{Format, Workbook, Worksheet};

use crate::error::{Result, not_saved};
use crate::write::{bold, row_number, safe_sheet_name, write_row};
use crate::{ExportHeader, XlsxCell};

/// Writes one report into an .xlsx file with one tab named after the report.
pub fn report_to_xlsx(path: &Path, report: &ReportTable, header: &ExportHeader) -> Result<()> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name(safe_sheet_name(&report.title))
        .map_err(not_saved)?;
    let widest = widest_table(report);
    for column in 0..widest {
        let column = u16::try_from(column).map_err(not_saved)?;
        worksheet.set_column_width(column, 16).map_err(not_saved)?;
    }

    let mut next_row = 0;
    for (index, line) in header_lines(report, header).iter().enumerate() {
        let format = if index < 2 { bold() } else { Format::new() };
        let cells = [XlsxCell::Text(line.clone())];
        write_row(worksheet, row_number(next_row)?, &cells, &format)?;
        next_row += 1;
    }
    next_row += 1;
    write_table(worksheet, &mut next_row, report, header)?;
    workbook.save(path).map_err(not_saved)
}

/// The text lines above the table. Example:
/// "Rahim Store", "Daily summary", "Period: 2026-03-01 to 2026-03-07",
/// "Generated: 2026-03-08 09:15", then one "Cashier: Rina" line per filter.
fn header_lines(report: &ReportTable, header: &ExportHeader) -> Vec<String> {
    let labels = &header.labels;
    let mut lines = vec![header.shop_name.clone(), report.title.clone()];
    let date_text = |date: &pos_core::BusinessDate| date.0.format("%Y-%m-%d").to_string();
    let period = match (&report.params.from, &report.params.to) {
        (Some(from), Some(to)) => Some(format!(
            "{} {} {}",
            date_text(from),
            labels.period_to,
            date_text(to)
        )),
        (Some(only), None) | (None, Some(only)) => Some(date_text(only)),
        (None, None) => None,
    };
    if let Some(period) = period {
        lines.push(format!("{}: {period}", labels.period));
    }
    let generated = header.generated_at.format("%Y-%m-%d %H:%M");
    lines.push(format!("{}: {generated}", labels.generated));
    for filter in &report.params.filters {
        lines.push(format!("{}: {}", filter.name, filter.value));
    }
    lines
}

/// Writes the column titles, rows and totals, then each section below it,
/// each after a blank row and its own title.
fn write_table(
    worksheet: &mut Worksheet,
    next_row: &mut usize,
    table: &ReportTable,
    header: &ExportHeader,
) -> Result<()> {
    let titles: Vec<XlsxCell> = table
        .columns
        .iter()
        .map(|column| XlsxCell::Text(column.title.clone()))
        .collect();
    write_row(worksheet, row_number(*next_row)?, &titles, &bold())?;
    *next_row += 1;
    for row in &table.rows {
        let cells = to_xlsx_cells(row, header.money_minor_digits);
        write_row(worksheet, row_number(*next_row)?, &cells, &Format::new())?;
        *next_row += 1;
    }
    if let Some(totals) = &table.totals {
        let mut cells = to_xlsx_cells(totals, header.money_minor_digits);
        if cells.first() == Some(&XlsxCell::Empty) {
            cells[0] = XlsxCell::Text(header.labels.total.clone());
        }
        write_row(worksheet, row_number(*next_row)?, &cells, &bold())?;
        *next_row += 1;
    }
    for section in &table.sections {
        *next_row += 1;
        let title = [XlsxCell::Text(section.title.clone())];
        write_row(worksheet, row_number(*next_row)?, &title, &bold())?;
        *next_row += 1;
        write_table(worksheet, next_row, section, header)?;
    }
    Ok(())
}

/// Turns report cells into Excel cells. Money uses the shop's currency decimals.
fn to_xlsx_cells(cells: &[Cell], money_minor_digits: u8) -> Vec<XlsxCell> {
    cells
        .iter()
        .map(|cell| match cell {
            Cell::Empty => XlsxCell::Empty,
            Cell::Text(text) => XlsxCell::Text(text.clone()),
            Cell::Money(amount) => XlsxCell::Money {
                amount: *amount,
                minor_digits: money_minor_digits,
            },
            Cell::Qty(quantity) => XlsxCell::Qty(*quantity),
            Cell::Int(number) => XlsxCell::Int(*number),
            Cell::Percent(rate) => XlsxCell::Percent(*rate),
            Cell::Date(date) => XlsxCell::Date(date.0),
        })
        .collect()
}

/// The most columns in the report or any of its sections.
fn widest_table(table: &ReportTable) -> usize {
    let widest_section = table.sections.iter().map(widest_table).max().unwrap_or(0);
    table.columns.len().max(widest_section)
}
