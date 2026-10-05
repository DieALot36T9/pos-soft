//! Reading the audit log back, newest first, with filters and pages (USR-08).

use pos_core::{AppError, BusinessDate, Id, Timestamp};
use rusqlite::{Connection, Row, named_params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::AuditError;

/// Which events to show. Every field left empty means "any".
/// Dates are business dates and both ends are included.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AuditFilter {
    /// First business day to show. Example: "2026-03-01".
    pub from: Option<BusinessDate>,
    /// Last business day to show.
    pub to: Option<BusinessDate>,
    /// Only this user's events.
    pub user_id: Option<Id>,
    /// Only this action. Example: "sale.void".
    pub action: Option<String>,
}

/// One page of results: skip `offset` events, then show up to `limit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AuditPage {
    pub limit: u32,
    pub offset: u32,
}

impl AuditPage {
    /// The most events one page may hold, so one request can't load the whole log.
    pub const MAX_LIMIT: u32 = 500;
}

/// One event as stored, ready for the audit screen and the Excel export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AuditRow {
    pub id: Id,
    pub at: Timestamp,
    pub business_date: BusinessDate,
    pub till_id: Id,
    pub user_id: Option<Id>,
    pub approved_by: Option<Id>,
    pub action: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<Id>,
    /// Always a JSON object (`record` refuses anything else).
    #[ts(type = "Record<string, unknown>")]
    pub details: Value,
}

/// Lists audit events that match `filter`, newest first, one page at a time.
pub fn query(
    conn: &Connection,
    filter: &AuditFilter,
    page: AuditPage,
) -> Result<Vec<AuditRow>, AppError> {
    if page.limit == 0 || page.limit > AuditPage::MAX_LIMIT {
        return Err(AuditError::BadPageLimit(page.limit).into());
    }
    // Events at the same moment are shown in the reverse order they were recorded (rowid).
    let mut statement = conn
        .prepare(
            "SELECT id, at, business_date, till_id, user_id, approved_by,
                    action, entity_type, entity_id, details_json
             FROM audit_events
             WHERE (:from IS NULL OR business_date >= :from)
               AND (:to IS NULL OR business_date <= :to)
               AND (:user_id IS NULL OR user_id = :user_id)
               AND (:action IS NULL OR action = :action)
             ORDER BY at DESC, rowid DESC
             LIMIT :limit OFFSET :offset",
        )
        .map_err(AuditError::from)?;
    let raw_rows = statement
        .query_map(
            named_params! {
                ":from": filter.from.map(|date| date.to_string()),
                ":to": filter.to.map(|date| date.to_string()),
                ":user_id": filter.user_id.as_ref().map(Id::as_str),
                ":action": filter.action.as_deref(),
                ":limit": page.limit,
                ":offset": page.offset,
            },
            read_raw_row,
        )
        .map_err(AuditError::from)?;

    let mut rows = Vec::new();
    for raw in raw_rows {
        let raw = raw.map_err(AuditError::from)?;
        rows.push(raw.into_row()?);
    }
    Ok(rows)
}

/// A row exactly as SQLite returns it, before the text is turned into our types.
struct RawRow {
    id: String,
    at: i64,
    business_date: String,
    till_id: String,
    user_id: Option<String>,
    approved_by: Option<String>,
    action: String,
    entity_type: Option<String>,
    entity_id: Option<String>,
    details_json: String,
}

fn read_raw_row(row: &Row) -> rusqlite::Result<RawRow> {
    Ok(RawRow {
        id: row.get("id")?,
        at: row.get("at")?,
        business_date: row.get("business_date")?,
        till_id: row.get("till_id")?,
        user_id: row.get("user_id")?,
        approved_by: row.get("approved_by")?,
        action: row.get("action")?,
        entity_type: row.get("entity_type")?,
        entity_id: row.get("entity_id")?,
        details_json: row.get("details_json")?,
    })
}

impl RawRow {
    /// Checks each stored value and turns it into our types. A broken value is a bug.
    fn into_row(self) -> Result<AuditRow, AuditError> {
        let broken = |field: &str| AuditError::BadStoredRow {
            id: self.id.clone(),
            detail: format!("{field} is not valid"),
        };
        let optional_id = |text: &Option<String>, field: &str| match text {
            Some(text) => Id::parse(text).map(Some).map_err(|_| broken(field)),
            None => Ok(None),
        };
        Ok(AuditRow {
            id: Id::parse(&self.id).map_err(|_| broken("id"))?,
            at: Timestamp(self.at),
            business_date: BusinessDate::parse(&self.business_date)
                .map_err(|_| broken("business_date"))?,
            till_id: Id::parse(&self.till_id).map_err(|_| broken("till_id"))?,
            user_id: optional_id(&self.user_id, "user_id")?,
            approved_by: optional_id(&self.approved_by, "approved_by")?,
            entity_id: optional_id(&self.entity_id, "entity_id")?,
            details: serde_json::from_str(&self.details_json)
                .map_err(|_| broken("details_json"))?,
            action: self.action,
            entity_type: self.entity_type,
        })
    }
}
