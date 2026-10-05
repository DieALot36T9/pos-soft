//! Recording one audit event (USR-08).

use pos_core::{AppError, BusinessDate, Id, Timestamp};
use rusqlite::{Transaction, named_params};
use serde_json::Value;

use crate::AuditError;

/// Keys that must never appear in `details`, at any depth (Security Model: never logged).
const SECRET_KEYS: &[&str] = &[
    "password",
    "pin",
    "recovery_code",
    "override_token",
    "token",
    "secret",
];

/// One sensitive action to write down. `app` fills it in; action names are the
/// constants in `app/src/audit_actions.rs`, like `sale.void`.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditEvent {
    /// What happened, as lower-case words joined by dots. Example: "sale.void".
    pub action: &'static str,
    /// The kind of thing it happened to. Example: "sale".
    pub entity_type: Option<&'static str>,
    /// The id of that thing.
    pub entity_id: Option<Id>,
    /// Extra facts as a JSON object. Example: `{"reason": "wrong item"}`. Never secrets.
    pub details: Value,
    /// Who did it. `None` for things the app does by itself, like a timed backup.
    pub user_id: Option<Id>,
    /// The admin who approved it, when an approval was needed.
    pub approved_by: Option<Id>,
}

/// When and where an event happened. `app` builds it from the clock and the till settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditCtx {
    pub at: Timestamp,
    pub business_date: BusinessDate,
    pub till_id: Id,
}

/// Writes one event to the audit log inside the caller's transaction and returns its id.
/// If the caller's transaction is rolled back, the event is gone too.
pub fn record(tx: &Transaction, ev: AuditEvent, ctx: &AuditCtx) -> Result<Id, AppError> {
    check_action(ev.action)?;
    check_details(&ev.details)?;
    let id = Id::new();
    tx.execute(
        "INSERT INTO audit_events
           (id, at, business_date, till_id, user_id, approved_by,
            action, entity_type, entity_id, details_json)
         VALUES
           (:id, :at, :business_date, :till_id, :user_id, :approved_by,
            :action, :entity_type, :entity_id, :details_json)",
        named_params! {
            ":id": id.as_str(),
            ":at": ctx.at.0,
            ":business_date": ctx.business_date.to_string(),
            ":till_id": ctx.till_id.as_str(),
            ":user_id": ev.user_id.as_ref().map(Id::as_str),
            ":approved_by": ev.approved_by.as_ref().map(Id::as_str),
            ":action": ev.action,
            ":entity_type": ev.entity_type,
            ":entity_id": ev.entity_id.as_ref().map(Id::as_str),
            ":details_json": ev.details.to_string(),
        },
    )
    .map_err(AuditError::from)?;
    Ok(id)
}

/// An action name is one or more lower-case words (letters, digits, `_`) joined by dots.
/// Example: "sale.void", "drawer.no_sale". This catches typos before they reach the log.
fn check_action(action: &str) -> Result<(), AuditError> {
    let word_is_plain = |word: &str| {
        !word.is_empty()
            && word
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    if action.split('.').all(word_is_plain) {
        Ok(())
    } else {
        Err(AuditError::BadAction(action.to_string()))
    }
}

/// Details must be a JSON object with no secret-sounding keys anywhere inside it.
fn check_details(details: &Value) -> Result<(), AuditError> {
    if !details.is_object() {
        return Err(AuditError::DetailsNotObject);
    }
    match find_secret_key(details) {
        Some(key) => Err(AuditError::SecretInDetails(key)),
        None => Ok(()),
    }
}

/// Looks through objects and lists for a key on the `SECRET_KEYS` list.
fn find_secret_key(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                if SECRET_KEYS.contains(&key.to_ascii_lowercase().as_str()) {
                    return Some(key.clone());
                }
                if let Some(found) = find_secret_key(inner) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(find_secret_key),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn usr_08_plain_action_names_are_accepted() {
        for action in ["sale.void", "drawer.no_sale", "logging.on", "backup2.made"] {
            assert!(check_action(action).is_ok(), "{action}");
        }
    }

    #[test]
    fn usr_08_secret_keys_are_found_in_lists_too() {
        let details = json!({ "users": [{ "name": "Rina" }, { "Password": "x" }] });
        assert_eq!(find_secret_key(&details), Some("Password".to_string()));
    }

    #[test]
    fn usr_08_ordinary_details_pass() {
        let details = json!({ "invoice_no": "T1-000123", "lines": [{ "name": "Milk 1L" }] });
        assert!(check_details(&details).is_ok());
    }
}
