//! First-run setup progress: which wizard steps are done and which comes next
//! (rules SET-01, SET-02 and SET-10).

use pos_core::Timestamp;
use rusqlite::{Connection, Transaction, named_params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Result, SettingsError};

/// One step of the setup wizard, in the order the wizard shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SetupStep {
    Shop,
    Currency,
    Tax,
    Admin,
    Till,
    /// Can be skipped and finished later (rule SET-10).
    Hardware,
    Backup,
    Products,
}

impl SetupStep {
    /// Every step, in wizard order.
    pub const ALL: [SetupStep; 8] = [
        SetupStep::Shop,
        SetupStep::Currency,
        SetupStep::Tax,
        SetupStep::Admin,
        SetupStep::Till,
        SetupStep::Hardware,
        SetupStep::Backup,
        SetupStep::Products,
    ];

    /// The name stored in `settings_setup_steps.step`. Example: "shop".
    pub fn as_str(self) -> &'static str {
        match self {
            SetupStep::Shop => "shop",
            SetupStep::Currency => "currency",
            SetupStep::Tax => "tax",
            SetupStep::Admin => "admin",
            SetupStep::Till => "till",
            SetupStep::Hardware => "hardware",
            SetupStep::Backup => "backup",
            SetupStep::Products => "products",
        }
    }

    /// True for every step except hardware, which the shop may finish later.
    pub fn is_required(self) -> bool {
        self != SetupStep::Hardware
    }

    fn parse(text: &str) -> Option<SetupStep> {
        SetupStep::ALL
            .into_iter()
            .find(|step| step.as_str() == text)
    }
}

/// How far the shop has got with first-run setup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SetupStatus {
    /// Finished steps, in wizard order.
    pub completed: Vec<SetupStep>,
    /// The first step not finished yet, where the wizard opens. `None` when all are done.
    pub next_step: Option<SetupStep>,
    /// True when every required step is done, so the rest of the app can be used.
    /// The hardware step may still be open (it's then `next_step`).
    pub is_complete: bool,
}

/// Reads which setup steps are done, so the wizard can resume where it stopped (rule SET-02).
pub fn setup_status(conn: &Connection) -> Result<SetupStatus> {
    let mut statement = conn.prepare("SELECT step FROM settings_setup_steps")?;
    let names = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut completed = Vec::new();
    for name in names {
        let name = name?;
        let step = SetupStep::parse(&name).ok_or_else(|| SettingsError::Unreadable {
            key: "settings_setup_steps".to_string(),
            detail: format!("unknown step {name}"),
        })?;
        completed.push(step);
    }
    completed.sort();
    Ok(status_from(completed))
}

/// Marks a wizard step as done. Doing it again keeps the first time it was done.
pub fn complete_step(tx: &Transaction, step: SetupStep, now: Timestamp) -> Result<()> {
    tx.execute(
        "INSERT INTO settings_setup_steps (step, completed_at) VALUES (:step, :completed_at)
         ON CONFLICT (step) DO NOTHING",
        named_params! { ":step": step.as_str(), ":completed_at": now.0 },
    )?;
    Ok(())
}

/// Works out the next step and whether setup is complete from the finished steps.
fn status_from(completed: Vec<SetupStep>) -> SetupStatus {
    let next_step = SetupStep::ALL
        .into_iter()
        .find(|step| !completed.contains(step));
    let is_complete = SetupStep::ALL
        .into_iter()
        .filter(|step| step.is_required())
        .all(|step| completed.contains(&step));
    SetupStatus {
        completed,
        next_step,
        is_complete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_01_nothing_done_means_setup_is_not_complete() {
        let status = status_from(Vec::new());
        assert_eq!(status.next_step, Some(SetupStep::Shop));
        assert!(!status.is_complete);
    }

    #[test]
    fn set_02_step_names_match_the_table() {
        for step in SetupStep::ALL {
            assert_eq!(SetupStep::parse(step.as_str()), Some(step));
            let json = serde_json::to_string(&step).unwrap_or_default();
            assert_eq!(json, format!("\"{}\"", step.as_str()));
        }
        assert_eq!(SetupStep::parse("language"), None);
    }
}
