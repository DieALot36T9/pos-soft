//! The audit log screen and its Excel export (USR-08). Read only: nothing here
//! can change or delete an event.

use std::path::Path;

use audit::{AuditFilter, AuditPage, AuditRow};
use io::{XlsxCell, XlsxSheet};
use pos_core::{AppError, Id, Timestamp};
use serde::Serialize;
use ts_rs::TS;

use crate::audit_actions;
use crate::state::AppState;

/// The column headings of the exported file. English only for now: when a second
/// language comes (ADR-022), they move next to the `ReceiptLabels`.
const EXPORT_HEADERS: [&str; 9] = [
    "Time",
    "Business date",
    "Till",
    "User",
    "Approved by",
    "Action",
    "Item type",
    "Item",
    "Details",
];

/// One page of events, with the names the screen needs to show them.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct AuditList {
    pub rows: Vec<AuditRow>,
    /// Every user, so the screen can show names instead of ids, and filter by user.
    pub people: Vec<Person>,
    /// Every action name, for the action filter.
    pub actions: Vec<String>,
}

/// A user's id and names, for the audit screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Person {
    pub id: Id,
    pub username: String,
    pub display_name: String,
}

/// One page of the audit log, newest first.
pub fn list(
    state: &AppState,
    filter: &AuditFilter,
    page: AuditPage,
) -> Result<AuditList, AppError> {
    let rows = state.db.read(|conn| audit::query(conn, filter, page))?;
    Ok(AuditList {
        rows,
        people: people(state)?,
        actions: audit_actions::ALL
            .iter()
            .map(|name| name.to_string())
            .collect(),
    })
}

/// Writes every event the filter finds into an .xlsx file. Returns how many.
pub fn export(state: &AppState, filter: &AuditFilter, file: &Path) -> Result<u32, AppError> {
    let people = people(state)?;
    let name_of = |id: &Option<Id>| -> String {
        match id {
            None => String::new(),
            Some(id) => people
                .iter()
                .find(|person| &person.id == id)
                .map(|person| person.display_name.clone())
                .unwrap_or_else(|| id.to_string()),
        }
    };
    let offset_minutes = state.clock.local_offset_minutes();
    let mut rows = Vec::new();
    let mut offset = 0;
    loop {
        let page = AuditPage {
            limit: AuditPage::MAX_LIMIT,
            offset,
        };
        let found = state.db.read(|conn| audit::query(conn, filter, page))?;
        let count = found.len();
        for event in found {
            rows.push(vec![
                XlsxCell::Text(local_time(event.at, offset_minutes)),
                XlsxCell::Date(event.business_date.0),
                XlsxCell::Text(event.till_id.to_string()),
                XlsxCell::Text(name_of(&event.user_id)),
                XlsxCell::Text(name_of(&event.approved_by)),
                XlsxCell::Text(event.action),
                XlsxCell::Text(event.entity_type.unwrap_or_default()),
                XlsxCell::Text(event.entity_id.map(|id| id.to_string()).unwrap_or_default()),
                XlsxCell::Text(event.details.to_string()),
            ]);
        }
        if count < AuditPage::MAX_LIMIT as usize {
            break;
        }
        offset += AuditPage::MAX_LIMIT;
    }
    let sheet = XlsxSheet {
        name: "Audit log".into(),
        headers: EXPORT_HEADERS.iter().map(|text| text.to_string()).collect(),
        rows,
    };
    let written = u32::try_from(sheet.rows.len())
        .map_err(|_| AppError::internal("too many audit events to export"))?;
    io::write_xlsx(file, &[sheet])?;
    Ok(written)
}

fn people(state: &AppState) -> Result<Vec<Person>, AppError> {
    let users = state.db.read(|conn| Ok(auth::list_users(conn)?))?;
    Ok(users
        .into_iter()
        .map(|user| Person {
            id: user.id,
            username: user.username,
            display_name: user.display_name,
        })
        .collect())
}

/// "2026-10-05 14:00:00" in the shop's time.
fn local_time(at: Timestamp, offset_minutes: i32) -> String {
    let local = at.0 + i64::from(offset_minutes) * 60_000;
    chrono::DateTime::from_timestamp_millis(local)
        .map(|time| time.naive_utc().format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usr_08_export_times_are_in_the_shops_time() {
        // 2026-10-05 08:00 UTC is 14:00 in Dhaka (UTC+6).
        assert_eq!(
            local_time(Timestamp(1_791_187_200_000), 360),
            "2026-10-05 14:00:00"
        );
    }
}
