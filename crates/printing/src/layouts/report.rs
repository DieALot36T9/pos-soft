//! Reports on receipt paper: X and Z reports and the daily summary (rule SHF-08).
//! `reports` builds the `ReportTable`; this only lays it out.

use pos_core::{Cell, ColumnKind, CurrencyFormat, ReportTable};

use super::{BOTTOM_FEED_DOTS, feed, row, rule, text};
use crate::doc::{Align, Block, Doc, RuleStyle, TextSize};
use crate::labels::{ReceiptLabels, fill};
use crate::layouts::receipt_data::ReceiptLayout;
use crate::numbers::{money, percent, quantity};

/// Lays out a report for receipt paper: shop name, title, period and filters,
/// then each table (heading row, rows, bold totals) and its smaller sections.
pub fn shift_report(r: &ReportTable, l: &ReceiptLayout, labels: &ReceiptLabels) -> Doc {
    let mut blocks = vec![
        text(&l.shop.name, TextSize::Large, true, Align::Center),
        text(&r.title, TextSize::Large, true, Align::Center),
        rule(RuleStyle::Single),
    ];
    let from = r.params.from.as_ref().map(ToString::to_string);
    let to = r.params.to.as_ref().map(ToString::to_string);
    let period = match (from, to) {
        (Some(from), Some(to)) => Some(fill(&labels.date_range, &[("from", &from), ("to", &to)])),
        (Some(day), None) | (None, Some(day)) => Some(day),
        (None, None) => None,
    };
    if let Some(period) = period {
        blocks.push(row(&labels.period, period, TextSize::Small, false));
    }
    for filter in &r.params.filters {
        blocks.push(row(&filter.name, &filter.value, TextSize::Small, false));
    }
    push_table(&mut blocks, r, &l.currency);
    for section in &r.sections {
        push_section(&mut blocks, section, &l.currency);
    }
    blocks.push(feed(BOTTOM_FEED_DOTS));
    Doc {
        width_dots: l.paper.dots(),
        blocks,
    }
}

/// A smaller table inside the report, with its own title. Sections can hold sections.
fn push_section(blocks: &mut Vec<Block>, section: &ReportTable, currency: &CurrencyFormat) {
    blocks.push(feed(12));
    blocks.push(text(&section.title, TextSize::Normal, true, Align::Left));
    push_table(blocks, section, currency);
    for inner in &section.sections {
        push_section(blocks, inner, currency);
    }
}

/// The heading row, the rows, and the totals in bold under a line.
fn push_table(blocks: &mut Vec<Block>, table: &ReportTable, currency: &CurrencyFormat) {
    if table.columns.is_empty() {
        return;
    }
    let kinds: Vec<ColumnKind> = table.columns.iter().map(|column| column.kind).collect();
    let titles: Vec<String> = table
        .columns
        .iter()
        .map(|column| column.title.clone())
        .collect();
    blocks.push(rule(RuleStyle::Single));
    blocks.push(table_row(&kinds, titles, true));
    for cells in &table.rows {
        let texts = cells.iter().map(|cell| cell_text(cell, currency)).collect();
        blocks.push(table_row(&kinds, texts, false));
    }
    if let Some(totals) = &table.totals {
        blocks.push(rule(RuleStyle::Single));
        let texts = totals
            .iter()
            .map(|cell| cell_text(cell, currency))
            .collect();
        blocks.push(table_row(&kinds, texts, true));
    }
}

/// One row of a table. Text gets twice the room of a number; numbers sit on the right.
fn table_row(kinds: &[ColumnKind], texts: Vec<String>, bold: bool) -> Block {
    let cells = texts
        .into_iter()
        .enumerate()
        .map(|(index, words)| {
            let kind = kinds.get(index).copied().unwrap_or(ColumnKind::Text);
            match kind {
                ColumnKind::Text => (words, 2, Align::Left),
                ColumnKind::Date => (words, 1, Align::Left),
                _ => (words, 1, Align::Right),
            }
        })
        .collect();
    Block::Columns {
        cells,
        size: TextSize::Small,
        bold,
    }
}

/// A report value as printed. Money has no symbol, so the columns stay narrow.
fn cell_text(cell: &Cell, currency: &CurrencyFormat) -> String {
    match cell {
        Cell::Empty => String::new(),
        Cell::Text(words) => words.clone(),
        Cell::Money(amount) => money(*amount, currency),
        Cell::Qty(qty) => quantity(*qty, "", &currency.decimal_sep),
        Cell::Int(number) => number.to_string(),
        Cell::Percent(rate) => percent(*rate, &currency.decimal_sep),
        Cell::Date(date) => date.to_string(),
    }
}
