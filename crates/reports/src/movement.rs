//! Stock movement for a period (RPT-05).

use std::collections::HashMap;

use pos_core::{Cell, ColumnKind, Qty, ReportTable};
use rusqlite::Connection;

use crate::sql::{query, text as text_param};
use crate::table::{column, new_table, range_params, text};
use crate::{DateRange, ReportsError};

/// Opening, received, sold, returned, adjusted and closing, in that order.
type Movement = [i64; 6];

/// Which column a ledger row in the period adds to. `None` for an unknown kind, which
/// still counts in the closing stock. Received includes opening stock and goods sent
/// back to suppliers; sold includes voided sales coming back; adjusted includes counts.
pub(crate) fn column_of(kind: &str) -> Option<usize> {
    match kind {
        "receive" | "opening" | "supplier_return" => Some(1),
        "sale" | "sale_void" => Some(2),
        "return" => Some(3),
        "adjust" | "count" => Some(4),
        _ => None,
    }
}

/// Stock movement for a period, per product: opening, received, sold, returned,
/// adjusted and closing quantities (signed: − is stock going out). Sorted by name.
///
/// The ledger is read from start to end once and added up here. A year has a ledger
/// row for every sale line (about 1.6 million), and grouping them in the database
/// meant sorting them all, or jumping around the table by product (RPT-11).
pub fn stock_movement(conn: &Connection, range: DateRange) -> Result<ReportTable, ReportsError> {
    let range = range.check()?;
    let from = range.from.to_string();
    let params = [(":to", text_param(&range.to.to_string()))];
    let mut products: HashMap<String, Movement> = HashMap::new();
    // `+` stops the database from using the date index: reading the whole table in
    // order is faster than following the index to almost every row.
    query(
        conn,
        "SELECT l.product_id, l.kind, l.business_date, l.qty_delta_milli
         FROM inventory_v_ledger l WHERE +l.business_date <= :to",
        &params,
        |row| {
            let product = row.get_ref(0)?.as_str()?;
            let movement = match products.get_mut(product) {
                Some(movement) => movement,
                None => products.entry(product.to_string()).or_insert([0; 6]),
            };
            let qty: i64 = row.get(3)?;
            let column = if row.get_ref(2)?.as_str()? < from.as_str() {
                Some(0)
            } else {
                column_of(row.get_ref(1)?.as_str()?)
            };
            if let Some(column) = column {
                movement[column] = add_qty(movement[column], qty)?;
            }
            movement[5] = add_qty(movement[5], qty)?;
            Ok(())
        },
    )?;
    let names: HashMap<String, String> = query(
        conn,
        "SELECT id, name FROM catalog_v_products",
        &[],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?
    .into_iter()
    .collect();
    let mut rows: Vec<(String, Movement)> = products
        .into_iter()
        .map(|(product, movement)| (names.get(&product).cloned().unwrap_or(product), movement))
        .collect();
    rows.sort();

    let mut table = new_table(
        "Stock movement",
        range_params(range),
        vec![
            column("product", "Product", ColumnKind::Text),
            column("opening", "Opening", ColumnKind::Qty),
            column("received", "Received", ColumnKind::Qty),
            column("sold", "Sold", ColumnKind::Qty),
            column("returned", "Returned", ColumnKind::Qty),
            column("adjusted", "Adjusted", ColumnKind::Qty),
            column("closing", "Closing", ColumnKind::Qty),
        ],
    );
    for (name, movement) in rows {
        let mut cells = vec![text(&name)];
        cells.extend(movement.map(|milli| Cell::Qty(Qty(milli))));
        table.rows.push(cells);
    }
    Ok(table)
}

/// `a + b` for quantities, as an error on overflow (inside the row reader).
fn add_qty(a: i64, b: i64) -> rusqlite::Result<i64> {
    a.checked_add(b)
        .ok_or(rusqlite::Error::IntegralValueOutOfRange(0, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpt_05_each_ledger_kind_goes_to_its_column() {
        assert_eq!(column_of("receive"), Some(1));
        assert_eq!(column_of("supplier_return"), Some(1));
        assert_eq!(column_of("sale_void"), Some(2));
        assert_eq!(column_of("return"), Some(3));
        assert_eq!(column_of("count"), Some(4));
        assert_eq!(column_of("something_new"), None);
    }
}
