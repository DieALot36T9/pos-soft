//! Payment methods on the Settings screen (SET-15): cash, card, mobile wallets and
//! others. Methods are never deleted, because old sales refer to them; they are
//! switched off instead. Each save is one transaction with its audit event.

use pos_core::{AppError, Id};
use sales::{PaymentKind, PaymentMethodInput};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::audit_actions::PAYMENT_METHOD_CHANGE;
use crate::audit_log;
use crate::state::AppState;
use crate::workflows::checkout::MethodKind;

/// One payment method as the settings list shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct PaymentMethodRow {
    pub id: Id,
    pub name: String,
    pub kind: MethodKind,
    pub requires_reference: bool,
    pub active: bool,
    pub sort: i64,
    /// Made by the app (cash, customer credit, points, exchange credit): its kind is fixed.
    pub is_system: bool,
}

/// What the admin types to add a method (`id` empty) or change one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct PaymentMethodSave {
    pub id: Option<Id>,
    /// Example: "bKash".
    pub name: String,
    pub kind: MethodKind,
    pub requires_reference: bool,
    pub active: bool,
    pub sort: i64,
}

/// Every payment method, in list order.
pub fn list(state: &AppState) -> Result<Vec<PaymentMethodRow>, AppError> {
    let methods = state.db.read(|conn| Ok(sales::payment_methods(conn)?))?;
    Ok(methods
        .into_iter()
        .map(|method| PaymentMethodRow {
            id: method.id,
            name: method.name,
            kind: method.kind.into(),
            requires_reference: method.requires_reference,
            active: method.active,
            sort: method.sort,
            is_system: method.is_system,
        })
        .collect())
}

/// Adds or changes a payment method, audited with the values saved.
pub fn save(state: &AppState, by: &Id, input: &PaymentMethodSave) -> Result<Id, AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let saved = sales::save_payment_method(
            tx,
            PaymentMethodInput {
                id: input.id.clone(),
                name: input.name.clone(),
                kind: payment_kind(input.kind),
                requires_reference: input.requires_reference,
                active: input.active,
                sort: input.sort,
            },
        )?;
        let details = serde_json::json!({
            "method_id": saved,
            "name": input.name.trim(),
            "kind": payment_kind(input.kind).as_str(),
            "requires_reference": input.requires_reference,
            "active": input.active,
            "new": input.id.is_none(),
        });
        audit_log::record(tx, clock, PAYMENT_METHOD_CHANGE, details, Some(by.clone()))?;
        Ok(saved)
    })
}

fn payment_kind(kind: MethodKind) -> PaymentKind {
    match kind {
        MethodKind::Cash => PaymentKind::Cash,
        MethodKind::Card => PaymentKind::Card,
        MethodKind::Wallet => PaymentKind::Wallet,
        MethodKind::Other => PaymentKind::Other,
        MethodKind::Credit => PaymentKind::Credit,
        MethodKind::Points => PaymentKind::Points,
        MethodKind::ExchangeCredit => PaymentKind::ExchangeCredit,
    }
}
