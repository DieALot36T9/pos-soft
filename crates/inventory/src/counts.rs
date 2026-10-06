//! Stock counts (INV-06): create a count, scan and type quantities, then post or cancel.
//! Reviewing and posting differences is in `count_post.rs`.

use pos_core::{Id, Qty, Timestamp};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::convert::{add_qty, bad_column, id_at, optional_id_at};
use crate::{InventoryError, Page, batches, levels};

/// Whether a count covers every product or only some.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum CountScope {
    /// Every product. Products never counted are set to 0 when posting.
    Full,
    /// Some products (by category, brand, place or list). Products not counted stay as they are.
    Partial,
}

/// Where a count is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum CountStatus {
    Open,
    Posted,
    Cancelled,
}

/// What the user fills in to start a count. `scope_details` says what a partial count
/// covers, e.g. `{"category_ids": [...]}`; it's kept for display only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewCount {
    pub name: String,
    pub scope: CountScope,
    pub scope_details: serde_json::Value,
}

/// A new count's id and number, e.g. "SC-000001".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountRef {
    pub id: Id,
    pub number: String,
}

/// One saved count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CountRow {
    pub id: Id,
    pub number: String,
    pub name: String,
    pub scope: CountScope,
    pub scope_details: serde_json::Value,
    pub status: CountStatus,
    pub created_at: Timestamp,
    pub created_by: Id,
    pub posted_at: Option<Timestamp>,
    pub posted_by: Option<Id>,
}

/// One scan or typed quantity. Counting the same product again adds to it; a negative
/// quantity takes back a wrong scan. `batch_id` counts one batch instead of the whole product.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountEntry {
    pub product_id: Id,
    pub batch_id: Option<Id>,
    pub qty: Qty,
}

impl CountScope {
    /// The text stored in the database: "full" or "partial".
    pub fn as_str(self) -> &'static str {
        match self {
            CountScope::Full => "full",
            CountScope::Partial => "partial",
        }
    }

    /// Reads the stored text back.
    pub fn parse(text: &str) -> Result<CountScope, InventoryError> {
        match text {
            "full" => Ok(CountScope::Full),
            "partial" => Ok(CountScope::Partial),
            _ => Err(InventoryError::BadData(format!(
                "unknown count scope '{text}'"
            ))),
        }
    }
}

impl CountStatus {
    /// The text stored in the database: "open", "posted" or "cancelled".
    pub fn as_str(self) -> &'static str {
        match self {
            CountStatus::Open => "open",
            CountStatus::Posted => "posted",
            CountStatus::Cancelled => "cancelled",
        }
    }

    /// Reads the stored text back.
    pub fn parse(text: &str) -> Result<CountStatus, InventoryError> {
        match text {
            "open" => Ok(CountStatus::Open),
            "posted" => Ok(CountStatus::Posted),
            "cancelled" => Ok(CountStatus::Cancelled),
            _ => Err(InventoryError::BadData(format!(
                "unknown count status '{text}'"
            ))),
        }
    }
}

/// Starts a count and gives it the next number (SC-000001, SC-000002, …, no gaps).
pub fn create_count(
    transaction: &Transaction,
    count: &NewCount,
    now: Timestamp,
    user: &Id,
) -> Result<CountRef, InventoryError> {
    let name = count.name.trim();
    if name.is_empty() {
        return Err(InventoryError::Invalid("count_name_missing"));
    }
    let id = Id::new();
    let number = format!("SC-{:06}", next_number(transaction, "count")?);
    transaction.execute(
        "INSERT INTO inventory_counts (id, number, name, scope, scope_json, status, created_at, created_by)
         VALUES (:id, :number, :name, :scope, :scope_json, 'open', :created_at, :created_by)",
        named_params! {
            ":id": id.as_str(),
            ":number": number,
            ":name": name,
            ":scope": count.scope.as_str(),
            ":scope_json": count.scope_details.to_string(),
            ":created_at": now.0,
            ":created_by": user.as_str(),
        },
    )?;
    Ok(CountRef { id, number })
}

/// One count by id.
pub fn get_count(connection: &Connection, count_id: &Id) -> Result<CountRow, InventoryError> {
    let sql = format!("SELECT {COUNT_COLUMNS} FROM inventory_counts WHERE id = :id");
    connection
        .query_row(&sql, named_params! { ":id": count_id.as_str() }, count_from)
        .optional()?
        .ok_or_else(|| InventoryError::NotFound {
            entity: "count",
            key: count_id.to_string(),
        })
}

/// Counts, newest first.
pub fn list_counts(connection: &Connection, page: Page) -> Result<Vec<CountRow>, InventoryError> {
    let sql = format!(
        "SELECT {COUNT_COLUMNS} FROM inventory_counts
         ORDER BY created_at DESC, number DESC LIMIT :limit OFFSET :offset"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(
        named_params! { ":limit": page.limit, ":offset": page.offset },
        count_from,
    )?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Cancels an open count. Its lines stay for reference; no stock changes.
pub fn cancel_count(transaction: &Transaction, count_id: &Id) -> Result<(), InventoryError> {
    require_open(transaction, count_id)?;
    transaction.execute(
        "UPDATE inventory_counts SET status = 'cancelled' WHERE id = :id",
        named_params! { ":id": count_id.as_str() },
    )?;
    Ok(())
}

/// Adds a scan or typed quantity to an open count. The first time an item is counted,
/// its stock right now is kept as the expected quantity, so sales made later don't
/// show up as differences (INV-06). A product is counted by batch or as a whole, not both.
pub fn record_count_line(
    transaction: &Transaction,
    count_id: &Id,
    entry: &CountEntry,
    now: Timestamp,
    user: &Id,
) -> Result<(), InventoryError> {
    require_open(transaction, count_id)?;
    check_not_mixed(transaction, count_id, entry)?;
    let counted_so_far = counted_so_far(transaction, count_id, entry)?;
    let counted = add_qty(counted_so_far.unwrap_or(Qty(0)), entry.qty)?;
    if counted.0 < 0 {
        return Err(InventoryError::Invalid("counted_below_zero"));
    }
    if counted_so_far.is_none() {
        let expected = expected_now(transaction, entry)?;
        start_line(transaction, count_id, entry, expected, now, user)?;
    }
    transaction.execute(
        "UPDATE inventory_count_lines
         SET counted_milli = :counted, counted_at = :now, counted_by = :user
         WHERE count_id = :count_id AND product_id = :product_id AND batch_id IS :batch_id",
        named_params! {
            ":counted": counted.0,
            ":now": now.0,
            ":user": user.as_str(),
            ":count_id": count_id.as_str(),
            ":product_id": entry.product_id.as_str(),
            ":batch_id": entry.batch_id.as_ref().map(Id::as_str),
        },
    )?;
    Ok(())
}

/// What has been counted so far for this item, or nothing if it's the first scan.
fn counted_so_far(
    connection: &Connection,
    count_id: &Id,
    entry: &CountEntry,
) -> Result<Option<Qty>, InventoryError> {
    let counted: Option<i64> = connection
        .query_row(
            "SELECT counted_milli FROM inventory_count_lines
             WHERE count_id = :count_id AND product_id = :product_id AND batch_id IS :batch_id",
            named_params! {
                ":count_id": count_id.as_str(),
                ":product_id": entry.product_id.as_str(),
                ":batch_id": entry.batch_id.as_ref().map(Id::as_str),
            },
            |row| row.get(0),
        )
        .optional()?;
    Ok(counted.map(Qty))
}

/// Adds the line for an item's first scan, keeping the stock expected right now.
fn start_line(
    transaction: &Transaction,
    count_id: &Id,
    entry: &CountEntry,
    expected: Qty,
    now: Timestamp,
    user: &Id,
) -> Result<(), InventoryError> {
    transaction.execute(
        "INSERT INTO inventory_count_lines
           (count_id, product_id, batch_id, counted_milli, expected_milli, counted_at, counted_by)
         VALUES (:count_id, :product_id, :batch_id, 0, :expected, :now, :user)",
        named_params! {
            ":count_id": count_id.as_str(),
            ":product_id": entry.product_id.as_str(),
            ":batch_id": entry.batch_id.as_ref().map(Id::as_str),
            ":expected": expected.0,
            ":now": now.0,
            ":user": user.as_str(),
        },
    )?;
    Ok(())
}

/// Refuses unless the count is still open.
pub(crate) fn require_open(
    connection: &Connection,
    count_id: &Id,
) -> Result<CountRow, InventoryError> {
    let count = get_count(connection, count_id)?;
    if count.status != CountStatus::Open {
        return Err(InventoryError::Conflict("count_not_open"));
    }
    Ok(count)
}

/// The stock the count expects right now: the batch's, or the whole product's.
fn expected_now(connection: &Connection, entry: &CountEntry) -> Result<Qty, InventoryError> {
    match &entry.batch_id {
        Some(batch_id) => {
            let batch = batches::load(connection, batch_id)?;
            if batch.product_id != entry.product_id {
                return Err(InventoryError::Invalid("batch_of_other_product"));
            }
            Ok(batch.on_hand)
        }
        None => levels::on_hand(connection, &entry.product_id),
    }
}

/// Counting a product as a whole and by batch in one count would count it twice.
fn check_not_mixed(
    connection: &Connection,
    count_id: &Id,
    entry: &CountEntry,
) -> Result<(), InventoryError> {
    let other_way: Option<i64> = connection
        .query_row(
            "SELECT 1 FROM inventory_count_lines
             WHERE count_id = :count_id AND product_id = :product_id
               AND (batch_id IS NULL) <> :by_batch_is_null
             LIMIT 1",
            named_params! {
                ":count_id": count_id.as_str(),
                ":product_id": entry.product_id.as_str(),
                ":by_batch_is_null": entry.batch_id.is_none(),
            },
            |row| row.get(0),
        )
        .optional()?;
    if other_way.is_some() {
        return Err(InventoryError::Conflict("count_mixes_batch_and_whole"));
    }
    Ok(())
}

/// Takes the next number from a sequence, inside the caller's transaction, so numbers have no gaps.
fn next_number(transaction: &Transaction, name: &str) -> Result<i64, InventoryError> {
    transaction.execute(
        "INSERT INTO inventory_sequences (name, next) VALUES (:name, 1) ON CONFLICT(name) DO NOTHING",
        named_params! { ":name": name },
    )?;
    let number: i64 = transaction.query_row(
        "SELECT next FROM inventory_sequences WHERE name = :name",
        named_params! { ":name": name },
        |row| row.get(0),
    )?;
    transaction.execute(
        "UPDATE inventory_sequences SET next = next + 1 WHERE name = :name",
        named_params! { ":name": name },
    )?;
    Ok(number)
}

const COUNT_COLUMNS: &str = "id, number, name, scope, scope_json, status, created_at, created_by, \
     posted_at, posted_by";

fn count_from(row: &Row) -> rusqlite::Result<CountRow> {
    let scope: String = row.get(3)?;
    let scope_json: String = row.get(4)?;
    let status: String = row.get(5)?;
    Ok(CountRow {
        id: id_at(row, 0)?,
        number: row.get(1)?,
        name: row.get(2)?,
        scope: CountScope::parse(&scope).map_err(|error| bad_column(3, error))?,
        scope_details: serde_json::from_str(&scope_json).map_err(|error| {
            bad_column(
                4,
                InventoryError::BadData(format!("bad scope_json: {error}")),
            )
        })?,
        status: CountStatus::parse(&status).map_err(|error| bad_column(5, error))?,
        created_at: Timestamp(row.get(6)?),
        created_by: id_at(row, 7)?,
        posted_at: row.get::<_, Option<i64>>(8)?.map(Timestamp),
        posted_by: optional_id_at(row, 9)?,
    })
}
