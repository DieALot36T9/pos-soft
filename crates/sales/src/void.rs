//! Voiding a completed sale (RET-04). The sale keeps its number and contents;
//! only its status and void fields change.

use pos_core::{Id, Timestamp};
use rusqlite::{Transaction, named_params};

use crate::sale_read::get_sale_status;
use crate::{SaleStatus, SalesError};

/// Marks a sale as void. The workflow checks `SaleVoid` and that the sale's shift
/// (`Sale::shift_id`) is still open, then reverses stock, shift, points and credit.
/// Refused if the sale is already void, or if any of it was returned (`sale_has_returns`),
/// because those items already came back and would be reversed twice.
pub fn void(
    tx: &Transaction,
    sale_id: &Id,
    by: &Id,
    approved_by: Option<&Id>,
    reason: &str,
    now: Timestamp,
) -> Result<(), SalesError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(SalesError::invalid("void_reason_missing"));
    }
    if get_sale_status(tx, sale_id)? == SaleStatus::Voided {
        return Err(SalesError::conflict("sale_already_voided"));
    }
    let returned: bool = tx.query_row(
        "SELECT EXISTS (SELECT 1 FROM sales_returns WHERE sale_id = :sale)",
        named_params! { ":sale": sale_id.as_str() },
        |row| row.get(0),
    )?;
    if returned {
        return Err(SalesError::conflict("sale_has_returns"));
    }
    tx.execute(
        "UPDATE sales_sales
         SET status = 'voided', voided_at = :now, voided_by = :by, void_approved_by = :approved_by, void_reason = :reason
         WHERE id = :id",
        named_params! {
            ":now": now.0, ":by": by.as_str(), ":approved_by": approved_by.map(Id::as_str),
            ":reason": reason, ":id": sale_id.as_str(),
        },
    )?;
    Ok(())
}
