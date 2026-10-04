//! A product's stock history with a running balance (INV-07).

use pos_core::{BusinessDate, Id, Qty};
use rusqlite::{Connection, named_params};
use serde::{Deserialize, Serialize};

use crate::ledger::{LEDGER_COLUMNS, row_from};
use crate::{InventoryError, LedgerRow, MoveKind, Page};

/// Which history rows to show. Empty `kinds` means every kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryFilter {
    pub from: Option<BusinessDate>,
    pub to: Option<BusinessDate>,
    pub kinds: Vec<MoveKind>,
    pub page: Page,
}

/// One ledger row and the product's stock right after it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerRowWithBalance {
    pub entry: LedgerRow,
    pub balance_after: Qty,
}

/// A product's ledger rows, newest first, each with the balance after it.
/// The balance counts every earlier row, even ones the filter hides, so it is
/// always the real stock at that moment.
pub fn history(
    connection: &Connection,
    product: &Id,
    filter: &HistoryFilter,
) -> Result<Vec<LedgerRowWithBalance>, InventoryError> {
    let kinds: Vec<&str> = filter.kinds.iter().map(|kind| kind.as_str()).collect();
    let kinds_json = serde_json::to_string(&kinds)
        .map_err(|error| InventoryError::BadData(error.to_string()))?;
    // The inner query works out the balance over all rows; the outer one filters.
    let sql = format!(
        "SELECT {LEDGER_COLUMNS}, balance_after FROM (
           SELECT {LEDGER_COLUMNS}, rowid AS position,
             SUM(qty_delta_milli) OVER (ORDER BY at, rowid ROWS UNBOUNDED PRECEDING) AS balance_after
           FROM inventory_ledger WHERE product_id = :product_id
         )
         WHERE (:from IS NULL OR business_date >= :from)
           AND (:to IS NULL OR business_date <= :to)
           AND (:any_kind OR kind IN (SELECT value FROM json_each(:kinds)))
         ORDER BY at DESC, position DESC
         LIMIT :limit OFFSET :offset"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        named_params! {
            ":product_id": product.as_str(),
            ":from": filter.from.map(|date| date.to_string()),
            ":to": filter.to.map(|date| date.to_string()),
            ":any_kind": kinds.is_empty(),
            ":kinds": kinds_json,
            ":limit": filter.page.limit,
            ":offset": filter.page.offset,
        },
        |row| {
            Ok(LedgerRowWithBalance {
                entry: row_from(row)?,
                balance_after: Qty(row.get(14)?),
            })
        },
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}
