//! Tax rates (rules TAX-02, TAX-03, TAX-10): add, change and deactivate, each in one
//! transaction with its audit event. Rates are never deleted.

use pos_core::{AppError, Id};
use serde::Deserialize;
use tax::{NewRate, RateUpdate, RateWithComponents};
use ts_rs::TS;

use crate::audit_actions::TAX_RATE_CHANGE;
use crate::audit_log;
use crate::state::AppState;

/// A rate as typed on the screen. Without `id` it is a new rate.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[ts(export)]
pub struct RateInput {
    pub id: Option<Id>,
    pub rate: NewRate,
}

/// Every rate, active ones and (when asked) deactivated ones, in list order.
pub fn list(state: &AppState, include_inactive: bool) -> Result<Vec<RateWithComponents>, AppError> {
    state.db.read(|conn| Ok(tax::list(conn, include_inactive)?))
}

/// Adds a rate or changes one. Returns the rate's id.
pub fn save(state: &AppState, input: RateInput, by: Option<&Id>) -> Result<Id, AppError> {
    let clock = state.clock.as_ref();
    let now = clock.now();
    state.db.write(|tx| {
        let details = serde_json::json!({
            "name": input.rate.name,
            "rate_bps": input.rate.rate_bps,
            "change": if input.id.is_some() { "update" } else { "create" },
        });
        let id = match &input.id {
            Some(id) => {
                let update = RateUpdate {
                    name: input.rate.name.clone(),
                    rate_bps: input.rate.rate_bps,
                    components: input.rate.components.clone(),
                    sort: input.rate.sort,
                };
                tax::update_rate(tx, id, update, now)?;
                id.clone()
            }
            None => tax::create_rate(tx, input.rate.clone(), now)?,
        };
        audit_log::record(tx, clock, TAX_RATE_CHANGE, details, by.cloned())?;
        Ok(id)
    })
}

/// Deactivates a rate: it stays on past sales but can't be chosen any more (TAX-10).
pub fn deactivate(state: &AppState, id: &Id, by: Option<&Id>) -> Result<(), AppError> {
    let clock = state.clock.as_ref();
    state.db.write(|tx| {
        let rate = tax::get(tx, id)?;
        tax::deactivate(tx, id, clock.now())?;
        let details = serde_json::json!({ "name": rate.name, "change": "deactivate" });
        audit_log::record(tx, clock, TAX_RATE_CHANGE, details, by.cloned())
    })
}
