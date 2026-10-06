//! Writing audit events with the till and business date filled in (USR-08).

use pos_core::{AppError, Clock, Id, business_date};
use rusqlite::Transaction;
use serde_json::Value;

use settings::TillSettings;

/// Records one event in the caller's transaction. Before the till step of setup
/// there is no till yet, so nothing is recorded: setup itself isn't audited.
pub fn record(
    tx: &Transaction,
    clock: &dyn Clock,
    action: &'static str,
    details: Value,
    user_id: Option<Id>,
) -> Result<(), AppError> {
    record_approved(tx, clock, action, details, user_id, None)
}

/// Like `record`, with the admin who approved the action (USR-05).
pub fn record_approved(
    tx: &Transaction,
    clock: &dyn Clock,
    action: &'static str,
    details: Value,
    user_id: Option<Id>,
    approved_by: Option<Id>,
) -> Result<(), AppError> {
    let till: TillSettings = settings::get(tx)?;
    let Some(till_id) = till.till_id else {
        return Ok(());
    };
    let now = clock.now();
    let context = audit::AuditCtx {
        at: now,
        business_date: business_date(
            now,
            clock.local_offset_minutes(),
            till.business_day_cutoff_hour,
        )?,
        till_id,
    };
    let event = audit::AuditEvent {
        action,
        entity_type: None,
        entity_id: None,
        details,
        user_id,
        approved_by,
    };
    audit::record(tx, event, &context)?;
    Ok(())
}
